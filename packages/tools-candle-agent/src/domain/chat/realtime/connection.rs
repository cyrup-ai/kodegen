//! Connection management for real-time chat
//!
//! This module provides comprehensive connection management with heartbeat monitoring,
//! health checks, and connection state tracking using atomic operations and lock-free
//! data structures for maximum performance.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ahash::RandomState;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::events::{ConnectionStatus, RealTimeEvent};
use crate::domain::util::duration_to_nanos_u64;

/// Connection state with atomic operations
#[derive(Debug)]
pub struct ConnectionState {
    /// Unique connection ID
    pub connection_id: String,
    /// User ID
    pub user_id: String,
    /// Session ID
    pub session_id: String,
    /// Last activity timestamp (nanoseconds since epoch)
    last_activity: AtomicU64,
    /// Connection status
    status: ArcSwap<ConnectionStatus>,
    /// Number of reconnection attempts
    reconnection_attempts: AtomicUsize,
    /// Total messages sent
    messages_sent: AtomicUsize,
    /// Total messages received
    messages_received: AtomicUsize,
    /// Total bytes sent
    bytes_sent: AtomicUsize,
    /// Total bytes received
    bytes_received: AtomicUsize,
    /// Connection start time
    connected_at: Instant,
    /// Whether the connection is active
    is_active: AtomicBool,
}

impl ConnectionState {
    /// Create a new connection state
    #[must_use]
    pub fn new(user_id: String, session_id: String) -> Self {
        let now = Instant::now();
        let now_nanos = duration_to_nanos_u64(now.elapsed());

        Self {
            connection_id: Uuid::new_v4().to_string(),
            user_id,
            session_id,
            last_activity: AtomicU64::new(now_nanos),
            status: ArcSwap::from_pointee(ConnectionStatus::Connecting),
            reconnection_attempts: AtomicUsize::new(0),
            messages_sent: AtomicUsize::new(0),
            messages_received: AtomicUsize::new(0),
            bytes_sent: AtomicUsize::new(0),
            bytes_received: AtomicUsize::new(0),
            connected_at: now,
            is_active: AtomicBool::new(true),
        }
    }

    /// Update the last activity timestamp
    pub fn update_heartbeat(&self) {
        let now_nanos = duration_to_nanos_u64(Instant::now().elapsed());
        self.last_activity.store(now_nanos, Ordering::Release);
    }

    /// Check if the connection is healthy
    pub fn is_connection_healthy(&self, heartbeat_timeout: u64) -> bool {
        if !self.is_active.load(Ordering::Acquire) {
            return false;
        }

        let now_nanos = duration_to_nanos_u64(Instant::now().elapsed());
        let last_activity = self.last_activity.load(Ordering::Acquire);

        now_nanos.saturating_sub(last_activity) < heartbeat_timeout * 1_000_000_000
    }

    /// Set connection status
    pub fn set_status(&self, status: ConnectionStatus) {
        self.status.store(Arc::new(status));
    }

    /// Get connection status
    pub fn get_status(&self) -> ConnectionStatus {
        *self.status.load_full()
    }

    /// Increment reconnection attempts
    pub fn increment_reconnection_attempts(&self) {
        self.reconnection_attempts.fetch_add(1, Ordering::AcqRel);
    }

    /// Reset reconnection attempts
    pub fn reset_reconnection_attempts(&self) {
        self.reconnection_attempts.store(0, Ordering::Release);
    }

    /// Record sent message
    pub fn record_sent_message(&self, bytes: usize) {
        self.messages_sent.fetch_add(1, Ordering::AcqRel);
        self.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
        self.update_heartbeat();
    }

    /// Record received message
    pub fn record_received_message(&self, bytes: usize) {
        self.messages_received.fetch_add(1, Ordering::AcqRel);
        self.bytes_received.fetch_add(bytes, Ordering::AcqRel);
        self.update_heartbeat();
    }

    /// Close the connection
    pub fn close(&self) {
        self.is_active.store(false, Ordering::Release);
        self.status.store(Arc::new(ConnectionStatus::Disconnected));
    }

    /// Get connection statistics
    pub fn get_statistics(&self) -> ConnectionStatistics {
        ConnectionStatistics {
            connection_id: self.connection_id.clone(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            status: *self.status.load_full(),
            uptime_seconds: self.connected_at.elapsed().as_secs(),
            messages_sent: self.messages_sent.load(Ordering::Acquire),
            messages_received: self.messages_received.load(Ordering::Acquire),
            bytes_sent: self.bytes_sent.load(Ordering::Acquire),
            bytes_received: self.bytes_received.load(Ordering::Acquire),
            reconnection_attempts: self.reconnection_attempts.load(Ordering::Acquire),
            last_activity: self.last_activity.load(Ordering::Acquire),
            is_active: self.is_active.load(Ordering::Acquire),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatistics {
    /// Connection ID
    pub connection_id: String,
    /// User ID
    pub user_id: String,
    /// Session ID
    pub session_id: String,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Total messages sent
    pub messages_sent: usize,
    /// Total messages received
    pub messages_received: usize,
    /// Total bytes sent
    pub bytes_sent: usize,
    /// Total bytes received
    pub bytes_received: usize,
    /// Number of reconnection attempts
    pub reconnection_attempts: usize,
    /// Last activity timestamp (nanoseconds since epoch)
    pub last_activity: u64,
    /// Whether the connection is currently active
    pub is_active: bool,
}

/// Connection manager with heartbeat and health monitoring
#[derive(Debug)]
pub struct ConnectionManager {
    /// Active connections with concurrent access
    connections: Arc<DashMap<String, Arc<ConnectionState>, RandomState>>,
    /// Heartbeat timeout in seconds
    heartbeat_timeout: u64,
    /// Health check interval in seconds
    health_check_interval: u64,
    /// Event sender for connection events (broadcast for pub-sub pattern)
    event_sender: broadcast::Sender<RealTimeEvent>,
    /// Whether the health check task is running
    health_check_running: Arc<AtomicBool>,
    /// Total connections handled
    total_connections: AtomicUsize,
    /// Active connections count
    active_connections: Arc<AtomicUsize>,
    /// Failed health checks
    failed_health_checks: Arc<AtomicUsize>,
    /// Successful reconnections
    successful_reconnections: Arc<AtomicUsize>,
}

impl ConnectionManager {
    /// Create a new connection manager
    #[must_use]
    pub fn new(heartbeat_timeout: u64, health_check_interval: u64) -> Self {
        // Create broadcast channel with capacity of 100 for connection events
        let (event_sender, _) = broadcast::channel(100);

        Self {
            connections: Arc::new(DashMap::with_hasher(RandomState::default())),
            heartbeat_timeout,
            health_check_interval,
            event_sender,
            health_check_running: Arc::new(AtomicBool::new(false)),
            total_connections: AtomicUsize::new(0),
            active_connections: Arc::new(AtomicUsize::new(0)),
            failed_health_checks: Arc::new(AtomicUsize::new(0)),
            successful_reconnections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Add a new connection
    ///
    /// # Errors
    ///
    /// Returns error string if connection cannot be added to the pool
    pub fn add_connection(
        &self,
        user_id: String,
        session_id: String,
    ) -> Result<Arc<ConnectionState>, String> {
        let connection = Arc::new(ConnectionState::new(user_id, session_id));

        self.connections
            .insert(connection.connection_id.clone(), connection.clone());
        self.total_connections.fetch_add(1, Ordering::AcqRel);
        self.active_connections.fetch_add(1, Ordering::AcqRel);

        // Notify about the new connection
        let _ = self
            .event_sender
            .send(RealTimeEvent::connection_status_changed(
                connection.user_id.clone(),
                ConnectionStatus::Connected,
            ));

        Ok(connection)
    }

    /// Remove a connection
    ///
    /// # Errors
    ///
    /// Returns error string if connection with the given ID does not exist
    pub fn remove_connection(&self, connection_id: &String) -> Result<(), String> {
        if let Some((_key, connection)) = self.connections.remove(connection_id) {
            connection.close();
            self.active_connections.fetch_sub(1, Ordering::AcqRel);

            // Notify about the disconnection
            let _ = self
                .event_sender
                .send(RealTimeEvent::connection_status_changed(
                    connection.user_id.clone(),
                    ConnectionStatus::Disconnected,
                ));

            Ok(())
        } else {
            Err("Connection not found".to_string())
        }
    }

    /// Get a connection by ID
    pub fn get_connection(&self, connection_id: &str) -> Option<Arc<ConnectionState>> {
        self.connections
            .get(connection_id)
            .map(|e| e.value().clone())
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> Vec<Arc<ConnectionState>> {
        self.connections.iter().map(|e| e.value().clone()).collect()
    }

    /// Start health check task
    pub fn start_health_check(&self) -> bool {
        if self.health_check_running.load(Ordering::Acquire) {
            return false;
        }

        self.health_check_running.store(true, Ordering::Release);

        let running = self.health_check_running.clone();
        let connections = Arc::clone(&self.connections);
        let heartbeat_timeout = self.heartbeat_timeout;
        let event_sender = self.event_sender.clone();
        let failed_checks = Arc::clone(&self.failed_health_checks);
        let active_connections_counter = Arc::clone(&self.active_connections);

        tokio::spawn(async move {
            log::info!("Health check task started (interval: 1s, timeout: {heartbeat_timeout}s)");

            let mut interval = tokio::time::interval(Duration::from_secs(1));
            while running.load(Ordering::Acquire) {
                interval.tick().await;

                // Step 1: Identify stale connections
                let stale_connections: Vec<(String, String, usize)> = connections
                    .iter()
                    .filter_map(|entry| {
                        let conn_id = entry.key().clone();
                        let conn = entry.value();

                        if conn.is_connection_healthy(heartbeat_timeout) {
                            None
                        } else {
                            let user_id = conn.user_id.clone();
                            let attempts = conn.reconnection_attempts.load(Ordering::Acquire);
                            Some((conn_id, user_id, attempts))
                        }
                    })
                    .collect();

                // Step 2: Handle each stale connection
                for (conn_id, user_id, attempts) in stale_connections {
                    if let Some(conn_entry) = connections.get(&conn_id) {
                        let conn = conn_entry.value();

                        if attempts < 3 {
                            // Attempt reconnection
                            conn.increment_reconnection_attempts();
                            conn.set_status(ConnectionStatus::Reconnecting);

                            log::warn!(
                                "Connection {} (user: {}) unhealthy, reconnect attempt {}/3",
                                conn_id,
                                user_id,
                                attempts + 1
                            );

                            // Send reconnection event
                            let _ = event_sender.send(RealTimeEvent::connection_status_changed(
                                user_id.clone(),
                                ConnectionStatus::Reconnecting,
                            ));
                        } else {
                            // Max reconnection attempts exceeded - remove connection
                            log::error!(
                                "Connection {conn_id} (user: {user_id}) failed after 3 reconnection attempts, removing"
                            );

                            conn.set_status(ConnectionStatus::Failed);

                            // Send failure event
                            let _ = event_sender.send(RealTimeEvent::connection_status_changed(
                                user_id.clone(),
                                ConnectionStatus::Failed,
                            ));

                            // Remove from connections and update counters
                            if connections.remove(&conn_id).is_some() {
                                failed_checks.fetch_add(1, Ordering::AcqRel);
                                active_connections_counter.fetch_sub(1, Ordering::AcqRel);
                            }
                        }
                    }
                }
            }

            log::info!("Health check task stopped");
        });

        true
    }

    /// Stop health check task
    pub fn stop_health_check(&self) {
        self.health_check_running.store(false, Ordering::Release);
    }

    /// Get manager statistics
    pub fn get_statistics(&self) -> ConnectionManagerStatistics {
        ConnectionManagerStatistics {
            total_connections: self.total_connections.load(Ordering::Acquire),
            active_connections: self.active_connections.load(Ordering::Acquire),
            failed_health_checks: self.failed_health_checks.load(Ordering::Acquire),
            successful_reconnections: self.successful_reconnections.load(Ordering::Acquire),
            heartbeat_timeout: self.heartbeat_timeout,
            health_check_interval: self.health_check_interval,
        }
    }

    /// Subscribe to connection events
    pub fn subscribe(&self) -> broadcast::Receiver<RealTimeEvent> {
        self.event_sender.subscribe()
    }
}

/// Connection manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionManagerStatistics {
    /// Total connections handled
    pub total_connections: usize,
    /// Currently active connections
    pub active_connections: usize,
    /// Number of failed health checks
    pub failed_health_checks: usize,
    /// Number of successful reconnections
    pub successful_reconnections: usize,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout: u64,
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        self.stop_health_check();
    }
}
