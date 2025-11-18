//! Static tool metadata for stdio server proxy.
//!
//! This module contains hardcoded metadata for all 109 tools across 14 categories.
//! Metadata is extracted from source files to avoid instantiating tool objects.

mod category_metadata;
mod routing;
mod types;

// Re-export public items (only what's used externally)
pub use category_metadata::all_tool_metadata;
pub use routing::{get_routing_table, CATEGORY_PORTS};
