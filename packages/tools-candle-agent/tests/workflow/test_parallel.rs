// Tests extracted from src/workflow/parallel.rs

use kodegen_candle_agent::workflow::parallel::*;
use kodegen_candle_agent::parallel;
use kodegen_candle_agent::workflow::ops::map;
use cyrup_sugars::prelude::MessageChunk;

// Simple test wrapper for i32 that implements MessageChunk
#[derive(Debug, Clone, Default, PartialEq)]
struct TestChunk(i32);

impl TestChunk {
    fn value(&self) -> i32 {
        self.0
    }

    fn new(value: i32) -> Self {
        Self(value)
    }
}

impl cyrup_sugars::prelude::MessageChunk for TestChunk {
    fn bad_chunk(_error: String) -> Self {
        Self::default()
    }

    fn error(&self) -> Option<&str> {
        None
    }
}

impl From<i32> for TestChunk {
    fn from(value: i32) -> Self {
        TestChunk(value)
    }
}

#[test]
fn test_parallel_n_creation() {
    let parallel: ParallelN<i32, TestChunk> = ParallelN::new();
    assert_eq!(parallel.operation_count(), 0);
    assert!(parallel.is_stack_allocated());
}

#[test]
fn test_parallel_builder() {
    let builder: ParallelBuilder<i32, TestChunk> = ParallelBuilder::new();
    assert_eq!(builder.operation_count(), 0);
    assert!(builder.is_stack_allocated());
}

#[test]
fn test_chunk_wrapper() {
    // Test TestChunk creation and value access
    let chunk = TestChunk::new(42);
    assert_eq!(chunk.value(), 42);

    // Test From trait
    let chunk_from: TestChunk = 100.into();
    assert_eq!(chunk_from.value(), 100);

    // Test equality
    let chunk1 = TestChunk::new(50);
    let chunk2 = TestChunk::new(50);
    assert_eq!(chunk1, chunk2);

    // Test MessageChunk trait
    assert!(chunk.error().is_none());
    let bad_chunk = TestChunk::bad_chunk("test error".to_string());
    assert_eq!(bad_chunk.value(), 0); // Default value
}

#[test]
fn test_stack_allocation_threshold() {
    let mut parallel: ParallelN<i32, TestChunk> = ParallelN::new();

    // Add 16 operations - should still be stack allocated
    for _ in 0..16 {
        parallel = parallel.add_operation(map(|x: i32| TestChunk::from(x + 1)));
    }
    assert!(parallel.is_stack_allocated());
    assert_eq!(parallel.operation_count(), 16);

    // Add 17th operation - should trigger heap allocation
    parallel = parallel.add_operation(map(|x: i32| TestChunk::from(x + 1)));
    assert!(!parallel.is_stack_allocated());
    assert_eq!(parallel.operation_count(), 17);
}

#[test]
fn test_parallel_macro() {
    let op1 = map(|x: i32| TestChunk::from(x + 1));
    let op2 = map(|x: i32| TestChunk::from(x * 2));
    let op3 = map(|x: i32| TestChunk::from(x - 1));

    let parallel_ops = parallel![op1, op2, op3];
    assert_eq!(parallel_ops.operation_count(), 3);
    assert!(parallel_ops.is_stack_allocated());
}
