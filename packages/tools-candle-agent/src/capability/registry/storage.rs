//! Registry storage - unified registries for all model types using parking_lot::RwLock

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use super::enums::*;
use crate::capability::text_embedding::StellaEmbeddingModel;
use crate::capability::text_to_text::CandleQwen3QuantizedModel;
use crate::capability::vision::LLaVAModel;
use crate::domain::model::traits::CandleModel;

//==============================================================================
// UNIFIED REGISTRIES
//==============================================================================
// All registries now use LazyLock<RwLock<HashMap<String, T>>> for unified
// storage that supports both static initialization and runtime registration.
// parking_lot::RwLock provides sync read/write access with excellent performance.

/// Unified text-to-text model registry
///
/// Initialized with Qwen3Quantized model and supports runtime registration
/// for models requiring async initialization.
pub(super) static TEXT_TO_TEXT_UNIFIED: LazyLock<RwLock<HashMap<String, TextToTextModel>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        let model = Arc::new(CandleQwen3QuantizedModel::default());
        let key = model.info().registry_key.to_string();
        map.insert(key, TextToTextModel::Qwen3Quantized(model));

        RwLock::new(map)
    });

/// Unified text embedding model registry
///
/// Initialized with Stella embedding model.
pub(super) static TEXT_EMBEDDING_UNIFIED: LazyLock<RwLock<HashMap<String, TextEmbeddingModel>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        let model = Arc::new(StellaEmbeddingModel::default());
        let key = model.info().registry_key.to_string();
        map.insert(key, TextEmbeddingModel::Stella(model));

        RwLock::new(map)
    });

/// Unified image embedding model registry
///
/// Starts empty and uses lazy loading on first registry::get() access.
/// Models are automatically created and registered when requested.
pub(super) static IMAGE_EMBEDDING_UNIFIED: LazyLock<RwLock<HashMap<String, ImageEmbeddingModel>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Unified text-to-image model registry
///
/// Starts empty and uses lazy loading on first registry::get() access.
/// Models are automatically created and registered when requested.
pub(super) static TEXT_TO_IMAGE_UNIFIED: LazyLock<RwLock<HashMap<String, TextToImageModel>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Unified vision model registry
///
/// Initialized with static vision models (LLaVA).
pub(crate) static VISION_UNIFIED: LazyLock<RwLock<HashMap<String, VisionModel>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        let model = Arc::new(LLaVAModel::default());
        let key = model.info().registry_key.to_string();
        map.insert(key, VisionModel::LLaVA(model));

        RwLock::new(map)
    });
