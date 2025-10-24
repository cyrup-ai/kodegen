//   // Removed unused import

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// Tag with hierarchical structure (Candle-prefixed for domain system)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandleConversationTag {
    /// Unique identifier
    pub id: String,
    /// Tag name
    pub name: String,
    /// Optional description
    pub description: String,
    /// Category for grouping
    pub category: String,
    /// Parent tag ID, if any
    pub parent_id: Option<String>,
    /// Child tags
    #[serde(skip_serializing, default)]
    pub children: Vec<String>,
    /// Number of times this tag has been used
    pub usage_count: usize,
    /// When the tag was created (unix timestamp)
    pub created_at: u64,
    /// When the tag was last used (unix timestamp)
    pub last_used: u64,
}

/// Tagging statistics (Candle-prefixed for domain system)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandleTaggingStatistics {
    /// Total number of tags
    pub total_tags: usize,
    /// Number of tagged messages
    pub tagged_messages: usize,
    /// Most used tag
    pub most_used_tag: Option<String>,
    /// Tags by category
    pub tags_by_category: HashMap<String, usize>,
    /// Tags by depth in hierarchy
    pub tags_by_depth: HashMap<usize, usize>,
    /// Average tags per message
    pub avg_tags_per_message: f64,
}
