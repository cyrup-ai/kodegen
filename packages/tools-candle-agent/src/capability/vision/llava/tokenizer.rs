//! Tokenization utilities for LLaVA

use candle_core::{Device, Tensor};
use candle_transformers::models::llava::config::LLaVAConfig as CandleLLaVAConfig;
use tokenizers::Tokenizer;

/// Tokenize prompt with image tokens (static version for thread)
///
/// Handles <image> placeholder insertion for multimodal embeddings
pub(crate) async fn tokenize_image_prompt_static(
    tokenizer: Tokenizer,
    llava_config: &CandleLLaVAConfig,
    device: Device,
    prompt: String,
) -> Result<Tensor, String> {
    let image_token_index = llava_config.image_token_index as i64;
    let bos_token_id = llava_config.bos_token_id as i64;

    // Wrap CPU-intensive tokenization in spawn_blocking
    let input_ids = tokio::task::spawn_blocking(move || {
        // Split by <image> and tokenize chunks (avoid unwrap in map)
        let mut chunks: Vec<Vec<i64>> = Vec::new();
        for s in prompt.split("<image>") {
            let encoding = tokenizer
                .encode(s, true)
                .map_err(|e| format!("Tokenization failed: {}", e))?;
            chunks.push(encoding.get_ids().iter().map(|x| *x as i64).collect());
        }

        // Interleave text tokens with image tokens
        let mut input_ids = Vec::new();
        let mut offset = 0;

        if !chunks.is_empty() && !chunks[0].is_empty() && chunks[0][0] == bos_token_id {
            offset = 1;
            input_ids.push(chunks[0][0]);
        }

        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                input_ids.push(image_token_index);
            }
            input_ids.extend(&chunk[offset..]);
            offset = 0;
        }

        Ok::<_, String>(input_ids)
    })
    .await
    .map_err(|e| format!("Spawn blocking failed: {}", e))??;

    // Create tensor (fast operation, keep outside spawn_blocking)
    let input_len = input_ids.len();
    Tensor::from_vec(input_ids, (1, input_len), &device)
        .map_err(|e| format!("Tokenization tensor failed: {}", e))
}
