//! Tool routing metadata for stdio server proxy.
//!
//! Tool metadata is now discovered automatically via inventory::iter<ToolMetadata>
//! from kodegen-mcp-schema. This module only contains infrastructure routing.

mod routing;

// Re-export routing infrastructure
pub use routing::{get_routing_table, CATEGORY_PORTS};
