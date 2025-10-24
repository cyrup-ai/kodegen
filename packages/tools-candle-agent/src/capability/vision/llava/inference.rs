//! Inference logic for LLaVA vision model

use candle_core::{Device, IndexOp, Tensor};
use candle_transformers::models::llama::Cache;
use candle_transformers::models::llava::{LLaVA, config::LLaVAConfig as CandleLLaVAConfig};
use tokenizers::Tokenizer;

use super::config::{GenerationConfig, ImageProcessingConfig};
use super::sampler::sample_token_static;
use super::tokenizer::tokenize_image_prompt_static;
use crate::builders::image::{ImageBuilder, ResizeFilter};
use crate::domain::image::Image;

/// References to LLaVA model components
pub(crate) struct LLaVAModelRefs<'a> {
    pub model: &'a LLaVA,
    pub tokenizer: &'a Tokenizer,
    pub llava_config: &'a CandleLLaVAConfig,
    pub device: &'a Device,
}

/// Configuration bundle for LLaVA processing
pub(crate) struct LLaVAConfigs {
    pub image_config: ImageProcessingConfig,
    pub gen_config: GenerationConfig,
}

/// Process ask request asynchronously on model thread
pub(crate) async fn process_ask(
    refs: LLaVAModelRefs<'_>,
    image_path: &str,
    question: &str,
    configs: LLaVAConfigs,
) -> Result<String, String> {
    let LLaVAModelRefs {
        model,
        tokenizer,
        llava_config,
        device,
    } = refs;
    let LLaVAConfigs {
        image_config,
        gen_config,
    } = configs;

    let ImageProcessingConfig {
        image_size,
        image_mean,
        image_std,
    } = image_config;
    let GenerationConfig {
        temperature,
        max_new_tokens,
        use_kv_cache,
    } = gen_config;
    // 1. Preprocess image - using to_tensor_sync since we're in a worker thread
    let image_tensor = Image::from_path(image_path)
        .resize(image_size, image_size, ResizeFilter::CatmullRom)
        .normalize_unsigned()
        .normalize_with(image_mean, image_std)
        .to_tensor_sync(device)
        .map_err(|e| format!("Image processing failed: {}", e))?;
    let image_size_tuple = (image_size as u32, image_size as u32);

    // 2. Format prompt with image token
    let prompt = format!("USER: <image>\n{}\nASSISTANT:", question);

    // 3. Tokenize prompt (handles <image> token)
    let input_ids = tokenize_image_prompt_static(
        tokenizer.clone(),
        llava_config,
        device.clone(),
        prompt.clone(),
    )
    .await?;

    // 4. Prepare multimodal embeddings (vision + text fusion)
    let image_batch = image_tensor
        .unsqueeze(0)
        .map_err(|e| format!("Image batch failed: {}", e))?;

    let input_embeds = model
        .prepare_inputs_labels_for_multimodal(&input_ids, &[image_batch], &[image_size_tuple])
        .map_err(|e| format!("Embedding preparation failed: {}", e))?;

    // 5. Create KV cache
    let llama_config = llava_config.to_llama_config();
    let mut cache = Cache::new(use_kv_cache, candle_core::DType::F16, &llama_config, device)
        .map_err(|e| format!("Cache creation failed: {}", e))?;

    // 6. Generate response (autoregressive loop)
    // NOTE: This loop remains synchronous because:
    // - LLaVA model is !Send (contains raw pointers, cannot move between threads)
    // - Model lives on LocalSet in dedicated worker thread
    // - model.forward() and model.llama.embed() require &mut model access
    // - Each non-model operation is fast (<1ms): tensor slicing, sampling, decoding
    // - spawn_blocking overhead (~100μs per call) would not improve performance
    // This is architecturally correct for Candle's !Send models
    let mut generated_text = String::new();
    let mut current_embeds = input_embeds;

    let mut index_pos = 0;
    for index in 0..max_new_tokens {
        // Get current embedding dimensions
        let (_, input_embeds_len, _) = current_embeds
            .dims3()
            .map_err(|e| format!("Failed to get embed dims: {}", e))?;

        // Determine context size and index based on cache state
        let (context_size, context_index) = if use_kv_cache && index > 0 {
            (1, index_pos) // Only new embedding after first iteration
        } else {
            (input_embeds_len, 0) // All embeddings on first iteration
        };

        // Slice embeddings to pass only relevant portion
        let input = current_embeds
            .i((.., input_embeds_len.saturating_sub(context_size).., ..))
            .map_err(|e| format!("Failed to slice embeddings: {}", e))?;

        // Forward pass with sliced input
        let logits = model
            .forward(&input, context_index, &mut cache)
            .map_err(|e| format!("Generation failed: {}", e))?;

        // Update position tracking
        let (_, input_len, _) = input
            .dims3()
            .map_err(|e| format!("Failed to get input dims: {}", e))?;
        index_pos += input_len;

        // Sample next token
        let next_token = sample_token_static(temperature, &logits)?;

        // Check EOS
        if next_token == llava_config.eos_token_id as u32 {
            break;
        }

        // Decode token
        if let Ok(text) = tokenizer.decode(&[next_token], false) {
            generated_text.push_str(&text);
        }

        // Embed next token and append
        let next_token_tensor = Tensor::new(&[next_token], device)
            .map_err(|e| format!("Token tensor failed: {}", e))?;
        let next_embeds = model
            .llama
            .embed(&next_token_tensor)
            .map_err(|e| format!("Embedding failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("Unsqueeze failed: {}", e))?;

        current_embeds = Tensor::cat(&[current_embeds, next_embeds], 1)
            .map_err(|e| format!("Embedding concat failed: {}", e))?;
    }

    Ok(generated_text)
}

/// Process ask_url request asynchronously on model thread
pub(crate) async fn process_ask_url(
    refs: LLaVAModelRefs<'_>,
    image_url: &str,
    question: &str,
    configs: LLaVAConfigs,
) -> Result<String, String> {
    let LLaVAModelRefs {
        model,
        tokenizer,
        llava_config,
        device,
    } = refs;
    let LLaVAConfigs {
        image_config,
        gen_config,
    } = configs;

    let ImageProcessingConfig {
        image_size,
        image_mean,
        image_std,
    } = image_config;
    let GenerationConfig {
        temperature,
        max_new_tokens,
        use_kv_cache,
    } = gen_config;
    // 1. Preprocess image from URL - using to_tensor_sync since we're in a worker thread
    let image_tensor = Image::from_url(image_url)
        .resize(image_size, image_size, ResizeFilter::CatmullRom)
        .normalize_unsigned()
        .normalize_with(image_mean, image_std)
        .to_tensor_sync(device)
        .map_err(|e| format!("Image processing failed: {}", e))?;
    let image_size_tuple = (image_size as u32, image_size as u32);

    // 2. Format prompt with image token
    let prompt = format!("USER: <image>\n{}\nASSISTANT:", question);

    // 3. Tokenize prompt (handles <image> token)
    let input_ids = tokenize_image_prompt_static(
        tokenizer.clone(),
        llava_config,
        device.clone(),
        prompt.clone(),
    )
    .await?;

    // 4. Prepare multimodal embeddings (vision + text fusion)
    let image_batch = image_tensor
        .unsqueeze(0)
        .map_err(|e| format!("Image batch failed: {}", e))?;

    let input_embeds = model
        .prepare_inputs_labels_for_multimodal(&input_ids, &[image_batch], &[image_size_tuple])
        .map_err(|e| format!("Embedding preparation failed: {}", e))?;

    // 5. Create KV cache
    let llama_config = llava_config.to_llama_config();
    let mut cache = Cache::new(use_kv_cache, candle_core::DType::F16, &llama_config, device)
        .map_err(|e| format!("Cache creation failed: {}", e))?;

    // 6. Generate response (autoregressive loop)
    // NOTE: This loop remains synchronous because:
    // - LLaVA model is !Send (contains raw pointers, cannot move between threads)
    // - Model lives on LocalSet in dedicated worker thread
    // - model.forward() and model.llama.embed() require &mut model access
    // - Each non-model operation is fast (<1ms): tensor slicing, sampling, decoding
    // - spawn_blocking overhead (~100μs per call) would not improve performance
    // This is architecturally correct for Candle's !Send models
    let mut generated_text = String::new();
    let mut current_embeds = input_embeds;

    let mut index_pos = 0;
    for index in 0..max_new_tokens {
        // Get current embedding dimensions
        let (_, input_embeds_len, _) = current_embeds
            .dims3()
            .map_err(|e| format!("Failed to get embed dims: {}", e))?;

        // Determine context size and index based on cache state
        let (context_size, context_index) = if use_kv_cache && index > 0 {
            (1, index_pos) // Only new embedding after first iteration
        } else {
            (input_embeds_len, 0) // All embeddings on first iteration
        };

        // Slice embeddings to pass only relevant portion
        let input = current_embeds
            .i((.., input_embeds_len.saturating_sub(context_size).., ..))
            .map_err(|e| format!("Failed to slice embeddings: {}", e))?;

        // Forward pass with sliced input
        let logits = model
            .forward(&input, context_index, &mut cache)
            .map_err(|e| format!("Generation failed: {}", e))?;

        // Update position tracking
        let (_, input_len, _) = input
            .dims3()
            .map_err(|e| format!("Failed to get input dims: {}", e))?;
        index_pos += input_len;

        // Sample next token
        let next_token = sample_token_static(temperature, &logits)?;

        // Check EOS
        if next_token == llava_config.eos_token_id as u32 {
            break;
        }

        // Decode token
        if let Ok(text) = tokenizer.decode(&[next_token], false) {
            generated_text.push_str(&text);
        }

        // Embed next token and append
        let next_token_tensor = Tensor::new(&[next_token], device)
            .map_err(|e| format!("Token tensor failed: {}", e))?;
        let next_embeds = model
            .llama
            .embed(&next_token_tensor)
            .map_err(|e| format!("Embedding failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("Unsqueeze failed: {}", e))?;

        current_embeds = Tensor::cat(&[current_embeds, next_embeds], 1)
            .map_err(|e| format!("Embedding concat failed: {}", e))?;
    }

    Ok(generated_text)
}
