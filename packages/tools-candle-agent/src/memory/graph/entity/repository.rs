//! Entity repository trait for CRUD operations
//!
//! Provides thread-safe interface for entity persistence operations.

use surrealdb::Value;

use super::futures::{
    PendingEntity, PendingEntityCount, PendingEntityList, PendingEntityOption, PendingUnit,
};
use super::types::Entity;

/// Thread-safe entity repository trait - Returns Futures
///
/// This trait provides an interface for entity CRUD operations that return futures.
/// All methods are thread-safe and return pending wrappers that can be awaited.
/// This pattern matches the rest of the memory system.
pub trait EntityRepository: Send + Sync {
    /// Create a new entity
    ///
    /// # Arguments
    /// * `entity` - Entity to create
    ///
    /// # Returns
    /// PendingEntity that resolves to the created entity with database-assigned ID
    fn create_entity(&self, entity: Box<dyn Entity>) -> PendingEntity;

    /// Get an entity by ID
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the entity
    ///
    /// # Returns
    /// PendingEntityOption that resolves to the entity if found
    fn get_entity(&self, id: &str) -> PendingEntityOption;

    /// Update an entity
    ///
    /// # Arguments
    /// * `entity` - Entity with updated data
    ///
    /// # Returns
    /// PendingEntity that resolves to the updated entity
    fn update_entity(&self, entity: Box<dyn Entity>) -> PendingEntity;

    /// Delete an entity
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the entity to delete
    ///
    /// # Returns
    /// PendingUnit that resolves when deletion is complete
    fn delete_entity(&self, id: &str) -> PendingUnit;

    /// Find entities by type with pagination
    ///
    /// # Arguments
    /// * `entity_type` - Type of entities to find
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// PendingEntityList that resolves to list of matching entities
    fn find_entities_by_type(
        &self,
        entity_type: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PendingEntityList;

    /// Find entities by attribute with pagination
    ///
    /// # Arguments
    /// * `attribute_name` - Name of the attribute to filter by
    /// * `attribute_value` - Value to match
    /// * `limit` - Maximum number of results
    /// * `offset` - Number of results to skip
    ///
    /// # Returns
    /// PendingEntityList that resolves to list of matching entities
    fn find_entities_by_attribute(
        &self,
        attribute_name: &str,
        attribute_value: &Value,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PendingEntityList;

    /// Count entities by type
    ///
    /// # Arguments
    /// * `entity_type` - Type of entities to count
    ///
    /// # Returns
    /// PendingEntityCount that resolves to the count
    fn count_entities_by_type(&self, entity_type: &str) -> PendingEntityCount;

    /// Batch create entities (optimized for bulk operations)
    ///
    /// # Arguments
    /// * `entities` - Vector of entities to create
    ///
    /// # Returns
    /// PendingEntityList that resolves to vector of created entities
    fn batch_create_entities(&self, entities: Vec<Box<dyn Entity>>) -> PendingEntityList;

    /// Batch update entities
    ///
    /// # Arguments
    /// * `entities` - Vector of entities to update
    ///
    /// # Returns
    /// PendingEntityList that resolves to vector of updated entities
    fn batch_update_entities(&self, entities: Vec<Box<dyn Entity>>) -> PendingEntityList;

    /// Batch delete entities
    ///
    /// # Arguments
    /// * `ids` - Vector of entity IDs to delete
    ///
    /// # Returns
    /// PendingUnit that resolves when deletion is complete
    fn batch_delete_entities(&self, ids: Vec<&str>) -> PendingUnit;
}
