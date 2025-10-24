use super::super::types::{MemoryContent, MemoryTypeEnum};
use super::MemoryNode;

impl PartialEq for MemoryNode {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.base_memory.id == other.base_memory.id
    }
}

impl Eq for MemoryNode {}

impl std::hash::Hash for MemoryNode {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base_memory.id.hash(state);
    }
}

impl Default for MemoryNode {
    fn default() -> Self {
        MemoryNode::new(
            MemoryTypeEnum::Semantic,
            MemoryContent::text("Default memory node"),
        )
    }
}

impl cyrup_sugars::prelude::MessageChunk for MemoryNode {
    fn bad_chunk(error: String) -> Self {
        MemoryNode::new(
            MemoryTypeEnum::Semantic,
            MemoryContent::text(format!("Error: {error}")),
        )
    }

    fn error(&self) -> Option<&str> {
        // Check if this memory node represents an error state
        match &self.base_memory.content {
            MemoryContent::Text(text) => {
                if text.starts_with("Error: ") {
                    Some("Memory node error")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
