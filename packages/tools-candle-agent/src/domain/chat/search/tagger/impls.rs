use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crossbeam_skiplist::SkipMap;
use hashbrown::HashMap;
use regex::Regex;
use std::pin::Pin;
use tokio_stream::Stream;
use uuid::Uuid;

use super::types::{CandleConversationTag, CandleTaggingStatistics};

/// Conversation tagger with lock-free operations (Candle-prefixed for domain system)
#[derive(Debug)]
pub struct CandleConversationTagger {
    /// Map of tag ID to tag
    pub tags: SkipMap<String, CandleConversationTag>,
    /// Map of message ID to set of tag IDs
    message_tags: SkipMap<String, HashSet<String>>,
    /// Map of tag ID to set of message IDs
    tag_messages: SkipMap<String, HashSet<String>>,
    /// Auto-tagging rules (regex pattern to tag IDs)
    auto_tag_rules: Vec<(Regex, Vec<String>)>,
    /// Tag usage statistics
    stats: CandleTaggingStatistics,
    /// Total number of tagged messages
    total_tagged_messages: AtomicUsize,
}

impl Default for CandleConversationTagger {
    fn default() -> Self {
        Self::new()
    }
}

impl CandleConversationTagger {
    /// Create a new conversation tagger
    #[must_use]
    pub fn new() -> Self {
        let mut tagger = Self {
            tags: SkipMap::new(),
            message_tags: SkipMap::new(),
            tag_messages: SkipMap::new(),
            auto_tag_rules: Vec::new(),
            stats: CandleTaggingStatistics::default(),
            total_tagged_messages: AtomicUsize::new(0),
        };

        tagger.initialize_default_rules();
        tagger
    }

    /// Create a new tag (streaming)
    pub fn create_tag_stream(
        &mut self,
        name: String,
        description: String,
        category: String,
    ) -> Pin<
        Box<
            dyn Stream<Item = crate::domain::context::chunks::CandleCollectionChunk<String>> + Send,
        >,
    > {
        let id: String = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tag = CandleConversationTag {
            id: id.clone(),
            name,
            description,
            category,
            parent_id: None,
            children: Vec::new(),
            usage_count: 0,
            created_at: now,
            last_used: now,
        };

        self.tags.insert(id.clone(), tag);
        self.stats.total_tags += 1;

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let result = crate::domain::context::chunks::CandleCollectionChunk {
                items: id,
                error_message: None,
            };
            let _ = tx.send(result);
        }))
    }

    /// Add a tag to a message (bidirectional mapping)
    pub fn add_tag(&self, message_id: &str, tag_id: &str) {
        // Update message → tags mapping
        let mut msg_tags = self
            .message_tags
            .get(message_id)
            .map(|e| e.value().clone())
            .unwrap_or_default();

        let is_first_tag = msg_tags.is_empty();
        msg_tags.insert(tag_id.to_string());
        self.message_tags.insert(message_id.to_string(), msg_tags);

        // Update tag → messages mapping (CRITICAL for bidirectionality)
        let mut tag_msgs = self
            .tag_messages
            .get(tag_id)
            .map(|e| e.value().clone())
            .unwrap_or_default();
        tag_msgs.insert(message_id.to_string());
        self.tag_messages.insert(tag_id.to_string(), tag_msgs);

        // Update tag usage count
        if let Some(tag_entry) = self.tags.get(tag_id) {
            let mut tag = tag_entry.value().clone();
            tag.usage_count += 1;
            tag.last_used = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.tags.insert(tag_id.to_string(), tag);
        }

        // Increment total tagged messages if this is the first tag for this message
        if is_first_tag {
            self.total_tagged_messages.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Remove a tag from a message (cleanup both directions)
    pub fn remove_tag(&self, message_id: &str, tag_id: &str) {
        // Remove from message → tags mapping
        if let Some(msg_tags_entry) = self.message_tags.get(message_id) {
            let mut msg_tags = msg_tags_entry.value().clone();
            msg_tags.remove(tag_id);

            if msg_tags.is_empty() {
                self.message_tags.remove(message_id);
                self.total_tagged_messages.fetch_sub(1, Ordering::Relaxed);
            } else {
                self.message_tags.insert(message_id.to_string(), msg_tags);
            }
        }

        // Remove from tag → messages mapping
        if let Some(tag_msgs_entry) = self.tag_messages.get(tag_id) {
            let mut tag_msgs = tag_msgs_entry.value().clone();
            tag_msgs.remove(message_id);

            if tag_msgs.is_empty() {
                self.tag_messages.remove(tag_id);
            } else {
                self.tag_messages.insert(tag_id.to_string(), tag_msgs);
            }
        }

        // Decrement tag usage count
        if let Some(tag_entry) = self.tags.get(tag_id) {
            let mut tag = tag_entry.value().clone();
            tag.usage_count = tag.usage_count.saturating_sub(1);
            self.tags.insert(tag_id.to_string(), tag);
        }
    }

    /// Get all tags for a message
    pub fn get_tags(&self, message_id: &str) -> HashSet<String> {
        self.message_tags
            .get(message_id)
            .map(|e| e.value().clone())
            .unwrap_or_default()
    }

    /// Get all messages with a specific tag
    pub fn get_messages_by_tag(&self, tag_id: &str) -> HashSet<String> {
        self.tag_messages
            .get(tag_id)
            .map(|e| e.value().clone())
            .unwrap_or_default()
    }

    /// Apply auto-tag rules to message content
    pub fn auto_tag_message(&self, message_id: &str, content: &str) {
        for (pattern, tag_ids) in &self.auto_tag_rules {
            if pattern.is_match(content) {
                for tag_id in tag_ids {
                    self.add_tag(message_id, tag_id);
                }
            }
        }
    }

    /// Add a new auto-tag rule (mutable because `auto_tag_rules` is Vec)
    pub fn add_auto_tag_rule(&mut self, pattern: Regex, tag_ids: Vec<String>) {
        self.auto_tag_rules.push((pattern, tag_ids));
    }

    /// Initialize with default auto-tag rules
    fn initialize_default_rules(&mut self) {
        // Error/exception detection
        if let Ok(pattern) =
            Regex::new(r"(?i)(error|exception|failed|failure|panic|crash|fatal|abort)")
        {
            let error_tag = self.create_default_tag("error", "Error messages", "system");
            self.auto_tag_rules.push((pattern, vec![error_tag]));
        }

        // Code detection (multiple patterns)
        if let Ok(pattern) = Regex::new(
            r"(```[\s\S]*?```|`[^`]+`|fn\s+\w+|function\s+\w+|def\s+\w+|class\s+\w+|struct\s+\w+|impl\s+\w+)",
        ) {
            let code_tag = self.create_default_tag("code", "Contains code", "content");
            self.auto_tag_rules.push((pattern, vec![code_tag]));
        }

        // Question detection
        if let Ok(pattern) = Regex::new(r"\?(?:\s|$)") {
            let question_tag = self.create_default_tag("question", "Question", "intent");
            self.auto_tag_rules.push((pattern, vec![question_tag]));
        }

        // Command detection
        if let Ok(pattern) =
            Regex::new(r"(?:^\$|^>|^#|\bcargo\s+\w+|\bnpm\s+\w+|\bgit\s+\w+|\bpython\s+|\.\/\w+)")
        {
            let command_tag = self.create_default_tag("command", "Shell command", "content");
            self.auto_tag_rules.push((pattern, vec![command_tag]));
        }

        // URL detection
        if let Ok(pattern) = Regex::new(r"https?://[^\s]+") {
            let url_tag = self.create_default_tag("url", "Contains URL", "content");
            self.auto_tag_rules.push((pattern, vec![url_tag]));
        }

        // File path detection (Unix and Windows)
        if let Ok(pattern) = Regex::new(r"(?:/[a-zA-Z0-9_.-]+)+|[A-Z]:\\[^\s]+|~/[^\s]+") {
            let path_tag = self.create_default_tag("file-path", "File path", "content");
            self.auto_tag_rules.push((pattern, vec![path_tag]));
        }

        // JSON detection
        if let Ok(pattern) = Regex::new(r#"^\s*[\[{].*[\]}]\s*$|"[^"]+"\s*:\s*"#) {
            let json_tag = self.create_default_tag("json", "JSON data", "format");
            self.auto_tag_rules.push((pattern, vec![json_tag]));
        }

        // SQL detection
        if let Ok(pattern) = Regex::new(r"(?i)\b(SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP)\s+")
        {
            let sql_tag = self.create_default_tag("sql", "SQL query", "format");
            self.auto_tag_rules.push((pattern, vec![sql_tag]));
        }

        // YAML/TOML detection
        if let Ok(pattern) = Regex::new(r"^\s*[\w-]+\s*:\s*[^\n]+|^\s*\[[\w.-]+\]") {
            let config_tag = self.create_default_tag("config", "Configuration", "format");
            self.auto_tag_rules.push((pattern, vec![config_tag]));
        }

        // Number/metric detection (percentages, large numbers)
        if let Ok(pattern) = Regex::new(r"\b\d+%|\b\d{4,}|\b\d+\.\d+\s*(ms|GB|MB|KB|sec|min)\b") {
            let metric_tag = self.create_default_tag("metric", "Contains metrics", "content");
            self.auto_tag_rules.push((pattern, vec![metric_tag]));
        }

        // Timestamp detection
        if let Ok(pattern) = Regex::new(r"\d{4}-\d{2}-\d{2}|\d{2}:\d{2}:\d{2}") {
            let time_tag = self.create_default_tag("timestamp", "Has timestamp", "content");
            self.auto_tag_rules.push((pattern, vec![time_tag]));
        }

        // Email detection
        if let Ok(pattern) = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b") {
            let email_tag = self.create_default_tag("email", "Contains email", "content");
            self.auto_tag_rules.push((pattern, vec![email_tag]));
        }
    }

    /// Helper to create default tags
    fn create_default_tag(&mut self, name: &str, description: &str, category: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tag = CandleConversationTag {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            category: category.to_string(),
            parent_id: None,
            children: Vec::new(),
            usage_count: 0,
            created_at: now,
            last_used: now,
        };

        self.tags.insert(id.clone(), tag);
        self.stats.total_tags += 1;

        id
    }

    /// Compute tag statistics
    pub fn tag_statistics(&self) -> CandleTaggingStatistics {
        let total_tags = self.tags.len();
        let tagged_messages = self.total_tagged_messages.load(Ordering::Relaxed);

        // Find most used tag
        let mut most_used_tag = None;
        let mut max_count = 0;
        for entry in &self.tags {
            let tag = entry.value();
            if tag.usage_count > max_count {
                max_count = tag.usage_count;
                most_used_tag = Some(tag.name.clone());
            }
        }

        // Tags by category
        let mut tags_by_category = HashMap::new();
        for entry in &self.tags {
            let tag = entry.value();
            *tags_by_category.entry(tag.category.clone()).or_insert(0) += 1;
        }

        // Tags by depth (currently flat hierarchy, depth=0)
        let mut tags_by_depth = HashMap::new();
        tags_by_depth.insert(0, total_tags);

        // Average tags per message
        let avg_tags_per_message = if tagged_messages > 0 {
            let total_tag_assignments: usize =
                self.message_tags.iter().map(|e| e.value().len()).sum();
            #[allow(clippy::cast_precision_loss)]
            let avg = total_tag_assignments as f64 / tagged_messages as f64;
            avg
        } else {
            0.0
        };

        CandleTaggingStatistics {
            total_tags,
            tagged_messages,
            most_used_tag,
            tags_by_category,
            tags_by_depth,
            avg_tags_per_message,
        }
    }

    /// Get all tags (for listing/export)
    pub fn get_all_tags(&self) -> Vec<(String, usize)> {
        self.tags
            .iter()
            .map(|e| {
                let tag = e.value();
                (tag.name.clone(), tag.usage_count)
            })
            .collect()
    }
}
