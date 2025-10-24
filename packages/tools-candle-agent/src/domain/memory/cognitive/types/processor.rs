//! High-level cognitive processing system with pattern matching and decision making

use crossbeam_skiplist::SkipMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use kodegen_simd::similarity::cosine_similarity;

use super::atomics::AtomicF32;
use super::state::{CognitiveError, CognitiveResult, CognitiveState};
use crate::domain::util::unix_timestamp_nanos;

/// High-level cognitive memory system
#[derive(Debug, Clone)]
pub struct CognitiveMemory {
    /// Core cognitive state
    state: Arc<CognitiveState>,
    /// Pattern storage for memory consolidation
    pattern_storage: Arc<SkipMap<Uuid, CognitivePattern>>,
    /// Performance metrics
    metrics: Arc<CognitiveMetrics>,
    /// Memory configuration
    config: CognitiveMemoryConfig,
}

/// Cognitive processor for pattern matching and decision making
#[derive(Debug, Clone)]
pub struct CognitiveProcessor {
    /// Processor configuration
    config: CognitiveProcessorConfig,
    /// Processing state
    state: Arc<ProcessingState>,
    /// Pattern matcher
    pattern_matcher: Arc<PatternMatcher>,
    /// Decision engine
    decision_engine: Arc<DecisionEngine>,
}

/// Configuration for cognitive memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMemoryConfig {
    /// Maximum number of patterns to store
    pub max_patterns: usize,
    /// Consolidation threshold for moving to long-term memory
    pub consolidation_threshold: f32,
    /// Pattern retention duration in seconds
    pub pattern_retention_seconds: u64,
    /// Enable performance monitoring
    pub enable_monitoring: bool,
}

/// Configuration for cognitive processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveProcessorConfig {
    /// Batch size for processing
    pub batch_size: usize,
    /// Decision threshold for accept/reject
    pub decision_threshold: f32,
    /// Learning rate for adaptation
    pub learning_rate: f32,
    /// Maximum iterations for processing
    pub max_iterations: u64,
}

/// Cognitive pattern for memory storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePattern {
    /// Pattern identifier
    pub id: Uuid,
    /// Pattern data (feature vector)
    pub data: Vec<f32>,
    /// Pattern strength
    pub strength: f32,
    /// Access count
    pub access_count: u64,
    /// Last access timestamp
    pub last_access: SystemTime,
}

/// Performance metrics for cognitive operations
#[derive(Debug)]
pub struct CognitiveMetrics {
    /// Number of patterns processed
    pub patterns_processed: AtomicU64,
    /// Number of decisions made
    pub decisions_made: AtomicU64,
    /// Average processing time in microseconds
    pub avg_processing_time_us: AtomicU64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: AtomicF32,
}

/// Processing state for cognitive operations
#[derive(Debug)]
pub struct ProcessingState {
    /// Whether currently processing
    pub is_processing: AtomicBool,
    /// Current iteration
    pub current_iteration: AtomicU64,
    /// Start time in nanoseconds
    pub start_time: AtomicU64,
}

/// Pattern matcher for cognitive processing
#[derive(Debug)]
pub struct PatternMatcher {
    /// Matching threshold
    threshold: f32,
    /// Stored patterns for matching
    patterns: Vec<Vec<f32>>,
    /// Pattern cache for performance
    cache: Arc<SkipMap<Uuid, f32>>,
}

/// Decision engine for cognitive processing
#[derive(Debug)]
pub struct DecisionEngine {
    /// Decision threshold
    threshold: f32,
    /// Decision history channel
    history_tx: mpsc::UnboundedSender<Decision>,
    /// Decision history receiver (protected)
    _history_rx: Arc<Mutex<mpsc::UnboundedReceiver<Decision>>>,
}

/// Represents a cognitive decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Decision identifier
    pub id: Uuid,
    /// Decision confidence
    pub confidence: f32,
    /// Decision timestamp
    pub timestamp: SystemTime,
    /// Decision outcome
    pub outcome: DecisionOutcome,
}

/// Possible decision outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionOutcome {
    /// Accept the decision
    Accept,
    /// Reject the decision
    Reject,
    /// Defer the decision
    Defer,
    /// Request more information
    RequestInfo,
}

impl CognitiveMemory {
    /// Create a new cognitive memory system
    #[must_use]
    pub fn new(config: CognitiveMemoryConfig) -> Self {
        Self {
            state: Arc::new(CognitiveState::new()),
            pattern_storage: Arc::new(SkipMap::new()),
            metrics: Arc::new(CognitiveMetrics::new()),
            config,
        }
    }

    /// Get the current cognitive state
    #[must_use]
    pub fn state(&self) -> &Arc<CognitiveState> {
        &self.state
    }

    /// Store a cognitive pattern
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if pattern storage capacity is exceeded
    pub fn store_pattern(&self, pattern: CognitivePattern) -> CognitiveResult<()> {
        if self.pattern_storage.len() >= self.config.max_patterns {
            return Err(CognitiveError::MemoryCapacityExceeded(format!(
                "Cannot store more than {} patterns",
                self.config.max_patterns
            )));
        }

        self.pattern_storage.insert(pattern.id, pattern);
        self.metrics
            .patterns_processed
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Retrieve a cognitive pattern by ID
    #[must_use]
    pub fn get_pattern(&self, id: &Uuid) -> Option<CognitivePattern> {
        self.pattern_storage
            .get(id)
            .map(|entry| entry.value().clone())
    }

    /// Get performance metrics
    #[must_use]
    pub fn metrics(&self) -> &CognitiveMetrics {
        &self.metrics
    }
}

impl CognitiveProcessor {
    /// Create a new cognitive processor
    #[must_use]
    pub fn new(config: CognitiveProcessorConfig) -> Self {
        Self {
            config,
            state: Arc::new(ProcessingState::new()),
            pattern_matcher: Arc::new(PatternMatcher::new(0.8)),
            decision_engine: Arc::new(DecisionEngine::new(0.7)),
        }
    }

    /// Process cognitive input and return decision
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if pattern matching or decision making fails
    pub fn process(&self, input: &[f32]) -> CognitiveResult<Decision> {
        // Set processing state
        self.state.is_processing.store(true, Ordering::Relaxed);
        let start_time = unix_timestamp_nanos();
        self.state.start_time.store(start_time, Ordering::Relaxed);

        // Generate pattern ID for caching
        let pattern_id = Uuid::new_v4();

        // Check cache first
        let pattern_match =
            if let Some(cached_result) = self.pattern_matcher.get_cached_result(&pattern_id) {
                cached_result
            } else {
                // Match patterns
                let match_result = self.pattern_matcher.match_pattern(input)?;
                // Cache the result
                self.pattern_matcher
                    .cache_pattern_result(pattern_id, match_result);
                match_result
            };

        // Make decision
        let decision = self.decision_engine.make_decision(pattern_match)?;

        // Update state
        self.state.current_iteration.fetch_add(1, Ordering::Relaxed);
        self.state.is_processing.store(false, Ordering::Relaxed);

        Ok(decision)
    }

    /// Get current processing state
    #[must_use]
    pub fn is_processing(&self) -> bool {
        self.state.is_processing.load(Ordering::Relaxed)
    }

    /// Get processor configuration
    #[inline]
    #[must_use]
    pub fn config(&self) -> &CognitiveProcessorConfig {
        &self.config
    }

    /// Update processor configuration
    #[inline]
    pub fn update_config(&mut self, config: CognitiveProcessorConfig) {
        self.config = config;
    }

    /// Clear pattern matcher cache
    #[inline]
    pub fn clear_pattern_cache(&self) {
        self.pattern_matcher.clear_cache();
    }

    /// Get pattern cache size for monitoring
    #[inline]
    #[must_use]
    pub fn pattern_cache_size(&self) -> usize {
        self.pattern_matcher.cache_size()
    }

    /// Get cache performance statistics
    #[inline]
    #[must_use]
    pub fn cache_performance(&self) -> (usize, bool) {
        let size = self.pattern_matcher.cache_size();
        let needs_cleanup = size > 1000; // Example threshold
        (size, needs_cleanup)
    }
}

impl Default for CognitiveMemoryConfig {
    fn default() -> Self {
        Self {
            max_patterns: 10000,
            consolidation_threshold: 0.8,
            pattern_retention_seconds: 86400, // 24 hours
            enable_monitoring: true,
        }
    }
}

impl Default for CognitiveProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            decision_threshold: 0.7,
            learning_rate: 0.01,
            max_iterations: 1000,
        }
    }
}

impl CognitiveMetrics {
    /// Create new metrics
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns_processed: AtomicU64::new(0),
            decisions_made: AtomicU64::new(0),
            avg_processing_time_us: AtomicU64::new(0),
            success_rate: AtomicF32::new(0.0),
        }
    }
}

impl Default for CognitiveMetrics {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingState {
    /// Create new processing state
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_processing: AtomicBool::new(false),
            current_iteration: AtomicU64::new(0),
            start_time: AtomicU64::new(0),
        }
    }
}

impl Default for ProcessingState {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl PatternMatcher {
    /// Create new pattern matcher
    #[must_use]
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            patterns: Vec::new(),
            cache: Arc::new(SkipMap::new()),
        }
    }

    /// Add a reference pattern for matching
    pub fn add_pattern(&mut self, pattern: Vec<f32>) {
        self.patterns.push(pattern);
    }

    /// Clear all stored patterns
    pub fn clear_patterns(&mut self) {
        self.patterns.clear();
    }

    /// Match input against stored patterns using SIMD-optimized cosine similarity
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if pattern strength is below threshold
    pub fn match_pattern(&self, input: &[f32]) -> CognitiveResult<f32> {
        // Handle edge cases
        if input.is_empty() {
            return Ok(0.0);
        }

        if self.patterns.is_empty() {
            return Ok(0.0);
        }

        // Find best matching pattern using SIMD-optimized cosine similarity
        let mut best_similarity = -1.0f32; // Start at minimum possible value

        for stored_pattern in &self.patterns {
            // Skip dimension mismatches
            if stored_pattern.len() != input.len() {
                continue;
            }

            // Check for zero-magnitude vectors
            let input_magnitude: f32 = input.iter().map(|x| x * x).sum::<f32>().sqrt();
            let pattern_magnitude: f32 = stored_pattern.iter().map(|x| x * x).sum::<f32>().sqrt();

            if input_magnitude == 0.0 || pattern_magnitude == 0.0 {
                continue;
            }

            // Use SIMD-optimized cosine similarity from kodegen_simd
            let similarity = cosine_similarity(input, stored_pattern);
            best_similarity = best_similarity.max(similarity);
        }

        // Normalize from [-1, 1] to [0, 1] range for threshold comparison
        let normalized_strength = f32::midpoint(best_similarity, 1.0);

        if normalized_strength >= self.threshold {
            Ok(normalized_strength)
        } else {
            Ok(0.0)
        }
    }

    /// Cache pattern match result
    #[inline]
    pub fn cache_pattern_result(&self, pattern_id: Uuid, strength: f32) {
        self.cache.insert(pattern_id, strength);
    }

    /// Get cached pattern result
    #[inline]
    #[must_use]
    pub fn get_cached_result(&self, pattern_id: &Uuid) -> Option<f32> {
        self.cache.get(pattern_id).map(|entry| *entry.value())
    }

    /// Clear pattern cache
    #[inline]
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache size for monitoring
    #[inline]
    #[must_use]
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl DecisionEngine {
    /// Create new decision engine
    #[must_use]
    pub fn new(threshold: f32) -> Self {
        let (history_tx, history_rx) = mpsc::unbounded_channel();
        Self {
            threshold,
            history_tx,
            _history_rx: Arc::new(Mutex::new(history_rx)),
        }
    }

    /// Make a decision based on pattern match strength
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError::Other` if decision history channel is closed
    pub fn make_decision(&self, pattern_strength: f32) -> CognitiveResult<Decision> {
        let decision = Decision {
            id: Uuid::new_v4(),
            confidence: pattern_strength,
            timestamp: SystemTime::now(),
            outcome: if pattern_strength >= self.threshold {
                DecisionOutcome::Accept
            } else if pattern_strength >= self.threshold * 0.5 {
                DecisionOutcome::Defer
            } else {
                DecisionOutcome::Reject
            },
        };

        self.history_tx.send(decision.clone()).map_err(|e| {
            CognitiveError::OperationFailed(format!("Decision history channel closed: {e}"))
        })?;
        Ok(decision)
    }
}
