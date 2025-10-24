//! Memory Operations for Ultra-High Performance Memory System
//!
//! This module provides memory operation types, constants, and cache tracking
//! utilities for the memory system. SIMD operations are provided by kodegen_simd.
//!
//! CPU feature detection and vectorized operations are delegated to the
//! production-grade kodegen_simd crate which provides comprehensive
//! AVX512/AVX2/SSE4.1/NEON support with runtime dispatch.

// Removed unused imports: GlobalAlloc, Layout
// Removed unused import: std::arch::x86_64::* (SIMD intrinsics provided by kodegen_simd)

// Removed unused imports: align_of, size_of
// Removed unused import: std::ptr::NonNull

// Removed unused import: std::sync::Arc

// Removed unused import: arc_swap::ArcSwap
// Removed unused import: arrayvec::ArrayVec
use atomic_counter::{AtomicCounter, RelaxedCounter};
// Removed unused import: futures::stream::StreamExt
// Removed unused import: jemalloc_sys as jemalloc
// Removed unused imports: Mmap, MmapOptions
use std::sync::LazyLock;
// Removed unused import: smallvec::SmallVec
// SIMD and performance dependencies
// use packed_simd::f32x8; // Replaced with wide for Rust 1.78+ compatibility
// Removed unused import: wide::f32x8 as WideF32x8

// Removed unused imports: MemoryError, MemoryNode, MemoryRelationship, MemoryType
// Removed unused import: crate::ZeroOneOrMany

// REMOVED: kodegen_simd imports not needed - ops.rs is pure memory operations
// CPU feature detection is available in domain::memory::mod which imports directly from kodegen_simd

/// Standard embedding dimension for text embeddings (optimized for SIMD)
pub const EMBEDDING_DIMENSION: usize = 768;

/// Small embedding dimension for stack allocation (SIMD-aligned)
pub const SMALL_EMBEDDING_DIMENSION: usize = 64;

/// SIMD vector width for f32 operations
pub const SIMD_WIDTH: usize = 8;
/// Maximum stack allocation size for embeddings
#[allow(dead_code)]
pub const MAX_STACK_EMBEDDING_SIZE: usize = 512;

/// Memory pool size for vector operations
#[allow(dead_code)]
pub const VECTOR_POOL_SIZE: usize = 1024;

/// Performance statistics with atomic counters
#[allow(dead_code)]
static CACHE_HITS: LazyLock<RelaxedCounter> = LazyLock::new(|| RelaxedCounter::new(0));
#[allow(dead_code)]
static CACHE_MISSES: LazyLock<RelaxedCounter> = LazyLock::new(|| RelaxedCounter::new(0));

/// Memory operation type for workflow system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Store operation
    Store,
    /// Retrieve operation
    Retrieve,
    /// Update operation
    Update,
    /// Delete operation
    Delete,
    /// Search operation
    Search,
    /// Index operation
    Index,
}

/// Check if embedding should use stack allocation based on size
///
/// Simple heuristic: embeddings <= 512 elements (2KB for f32) use stack,
/// larger embeddings use heap to avoid stack overflow.
///
/// # Arguments
/// * `embedding_size` - Number of f32 elements in embedding
///
/// # Returns
/// `true` if safe to allocate on stack, `false` if heap required
#[inline]
#[allow(dead_code)]
#[must_use]
pub fn should_use_stack_allocation(embedding_size: usize) -> bool {
    embedding_size <= MAX_STACK_EMBEDDING_SIZE
}

/// Get vector pool allocation size
///
/// Returns the compile-time constant for vector pool sizing.
/// This is memory management configuration, not SIMD operations.
///
/// # Returns
/// Pool size (number of vectors to pre-allocate)
#[inline]
#[allow(dead_code)]
#[must_use]
pub fn get_vector_pool_size() -> usize {
    VECTOR_POOL_SIZE
}

/// Record cache hit for performance tracking
#[inline]
pub fn record_cache_hit() {
    (*CACHE_HITS).inc();
}

/// Record cache miss for performance tracking
#[inline]
pub fn record_cache_miss() {
    (*CACHE_MISSES).inc();
}
