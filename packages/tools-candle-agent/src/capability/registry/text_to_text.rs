//! TextToTextCapable trait implementation for TextToTextModel

use super::pool::capabilities::text_to_text_pool;
use super::pool::core::{PoolError, ensure_workers_spawned_adaptive};
use crate::capability::traits::TextToTextCapable;
use crate::domain::completion::{CandleCompletionChunk, types::CandleCompletionParams};
use crate::domain::model::traits::CandleModel;
use crate::domain::prompt::CandlePrompt;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

// LoadedModel imports
use crate::capability::text_to_text::qwen3_quantized::LoadedQwen3QuantizedModel;

use super::enums::TextToTextModel;

impl TextToTextCapable for TextToTextModel {
    fn prompt(
        &self,
        prompt: CandlePrompt,
        params: &CandleCompletionParams,
    ) -> Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>> {
        match self {
            Self::Qwen3Quantized(m) => {
                spawn_stream_qwen3_quantized(m.clone(), prompt, params.clone())
            }
        }
    }
}

// Helper macro to eliminate duplication in streaming worker spawning
macro_rules! impl_text_to_text_spawn {
    ($fn_name:ident, $model_ty:ty, $loaded_ty:ty) => {
        fn $fn_name(
            model: Arc<$model_ty>,
            prompt: CandlePrompt,
            params: CandleCompletionParams,
        ) -> Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>> {
            let registry_key = model.info().registry_key;
            let per_worker_mb = model.info().est_memory_allocation_mb;
            let pool = text_to_text_pool();

            Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                if let Err(e) = ensure_workers_spawned_adaptive(
                    pool,
                    registry_key,
                    per_worker_mb,
                    pool.config().max_workers_per_model,
                    |_, allocation_guard| {
                        let m_clone = model.clone();
                        pool.spawn_text_to_text_worker(
                            registry_key,
                            move || async move {
                                <$loaded_ty>::load(&m_clone)
                                    .await
                                    .map_err(|e| PoolError::SpawnFailed(e.to_string()))
                            },
                            per_worker_mb,
                            allocation_guard,
                        )
                    },
                )
                .await
                {
                    let _ = tx.send(CandleCompletionChunk::Error(e.to_string()));
                    return;
                }

                let mut stream = pool.prompt(registry_key, prompt, params);
                use tokio_stream::StreamExt;
                while let Some(chunk) = stream.next().await {
                    if tx.send(chunk).is_err() {
                        break;
                    }
                }
            }))
        }
    };
}

// Generate functions for each model type
impl_text_to_text_spawn!(
    spawn_stream_qwen3_quantized,
    crate::capability::text_to_text::qwen3_quantized::CandleQwen3QuantizedModel,
    LoadedQwen3QuantizedModel
);
