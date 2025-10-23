//! Resource inlining functionality
//!
//! This module provides functionality for inlining external resources (CSS, images, SVGs)
//! into HTML content, converting them to embedded content for self-contained documents.

// Sub-modules
pub mod core;
pub mod downloaders;
pub mod processors;
pub mod utils;

// Re-exports for public API
pub use core::{
    InliningError, InliningResult, ResourceType, inline_all_resources, inline_resources_from_info,
};
pub use downloaders::InlineConfig;
pub use processors::{process_css_links, process_images, process_svgs};
pub use utils::resolve_url;
