//! Session ID mapping for STDIO MCP server
//!
//! This module provides session ID mapping functionality to isolate sessions
//! between different stdio connections when proxying to HTTP MCP servers.
//!
//! ## Problem
//!
//! When multiple Claude Code instances (or other MCP clients) connect to the
//! same kodegen stdio server, and that server proxies to HTTP MCP servers,
//! session IDs from different clients can collide. For example:
//!
//! - Client A starts a browser research session with session_id="abc123"
//! - Client B starts a browser research session with session_id="abc123"
//! - Both get routed to the same HTTP server, causing conflicts
//!
//! ## Solution
//!
//! The SessionMapper maintains a mapping of (connection_id, client_session_id) → server_session_id
//! where:
//! - connection_id: Unique UUID per stdio connection
//! - client_session_id: Session ID from the MCP client
//! - server_session_id: Unique UUID sent to the HTTP server
//!
//! This ensures that even if two clients use the same session_id, they map
//! to different UUIDs on the HTTP server side.

use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Key for session mapping: (connection_id, client_session_id)
type SessionKey = (String, String);

/// Session ID mapper for isolating stdio connections
#[derive(Clone)]
pub struct SessionMapper {
    /// Thread-safe mapping from (connection_id, client_session_id) to server_session_id
    /// 
    /// The outer key is the stdio connection ID (UUID).
    /// The inner key is the client's session ID (from tool args).
    /// The value is the server session ID (UUID) sent to HTTP servers.
    mappings: Arc<DashMap<SessionKey, String>>,
}

impl SessionMapper {
    /// Create a new SessionMapper
    pub fn new() -> Self {
        Self {
            mappings: Arc::new(DashMap::new()),
        }
    }

    /// Map a client session ID to a server session ID for a specific connection
    ///
    /// If a mapping already exists for this (connection_id, client_session_id) pair,
    /// returns the existing server session ID. Otherwise, generates a new UUID.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for the stdio connection
    /// * `client_session_id` - Session ID from the MCP client's tool arguments
    ///
    /// # Returns
    ///
    /// The server session ID (UUID) to use when calling the HTTP server
    pub fn map_session_id(&self, connection_id: &str, client_session_id: &str) -> String {
        let key = (connection_id.to_string(), client_session_id.to_string());
        
        // Try to get existing mapping
        if let Some(entry) = self.mappings.get(&key) {
            return entry.value().clone();
        }

        // Generate new UUID for this session
        let server_session_id = Uuid::new_v4().to_string();
        
        // Store the mapping
        self.mappings.insert(key.clone(), server_session_id.clone());
        
        log::debug!(
            "Mapped session: connection={}, client_session={}, server_session={}",
            connection_id,
            client_session_id,
            server_session_id
        );
        
        server_session_id
    }

    /// Get the mapped server session ID for a given client session ID
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for the stdio connection
    /// * `client_session_id` - Session ID from the MCP client
    ///
    /// # Returns
    ///
    /// The mapped server session ID, or None if no mapping exists
    #[allow(dead_code)]
    pub fn get_mapped_id(&self, connection_id: &str, client_session_id: &str) -> Option<String> {
        let key = (connection_id.to_string(), client_session_id.to_string());
        self.mappings.get(&key).map(|entry| entry.value().clone())
    }

    /// Clean up all session mappings for a specific connection
    ///
    /// Called when a stdio connection is closed to free memory.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - Unique identifier for the stdio connection
    ///
    /// # Returns
    ///
    /// Number of sessions cleaned up
    pub fn cleanup_connection(&self, connection_id: &str) -> usize {
        let mut cleaned = 0;
        
        // Collect keys to remove (avoid holding lock during iteration)
        let keys_to_remove: Vec<SessionKey> = self
            .mappings
            .iter()
            .filter(|entry| entry.key().0 == connection_id)
            .map(|entry| entry.key().clone())
            .collect();
        
        // Remove all sessions for this connection
        for key in keys_to_remove {
            self.mappings.remove(&key);
            cleaned += 1;
        }
        
        if cleaned > 0 {
            log::info!(
                "Cleaned up {} session mapping(s) for connection {}",
                cleaned,
                connection_id
            );
        }
        
        cleaned
    }

    /// Get the total number of active session mappings
    ///
    /// Useful for debugging and monitoring
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Check if there are no active session mappings
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

impl Default for SessionMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_mapper_new_mapping() {
        let mapper = SessionMapper::new();
        let conn_id = "conn-1";
        let client_session = "session-abc";
        
        let server_session_1 = mapper.map_session_id(conn_id, client_session);
        let server_session_2 = mapper.map_session_id(conn_id, client_session);
        
        // Same mapping should return same server session ID
        assert_eq!(server_session_1, server_session_2);
        
        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&server_session_1).is_ok());
    }

    #[test]
    fn test_session_mapper_different_connections() {
        let mapper = SessionMapper::new();
        let client_session = "session-xyz";
        
        let server_session_conn1 = mapper.map_session_id("conn-1", client_session);
        let server_session_conn2 = mapper.map_session_id("conn-2", client_session);
        
        // Different connections with same client session ID should map to different server IDs
        assert_ne!(server_session_conn1, server_session_conn2);
    }

    #[test]
    fn test_session_mapper_get_mapped_id() {
        let mapper = SessionMapper::new();
        let conn_id = "conn-1";
        let client_session = "session-def";
        
        // Initially, no mapping exists
        assert!(mapper.get_mapped_id(conn_id, client_session).is_none());
        
        // Create mapping
        let server_session = mapper.map_session_id(conn_id, client_session);
        
        // Now get_mapped_id should return the same ID
        assert_eq!(
            mapper.get_mapped_id(conn_id, client_session),
            Some(server_session)
        );
    }

    #[test]
    fn test_session_mapper_cleanup() {
        let mapper = SessionMapper::new();
        let conn_id_1 = "conn-1";
        let conn_id_2 = "conn-2";
        
        // Create mappings for two connections
        mapper.map_session_id(conn_id_1, "session-a");
        mapper.map_session_id(conn_id_1, "session-b");
        mapper.map_session_id(conn_id_2, "session-c");
        
        assert_eq!(mapper.len(), 3);
        
        // Cleanup connection 1
        let cleaned = mapper.cleanup_connection(conn_id_1);
        assert_eq!(cleaned, 2);
        assert_eq!(mapper.len(), 1);
        
        // Connection 2's session should still exist
        assert!(mapper.get_mapped_id(conn_id_2, "session-c").is_some());
        
        // Connection 1's sessions should be gone
        assert!(mapper.get_mapped_id(conn_id_1, "session-a").is_none());
        assert!(mapper.get_mapped_id(conn_id_1, "session-b").is_none());
    }

    #[test]
    fn test_session_mapper_empty() {
        let mapper = SessionMapper::new();
        assert!(mapper.is_empty());
        
        mapper.map_session_id("conn-1", "session-1");
        assert!(!mapper.is_empty());
        
        mapper.cleanup_connection("conn-1");
        assert!(mapper.is_empty());
    }
}
