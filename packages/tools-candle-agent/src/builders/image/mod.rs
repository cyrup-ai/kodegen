//! Image builder implementations - Zero Box<dyn> trait-based architecture
//!
//! All image construction logic and builder patterns with zero allocation.
//!
//! # Module Organization
//! - `api`: Public ImageBuilder trait and ResizeFilter enum
//! - `operations`: Internal operation types and converters
//! - `constructors`: Image::from_* entry points
//! - `builder_impl`: ImageBuilderImpl struct and trait implementation
//! - `processing`: Image processing pipeline methods

mod api;
mod builder_impl;
mod constructors;
mod operations;
mod processing;

// Re-export public API
pub use api::{ImageBuilder, ResizeFilter};

// Note: Image::from_* constructors are automatically available via trait implementation
// Note: ImageBuilderImpl is private, no re-export needed
