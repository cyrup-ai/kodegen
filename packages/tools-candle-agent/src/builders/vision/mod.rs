//! Vision builder - Fluent API for vision operations

mod traits;
mod vision_builder;

#[cfg(test)]
mod tests;

pub use traits::CandleVisionBuilder;
pub use vision_builder::VisionBuilderImpl;

pub(crate) use crate::capability::registry::VisionModel;
pub(crate) use crate::capability::traits::VisionCapable;
pub(crate) use crate::domain::completion::CandleStringChunk;
pub(crate) use std::pin::Pin;
pub(crate) use tokio_stream::Stream;
