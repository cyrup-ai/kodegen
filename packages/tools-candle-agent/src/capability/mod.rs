//! Capability-based Model Organization
//!
//! Models organized by what they CAN DO rather than who created them.
//! See GLOSSARY.md for architecture details.

pub mod registry;
pub mod traits;

pub mod image_embedding;
pub mod text_embedding;
pub mod text_to_image;
pub mod text_to_text;
pub mod vision;
