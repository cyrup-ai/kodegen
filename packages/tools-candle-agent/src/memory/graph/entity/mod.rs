//! Entity model for graph-based memory
//!
//! This module provides entity abstractions for mapping domain objects
//! to graph nodes with support for validation, attributes, and persistence.
//! All operations are thread-safe and use async patterns for maximum performance.

mod futures;
mod repository;
mod surreal;
mod types;

// Re-export all public types to preserve API compatibility
pub use futures::{
    PendingEntity, PendingEntityCount, PendingEntityList, PendingEntityOption, PendingUnit,
};

pub use types::{BaseEntity, Entity, EntityValidatorFn};

pub use repository::EntityRepository;

pub use surreal::SurrealEntityRepository;
