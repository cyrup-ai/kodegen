//! Shared utilities for Stella embedding model

use anyhow::{Context, Result, anyhow};
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::models::stella_en_v5::{Config, EmbedDim, ModelVariant};
use std::path::PathBuf;
use tokenizers::{PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use super::super::safetensors_validation::validate_safetensors_file;

/// Configure tokenizer with variant-specific padding and truncation
///
/// # Padding
/// - Large (1.5B): Left padding with <|endoftext|> token
/// - Small (400M): Right padding with default token
pub(crate) fn configure_stella_tokenizer(
    tokenizer: &mut Tokenizer,
    variant: ModelVariant,
    max_length: usize,
) -> Result<()> {
    // Variant-specific padding
    match variant {
        ModelVariant::Large => {
            let pad_id = tokenizer
                .token_to_id("<|endoftext|>")
                .ok_or_else(|| anyhow!("Tokenizer missing <|endoftext|> token"))?;
            tokenizer.with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::BatchLongest,
                direction: PaddingDirection::Left,
                pad_to_multiple_of: None,
                pad_id,
                pad_type_id: 0,
                pad_token: "<|endoftext|>".to_string(),
            }));
        }
        ModelVariant::Small => {
            tokenizer.with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::BatchLongest,
                direction: PaddingDirection::Right,
                ..Default::default()
            }));
        }
    }

    // Set truncation if not already set
    if tokenizer.get_truncation().is_none() {
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length,
                strategy: tokenizers::TruncationStrategy::LongestFirst,
                stride: 0,
                direction: tokenizers::TruncationDirection::Right,
            }))
            .map_err(|e| anyhow!("Failed to set truncation: {}", e))?;
    }

    Ok(())
}

/// Create Stella config based on variant and embedding dimension
pub(crate) fn create_stella_config(variant: ModelVariant, embed_dim: EmbedDim) -> Config {
    match variant {
        ModelVariant::Large => Config::new_1_5_b_v5(embed_dim),
        ModelVariant::Small => Config::new_400_m_v5(embed_dim),
    }
}

/// Load Stella model weights (base + projection head)
///
/// Validates SafeTensors files before unsafe mmap to prevent crashes.
pub(crate) fn load_stella_weights(
    base_weights: PathBuf,
    projection_head: PathBuf,
    dtype: DType,
    device: &Device,
) -> anyhow::Result<(VarBuilder<'static>, VarBuilder<'static>)> {
    // Validate files before unsafe mmap
    validate_safetensors_file(&base_weights)?;
    validate_safetensors_file(&projection_head)?;

    let base_vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[base_weights], dtype, device)
            .context("Failed to load base model weights")?
    };

    let embed_vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[projection_head], DType::F32, device)
            .context("Failed to load projection head weights")?
    };

    Ok((base_vb, embed_vb))
}
