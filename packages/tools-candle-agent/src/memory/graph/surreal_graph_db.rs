//! SurrealDB implementation of GraphDatabase trait

use std::sync::Arc;

use serde::Deserialize;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::sync::oneshot;

use super::graph_db::{
    GraphDatabase, GraphError, GraphQueryOptions, Node, NodeId, NodeQuery, NodeStream, NodeUpdate,
    PendingBatchResult, PendingNode, Result,
};

/// SurrealDB-backed graph database implementation
#[derive(Clone)]
pub struct SurrealGraphDatabase {
    client: Arc<Surreal<Any>>,
}

impl SurrealGraphDatabase {
    /// Create a new SurrealDB graph database instance
    pub fn new(client: Arc<Surreal<Any>>) -> Self {
        Self { client }
    }
}

impl GraphDatabase for SurrealGraphDatabase {
    fn create_node(&self, properties: super::graph_db::NodeProperties) -> PendingNode {
        let (tx, rx) = oneshot::channel();
        let client = self.client.clone();

        tokio::spawn(async move {
            let result: Result<NodeId> = async {
                // Extract table name from properties or use default
                let table = properties
                    .get("_table")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "node".to_string());

                // Filter out metadata fields starting with underscore
                let content: std::collections::HashMap<String, serde_json::Value> = properties
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Create node with generated ID using ULID
                #[derive(Deserialize)]
                struct Created {
                    id: String,
                }

                let created: Option<Created> = client
                    .create(&table)
                    .content(content)
                    .await
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                let node_id = created
                    .map(|c| c.id)
                    .ok_or_else(|| GraphError::DatabaseError("No node created".to_string()))?;

                Ok(node_id)
            }
            .await;

            let _ = tx.send(result);
        });

        PendingNode::new(rx)
    }

    fn get_node(&self, id: &str) -> NodeQuery {
        let (tx, rx) = oneshot::channel();
        let client = self.client.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result: Result<Option<Node>> = async {
                // Parse table:id format
                let parts: Vec<&str> = id.split(':').collect();
                if parts.len() != 2 {
                    return Err(GraphError::ValidationError(
                        "ID must be in format 'table:id'".to_string(),
                    ));
                }

                let node: Option<Node> = client
                    .select((parts[0], parts[1]))
                    .await
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                Ok(node)
            }
            .await;

            let _ = tx.send(result);
        });

        NodeQuery::new(rx)
    }

    fn update_node(&self, id: &str, properties: super::graph_db::NodeProperties) -> NodeUpdate {
        let (tx, rx) = oneshot::channel();
        let client = self.client.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result: Result<()> = async {
                // Parse table:id format
                let parts: Vec<&str> = id.split(':').collect();
                if parts.len() != 2 {
                    return Err(GraphError::ValidationError(
                        "ID must be in format 'table:id'".to_string(),
                    ));
                }

                // Filter out metadata fields
                let content: std::collections::HashMap<String, serde_json::Value> = properties
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_'))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let _: Option<Node> = client
                    .update((parts[0], parts[1]))
                    .content(content)
                    .await
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                Ok(())
            }
            .await;

            let _ = tx.send(result);
        });

        NodeUpdate::new(rx)
    }

    fn delete_node(&self, id: &str) -> NodeUpdate {
        let (tx, rx) = oneshot::channel();
        let client = self.client.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result: Result<()> = async {
                // Parse table:id format
                let parts: Vec<&str> = id.split(':').collect();
                if parts.len() != 2 {
                    return Err(GraphError::ValidationError(
                        "ID must be in format 'table:id'".to_string(),
                    ));
                }

                let _: Option<Node> = client
                    .delete((parts[0], parts[1]))
                    .await
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                Ok(())
            }
            .await;

            let _ = tx.send(result);
        });

        NodeUpdate::new(rx)
    }

    fn get_nodes_by_type(&self, node_type: &str) -> NodeStream {
        let (tx, _rx_nodes) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let node_type = node_type.to_string();

        tokio::spawn(async move {
            let query = format!("SELECT * FROM {}", node_type);

            match client.query(query).await {
                Ok(mut response) => match response.take::<Vec<Node>>(0) {
                    Ok(nodes) => {
                        for node in nodes {
                            if tx.send(Ok(node)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                            .await;
                    }
                },
                Err(e) => {
                    let _ = tx
                        .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                        .await;
                }
            }
        });

        NodeStream::new(_rx_nodes)
    }

    fn query(&self, query: &str, params: Option<GraphQueryOptions>) -> NodeStream {
        let (tx, rx_nodes) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let mut query_str = query.to_string();

        tokio::spawn(async move {
            // Apply limit and offset to query if provided
            if let Some(opts) = params {
                if let Some(limit) = opts.limit {
                    query_str.push_str(&format!(" LIMIT {}", limit));
                }
                if let Some(offset) = opts.offset {
                    query_str.push_str(&format!(" START {}", offset));
                }

                // Bind filters as parameters
                if !opts.filters.is_empty() {
                    let mut query_builder = client.query(&query_str);
                    for (key, value) in opts.filters {
                        query_builder = query_builder.bind((key, value));
                    }

                    match query_builder.await {
                        Ok(mut response) => match response.take::<Vec<Node>>(0) {
                            Ok(nodes) => {
                                for node in nodes {
                                    if tx.send(Ok(node)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                                    .await;
                            }
                        },
                        Err(e) => {
                            let _ = tx
                                .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                                .await;
                        }
                    }
                    return;
                }
            }

            // Simple query without parameters
            match client.query(&query_str).await {
                Ok(mut response) => match response.take::<Vec<Node>>(0) {
                    Ok(nodes) => {
                        for node in nodes {
                            if tx.send(Ok(node)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                            .await;
                    }
                },
                Err(e) => {
                    let _ = tx
                        .send(Err(GraphError::DatabaseError(format!("{:?}", e))))
                        .await;
                }
            }
        });

        NodeStream::new(rx_nodes)
    }

    fn batch_query(&self, query: &str, params: serde_json::Value) -> PendingBatchResult {
        let (tx, rx) = oneshot::channel();
        let client = self.client.clone();
        let query = query.to_string();

        tokio::spawn(async move {
            let result: Result<Vec<Node>> = async {
                // Build query with bindings
                let mut query_builder = client.query(&query);

                // Bind parameters from JSON object
                if let serde_json::Value::Object(map) = params {
                    for (key, value) in map {
                        query_builder = query_builder.bind((key, value));
                    }
                }

                let mut response = query_builder
                    .await
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                // Take results from last statement (typically the FOR loop statement)
                // For batch operations, results may be in any statement, try statement 0
                let nodes: Vec<Node> = response
                    .take(0)
                    .map_err(|e| GraphError::DatabaseError(format!("{:?}", e)))?;

                Ok(nodes)
            }
            .await;

            let _ = tx.send(result);
        });

        PendingBatchResult::new(rx)
    }
}