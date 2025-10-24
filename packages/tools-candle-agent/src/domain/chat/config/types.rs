//! Chat configuration types including personality, behavior, and UI settings

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Duration;

/// Duration serialization helper
pub(super) mod duration_secs {
    use super::{Deserialize, Deserializer, Duration, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Core Candle chat configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CandleChatConfig {
    /// Maximum message length
    pub max_message_length: usize,
    /// Enable message history
    pub enable_history: bool,
    /// History retention period in seconds (for rkyv compatibility)
    #[serde(with = "duration_secs")]
    pub history_retention: Duration,
    /// Enable streaming responses
    pub enable_streaming: bool,
    /// Candle personality configuration
    pub personality: CandlePersonalityConfig,
    /// Candle behavior configuration
    pub behavior: CandleBehaviorConfig,
    /// Candle UI configuration
    pub ui: CandleUIConfig,
}

/// Candle personality configuration for AI behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlePersonalityConfig {
    /// Personality type identifier
    pub personality_type: String,
    /// Response style settings
    pub response_style: String,
    /// Tone configuration
    pub tone: String,
    /// Custom instructions
    pub custom_instructions: Option<String>,
    /// Creativity level (0.0-1.0)
    pub creativity: f64,
    /// Formality level (0.0-1.0)
    pub formality: f64,
    /// Humor level (0.0-1.0)
    pub humor: f64,
    /// Empathy level (0.0-1.0)
    pub empathy: f64,
    /// Expertise level
    pub expertise_level: String,
    /// Verbosity level
    pub verbosity: String,
    /// Personality traits
    pub traits: Vec<String>,
}

/// Candle behavior configuration for chat system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleBehaviorConfig {
    /// Enable auto-responses
    pub auto_response: bool,
    /// Response delay settings
    #[serde(with = "duration_secs")]
    pub response_delay: Duration,
    /// Enable message filtering
    pub enable_filtering: bool,
    /// Maximum concurrent conversations
    pub max_concurrent_chats: usize,
    /// Proactivity level (0.0-1.0)
    pub proactivity: f64,
    /// Question frequency (0.0-1.0)
    pub question_frequency: f64,
    /// Conversation flow style
    pub conversation_flow: String,
    /// Follow-up behavior style
    pub follow_up_behavior: String,
    /// Error handling approach
    pub error_handling: String,
}

/// Candle user interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleUIConfig {
    /// Theme settings
    pub theme: String,
    /// Font size
    pub font_size: u32,
    /// Enable dark mode
    pub dark_mode: bool,
    /// Animation settings
    pub enable_animations: bool,
    /// Layout style
    pub layout: String,
    /// Color scheme
    pub color_scheme: String,
    /// Display density
    pub display_density: String,
    /// Animation settings
    pub animations: String,
}

impl Default for CandlePersonalityConfig {
    fn default() -> Self {
        Self {
            personality_type: "balanced".to_string(),
            response_style: "helpful".to_string(),
            tone: "neutral".to_string(),
            custom_instructions: None,
            creativity: 0.5,
            formality: 0.5,
            humor: 0.3,
            empathy: 0.7,
            expertise_level: "intermediate".to_string(),
            verbosity: "balanced".to_string(),
            traits: Vec::new(),
        }
    }
}

impl Default for CandleBehaviorConfig {
    fn default() -> Self {
        Self {
            auto_response: false,
            response_delay: Duration::from_millis(500),
            enable_filtering: true,
            max_concurrent_chats: 10,
            proactivity: 0.5,
            question_frequency: 0.3,
            conversation_flow: String::from("natural"),
            follow_up_behavior: String::from("contextual"),
            error_handling: String::from("graceful"),
        }
    }
}

impl Default for CandleUIConfig {
    fn default() -> Self {
        Self {
            theme: String::from("default"),
            font_size: 14,
            dark_mode: false,
            enable_animations: true,
            layout: String::from("standard"),
            color_scheme: String::from("adaptive"),
            display_density: String::from("comfortable"),
            animations: String::from("smooth"),
        }
    }
}
