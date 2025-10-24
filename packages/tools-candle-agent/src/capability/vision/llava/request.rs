//! Request types for LLaVA model thread communication

use super::config::VisionConfig;
use tokio::sync::mpsc;

/// Request types for LLaVA model thread communication
pub(crate) enum LLaVARequest {
    Ask {
        image_path: String,
        question: String,
        config: Option<VisionConfig>,
        response_tx: mpsc::UnboundedSender<Result<String, String>>,
    },
    AskUrl {
        image_url: String,
        question: String,
        config: Option<VisionConfig>,
        response_tx: mpsc::UnboundedSender<Result<String, String>>,
    },
    #[allow(dead_code)] // Reserved for graceful shutdown implementation
    Shutdown,
}
