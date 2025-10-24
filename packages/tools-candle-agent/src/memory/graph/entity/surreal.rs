//! SurrealDB-backed entity repository implementation

use std::marker::PhantomData;
use std::sync::Arc;

use surrealdb::Value;
use tokio::sync::oneshot;

use super::futures::{
    PendingEntity, PendingEntityCount, PendingEntityList, PendingEntityOption, PendingUnit,
};
use super::repository::EntityRepository;
use super::types::{Entity, EntityValidatorFn};
use crate::memory::graph::graph_db::{GraphDatabase, GraphError, GraphQueryOptions};

/// SurrealDB-backed entity repository implementation
pub struct SurrealEntityRepository<E: Entity + Clone + 'static> {
    db: Arc<dyn GraphDatabase>,
    table_name: String,
    validator: Option<EntityValidatorFn>,
    _phantom: PhantomData<E>,
}

impl<E: Entity + Clone + 'static> SurrealEntityRepository<E> {
    /// Create a new SurrealDB entity repository
    ///
    /// # Arguments
    /// * `db` - Graph database connection
    /// * `table_name` - Name of the table/collection for this entity type
    ///
    /// # Returns
    /// New repository instance
    pub fn new(db: Arc<dyn GraphDatabase>, table_name: String) -> Self {
        Self {
            db,
            table_name,
            validator: None,
            _phantom: PhantomData,
        }
    }

    /// Create with custom validation
    ///
    /// # Arguments
    /// * `db` - Graph database connection
    /// * `table_name` - Name of the table/collection
    /// * `validator` - Custom validation function
    ///
    /// # Returns
    /// New repository instance with validation
    pub fn with_validator(
        db: Arc<dyn GraphDatabase>,
        table_name: String,
        validator: EntityValidatorFn,
    ) -> Self {
        Self {
            db,
            table_name,
            validator: Some(validator),
            _phantom: PhantomData,
        }
    }

    /// Set or update the validator function
    pub fn set_validator(&mut self, validator: EntityValidatorFn) {
        self.validator = Some(validator);
    }

    /// Remove the validator function
    pub fn remove_validator(&mut self) {
        self.validator = None;
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl<E: Entity + Clone + 'static> EntityRepository for SurrealEntityRepository<E> {
    fn create_entity(&self, entity: Box<dyn Entity>) -> PendingEntity {
        // Validate synchronously
        if let Some(validator) = &self.validator
            && let Err(e) = validator(entity.as_ref())
        {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Err(e));
            return PendingEntity::new(rx);
        }

        if let Err(e) = entity.validate() {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Err(e));
            return PendingEntity::new(rx);
        }

        let node = entity.to_node();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .create_node(node.properties)
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .map(|_| entity);
            let _ = tx.send(result);
        });

        PendingEntity::new(rx)
    }

    fn get_entity(&self, id: &str) -> PendingEntityOption {
        let id = id.to_string();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .get_node(&id)
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .map(|opt| {
                    opt.and_then(|node| {
                        E::from_node(node)
                            .ok()
                            .map(|entity| Box::new(entity) as Box<dyn Entity>)
                    })
                });
            let _ = tx.send(result);
        });

        PendingEntityOption::new(rx)
    }

    fn update_entity(&self, entity: Box<dyn Entity>) -> PendingEntity {
        // Validate synchronously
        if let Some(validator) = &self.validator
            && let Err(e) = validator(entity.as_ref())
        {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Err(e));
            return PendingEntity::new(rx);
        }

        if let Err(e) = entity.validate() {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Err(e));
            return PendingEntity::new(rx);
        }

        let node = entity.to_node();
        let entity_id = entity.id().to_string();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .update_node(&entity_id, node.properties)
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .map(|_| entity);
            let _ = tx.send(result);
        });

        PendingEntity::new(rx)
    }

    fn delete_entity(&self, id: &str) -> PendingUnit {
        let id = id.to_string();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .delete_node(&id)
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()));
            let _ = tx.send(result);
        });

        PendingUnit::new(rx)
    }

    fn find_entities_by_type(
        &self,
        entity_type: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PendingEntityList {
        let entity_type = entity_type.to_string();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            use futures_util::StreamExt;

            let node_stream = db.get_nodes_by_type(&entity_type);
            let mut entities = Vec::new();
            let mut count = 0;
            let start_offset = offset.unwrap_or(0);
            let max_limit = limit.unwrap_or(usize::MAX);

            let mut stream = node_stream;
            while let Some(node_result) = stream.next().await {
                match node_result {
                    Ok(node) => {
                        if count < start_offset {
                            count += 1;
                            continue;
                        }

                        if entities.len() >= max_limit {
                            break;
                        }

                        if let Ok(entity) = E::from_node(node) {
                            entities.push(Box::new(entity) as Box<dyn Entity>);
                        }
                        count += 1;
                    }
                    Err(_) => break,
                }
            }

            let _ = tx.send(Ok(entities));
        });

        PendingEntityList::new(rx)
    }

    fn find_entities_by_attribute(
        &self,
        attribute_name: &str,
        attribute_value: &Value,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PendingEntityList {
        let attribute_name = attribute_name.to_string();
        let attribute_value = attribute_value.clone();
        let table_name = self.table_name.clone();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        // Convert attribute_value to JSON value
        let json_value = match serde_json::to_value(&attribute_value) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(Err(GraphError::ConversionError(format!(
                    "Failed to convert attribute value: {}",
                    e
                ))));
                return PendingEntityList::new(rx);
            }
        };

        tokio::spawn(async move {
            use futures_util::StreamExt;

            let query = format!(
                "SELECT * FROM {} WHERE {} = $value",
                table_name, attribute_name
            );
            let options = GraphQueryOptions {
                limit,
                offset,
                filters: {
                    let mut filters = std::collections::HashMap::new();
                    filters.insert("value".to_string(), json_value);
                    filters
                },
            };

            let node_stream = db.query(&query, Some(options));
            let mut entities = Vec::new();
            let mut stream = node_stream;

            while let Some(node_result) = stream.next().await {
                if let Ok(node) = node_result
                    && let Ok(entity) = E::from_node(node)
                {
                    entities.push(Box::new(entity) as Box<dyn Entity>);
                }
            }

            let _ = tx.send(Ok(entities));
        });

        PendingEntityList::new(rx)
    }

    fn count_entities_by_type(&self, entity_type: &str) -> PendingEntityCount {
        let entity_type = entity_type.to_string();
        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            use futures_util::StreamExt;

            let query = format!("SELECT COUNT(*) FROM {}", entity_type);
            let node_stream = db.query(&query, Some(GraphQueryOptions::default()));
            let mut stream = node_stream;
            let mut count = 0;

            if let Some(Ok(node)) = stream.next().await
                && let Some(count_value) = node.properties.get("count")
            {
                count = count_value.as_u64().unwrap_or(0) as usize;
            }

            let _ = tx.send(Ok(count));
        });

        PendingEntityCount::new(rx)
    }

    fn batch_create_entities(&self, entities: Vec<Box<dyn Entity>>) -> PendingEntityList {
        // Validate all entities first
        for entity in &entities {
            if let Some(validator) = &self.validator
                && let Err(e) = validator(entity.as_ref())
            {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Err(e));
                return PendingEntityList::new(rx);
            }
            if let Err(e) = entity.validate() {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Err(e));
                return PendingEntityList::new(rx);
            }
        }

        // Convert entities to JSON array for SurrealQL
        let create_items: Vec<serde_json::Value> = entities
            .iter()
            .map(|entity| {
                let node = entity.to_node();
                serde_json::json!({
                    "properties": node.properties
                })
            })
            .collect();

        let query = format!(
            "BEGIN TRANSACTION; FOR $item IN $items {{ CREATE {}:ulid() CONTENT $item.properties; }}; COMMIT TRANSACTION;",
            self.table_name
        );

        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .batch_query(&query, serde_json::json!({ "items": create_items }))
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .and_then(|nodes| {
                    let mut results = Vec::with_capacity(nodes.len());
                    for node in nodes {
                        match E::from_node(node) {
                            Ok(entity) => results.push(Box::new(entity) as Box<dyn Entity>),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(results)
                });
            let _ = tx.send(result);
        });

        PendingEntityList::new(rx)
    }

    fn batch_update_entities(&self, entities: Vec<Box<dyn Entity>>) -> PendingEntityList {
        // Validate all entities first
        for entity in &entities {
            if let Some(validator) = &self.validator
                && let Err(e) = validator(entity.as_ref())
            {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Err(e));
                return PendingEntityList::new(rx);
            }
            if let Err(e) = entity.validate() {
                let (tx, rx) = oneshot::channel();
                let _ = tx.send(Err(e));
                return PendingEntityList::new(rx);
            }
        }

        // Convert entities to JSON array with IDs for SurrealQL
        let update_items: Vec<serde_json::Value> = entities
            .iter()
            .map(|entity| {
                let node = entity.to_node();
                serde_json::json!({
                    "id": entity.id(),
                    "properties": node.properties
                })
            })
            .collect();

        let query = "BEGIN TRANSACTION; FOR $item IN $items { UPDATE $item.id CONTENT $item.properties; }; COMMIT TRANSACTION;".to_string();

        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .batch_query(&query, serde_json::json!({ "items": update_items }))
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .and_then(|nodes| {
                    let mut results = Vec::with_capacity(nodes.len());
                    for node in nodes {
                        match E::from_node(node) {
                            Ok(entity) => results.push(Box::new(entity) as Box<dyn Entity>),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(results)
                });
            let _ = tx.send(result);
        });

        PendingEntityList::new(rx)
    }

    fn batch_delete_entities(&self, ids: Vec<&str>) -> PendingUnit {
        let id_array: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let query =
            "BEGIN TRANSACTION; FOR $id IN $ids { DELETE $id; }; COMMIT TRANSACTION;".to_string();

        let db = self.db.clone();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = db
                .batch_query(&query, serde_json::json!({ "ids": id_array }))
                .await
                .map_err(|e| GraphError::DatabaseError(e.to_string()))
                .map(|_| ());
            let _ = tx.send(result);
        });

        PendingUnit::new(rx)
    }
}
