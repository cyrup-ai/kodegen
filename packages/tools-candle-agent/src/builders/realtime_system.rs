//! Real-time system builder implementations
//!
//! All real-time system construction logic and builder patterns.

use std::collections::HashMap;
use std::sync::{Arc, atomic::AtomicUsize};
use tokio::sync::{RwLock, broadcast};

use crate::domain::chat::realtime::{
    CandleRealTimeSystem as RealTimeSystem, CandleTypingIndicator as TypingIndicator, 
    CandleLiveUpdateSystem as LiveUpdateSystem, CandleConnectionManager as ConnectionManager,
    CandleRealTimeSystemStatistics as RealTimeSystemStatistics, 
    CandleTypingStatistics as TypingStatistics, CandleLiveUpdateStatistics as LiveUpdateStatistics,
    CandleConnectionManagerStatistics as ConnectionManagerStatistics,
};

/// Real-time system builder for ergonomic configuration
pub struct RealTimeSystemBuilder {
    typing_expiry: u64,
    typing_cleanup_interval: u64,
    queue_size_limit: usize,
    backpressure_threshold: usize,
    processing_rate: u64,
    heartbeat_timeout: u64,
    health_check_interval: u64,
}

impl RealTimeSystemBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            typing_expiry: 30,
            typing_cleanup_interval: 10,
            queue_size_limit: 10000,
            backpressure_threshold: 8000,
            processing_rate: 100,
            heartbeat_timeout: 60,
            health_check_interval: 30,
        }
    }

    /// Set typing expiry duration
    pub fn typing_expiry(mut self, seconds: u64) -> Self {
        self.typing_expiry = seconds;
        self
    }

    /// Set typing cleanup interval
    pub fn typing_cleanup_interval(mut self, seconds: u64) -> Self {
        self.typing_cleanup_interval = seconds;
        self
    }

    /// Set queue size limit
    pub fn queue_size_limit(mut self, limit: usize) -> Self {
        self.queue_size_limit = limit;
        self
    }

    /// Set backpressure threshold
    pub fn backpressure_threshold(mut self, threshold: usize) -> Self {
        self.backpressure_threshold = threshold;
        self
    }

    /// Set processing rate
    pub fn processing_rate(mut self, rate: u64) -> Self {
        self.processing_rate = rate;
        self
    }

    /// Set heartbeat timeout
    pub fn heartbeat_timeout(mut self, seconds: u64) -> Self {
        self.heartbeat_timeout = seconds;
        self
    }

    /// Set health check interval
    pub fn health_check_interval(mut self, seconds: u64) -> Self {
        self.health_check_interval = seconds;
        self
    }

    /// Build the real-time system
    pub fn build(self) -> RealTimeSystem {
        let typing_indicator = Arc::new(TypingIndicator::new(
            self.typing_expiry,
            self.typing_cleanup_interval,
        ));
        let live_update_system = Arc::new(LiveUpdateSystem::new(
            self.queue_size_limit,
            self.backpressure_threshold,
            self.processing_rate,
        ));
        let connection_manager = Arc::new(ConnectionManager::new(
            self.heartbeat_timeout,
            self.health_check_interval,
        ));
        let (event_broadcaster, _) = broadcast::channel(1000);

        RealTimeSystem {
            typing_indicator,
            live_update_system,
            connection_manager,
            event_broadcaster,
            statistics: Arc::new(RwLock::new(RealTimeSystemStatistics {
                typing_stats: TypingStatistics {
                    active_users: 0,
                    total_typing_events: 0,
                    expiry_duration: self.typing_expiry,
                    cleanup_interval: self.typing_cleanup_interval,
                },
                live_update_stats: LiveUpdateStatistics {
                    total_messages: 0,
                    active_subscribers: 0,
                    queue_size: 0,
                    backpressure_events: 0,
                    processing_rate: self.processing_rate as f64,
                    last_update: 0,
                },
                connection_stats: ConnectionManagerStatistics {
                    total_connections: 0,
                    total_heartbeats: 0,
                    failed_connections: 0,
                    heartbeat_timeout: self.heartbeat_timeout,
                    health_check_interval: self.health_check_interval,
                },
                total_events: 0,
                system_uptime: 0,
            })),
        }
    }
}

impl Default for RealTimeSystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}