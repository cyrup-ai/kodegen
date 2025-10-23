//! Content saving utilities for web scraping

// Module declarations
pub mod cache_check;
mod compression;
mod html_saver;
mod indexing;
mod json_saver;
pub mod markdown_converter;
mod markdown_saver;

// Re-export public API from cache_check module
pub use cache_check::{
    check_etag_from_events, extract_etag_from_headers, get_mirror_path_sync, read_cached_etag,
};

// Re-export public API from compression module
pub use compression::{CacheMetadata, save_compressed_file};

// Re-export public API from html_saver module
pub use html_saver::{save_html_content, save_html_content_with_resources};

// Re-export public API from indexing module
pub use indexing::optimize_search_index;

// Re-export public API from json_saver module
pub use json_saver::{save_json_data, save_page_data};

// Re-export public API from markdown_saver module
pub use markdown_saver::save_markdown_content;
