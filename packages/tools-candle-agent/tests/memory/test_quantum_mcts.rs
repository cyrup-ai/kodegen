// tests/test_quantum_mcts.rs
//! Integration tests for quantum MCTS recursive improvement

#![cfg(feature = "cognitive")]

use std::sync::Arc;

use cyrup_memory::cognitive::{
    committee::CommitteeEvent,
    mcts::CodeState,
    performance::PerformanceAnalyzer,
    quantum::QuantumConfig,
    quantum_mcts::QuantumMCTS,
    quantum_orchestrator::{QuantumOrchestrationConfig, QuantumOrchestrator},
    types::{
        ContentCategory, ContentType, EvolutionRules, OptimizationSpec, OptimizationType,
        Restrictions, SecurityLevel,
    },
};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_quantum_mcts_basic() {
    // Setup
    let (event_tx, mut event_rx) = mpsc::channel(100);

    let initial_state = CodeState {
        code: r#"
fn process_data(items: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        if item > 0 {
            result.push(item * 2);
        }
    }
    result
}
"#
        .to_string(),
        code_content: "process_data function for data processing".to_string(),
        latency: 100.0,
        memory: 50.0,
        relevance: 80.0,
    };

    let spec = Arc::new(OptimizationSpec {
        content_type: ContentType {
            category: ContentCategory::Code,
            complexity: 0.5,
            processing_hints: vec!["optimization".to_string()],
            format: "rust".to_string(),
            restrictions: Restrictions {
                max_memory_usage: Some(1024),
                max_processing_time: Some(1000),
                allowed_operations: vec!["loop_optimization".to_string()],
                forbidden_operations: vec![],
                security_level: SecurityLevel::Internal,
                compiler: "rustc".to_string(),
                max_latency_increase: 10.0,
                max_memory_increase: 20.0,
                min_relevance_improvement: 5.0,
            },
        },
        baseline_metrics: initial_state.clone(),
        objective: "Optimize for performance while maintaining accuracy".to_string(),
        improvement_threshold: 0.1,
        constraints: vec!["memory_safe".to_string()],
        success_criteria: vec!["performance_improved".to_string()],
        optimization_type: OptimizationType::Performance,
        timeout_ms: Some(5000),
        max_iterations: Some(100),
        target_quality: 0.8,
        evolution_rules: EvolutionRules::default(),
    });

    let performance_analyzer = Arc::new(PerformanceAnalyzer::new());
    let config = QuantumConfig::default();

    // Create quantum MCTS
    let mut quantum_mcts = QuantumMCTS::new(
        initial_state,
        "Optimize data processing".to_string(),
        config,
    )
    .unwrap();

    // Run recursive improvement
    quantum_mcts.recursive_improve(50).unwrap();

    // Get results
    let best_modification = quantum_mcts.best_quantum_modification();
    assert!(best_modification.is_some());

    let stats = quantum_mcts.get_quantum_statistics();
    assert!(stats.total_nodes > 1);
    assert!(stats.total_visits > 0);

    // Verify quantum properties
    assert!(stats.max_amplitude > 0.0);
    assert!(stats.avg_decoherence < 1.0);
}

#[tokio::test]
async fn test_quantum_orchestrator() {
    // Setup
    let (event_tx, mut event_rx) = mpsc::channel(100);

    let initial_state = CodeState {
        code: r#"
async fn fetch_data(urls: Vec<String>) -> Vec<Result<String, Error>> {
    let mut results = Vec::new();
    for url in urls {
        match fetch(url).await {
            Ok(data) => results.push(Ok(data)),
            Err(e) => results.push(Err(e))}
    }
    results
}
"#
        .to_string(),
        code_content: "async fetch_data function for parallel data fetching".to_string(),
        latency: 200.0,
        memory: 100.0,
        relevance: 75.0,
    };

    let spec = Arc::new(OptimizationSpec {
        content_type: ContentType {
            category: ContentCategory::Code,
            complexity: 0.7,
            processing_hints: vec!["async".to_string(), "parallelization".to_string()],
            format: "rust".to_string(),
            restrictions: Restrictions {
                max_memory_usage: Some(2048),
                max_processing_time: Some(2000),
                allowed_operations: vec!["async_optimization".to_string()],
                forbidden_operations: vec![],
                security_level: SecurityLevel::Internal,
                compiler: "rustc".to_string(),
                max_latency_increase: 5.0,
                max_memory_increase: 10.0,
                min_relevance_improvement: 10.0,
            },
        },
        baseline_metrics: initial_state.clone(),
        objective: "Parallelize async operations for better performance".to_string(),
        improvement_threshold: 0.15,
        constraints: vec!["async_safe".to_string()],
        success_criteria: vec!["async_performance_improved".to_string()],
        optimization_type: OptimizationType::Performance,
        timeout_ms: Some(7000),
        max_iterations: Some(150),
        target_quality: 0.85,
        evolution_rules: EvolutionRules::default(),
    });

    let performance_analyzer = Arc::new(PerformanceAnalyzer::new());
    let orchestration_config = QuantumOrchestrationConfig {
        max_recursive_depth: 3,
        improvement_threshold: 0.03,
        coherence_time_ms: 100,
        parallel_circuits: 2,
        convergence_epsilon: 0.01,
        max_iterations_per_depth: 30,
    };
    let mcts_config = QuantumConfig::default();

    // Create orchestrator
    let orchestrator = QuantumOrchestrator::new(
        orchestration_config,
        mcts_config,
        performance_analyzer,
        event_tx,
    )
    .await
    .unwrap();

    // Run recursive improvement
    let outcome = orchestrator
        .run_recursive_improvement(initial_state, spec, "Optimize async fetching".to_string())
        .await
        .unwrap();

    // Verify results
    assert!(outcome.improvement_percentage > 0.0);
    assert!(!outcome.optimized_code.is_empty());
    assert!(
        outcome
            .applied_techniques
            .contains(&"quantum_mcts".to_string())
    );

    // Check improvement history
    let history = orchestrator.get_improvement_history().await;
    assert!(!history.is_empty());

    // Visualize evolution
    let visualization = orchestrator.visualize_evolution().await.unwrap();
    assert!(visualization.contains("Quantum Recursive Improvement"));
}

#[tokio::test]
async fn test_quantum_convergence() {
    // Setup
    let (event_tx, _) = mpsc::channel(100);

    let initial_state = CodeState {
        code: "fn simple() -> i32 { 42 }".to_string(),
        code_content: "simple function returning constant value".to_string(),
        latency: 10.0,
        memory: 5.0,
        relevance: 100.0,
    };

    let spec = Arc::new(OptimizationSpec {
        content_type: ContentType {
            category: ContentCategory::Code,
            complexity: 0.1,
            processing_hints: vec!["simple".to_string()],
            format: "rust".to_string(),
            restrictions: Restrictions {
                max_memory_usage: Some(512),
                max_processing_time: Some(100),
                allowed_operations: vec!["basic_optimization".to_string()],
                forbidden_operations: vec![],
                security_level: SecurityLevel::Public,
                compiler: "rustc".to_string(),
                max_latency_increase: 1.0,
                max_memory_increase: 1.0,
                min_relevance_improvement: 0.0,
            },
        },
        baseline_metrics: initial_state.clone(),
        objective: "Already optimal code".to_string(),
        improvement_threshold: 0.05,
        constraints: vec!["minimal_change".to_string()],
        success_criteria: vec!["no_regression".to_string()],
        optimization_type: OptimizationType::Quality,
        timeout_ms: Some(3000),
        max_iterations: Some(50),
        target_quality: 0.95,
        evolution_rules: EvolutionRules::default(),
    });

    let performance_analyzer = Arc::new(PerformanceAnalyzer::new());
    let config = QuantumConfig {
        max_superposition_states: 2,
        decoherence_threshold: 0.1,
        ..Default::default()
    };

    // Create quantum MCTS
    let mut quantum_mcts =
        QuantumMCTS::new(initial_state, "Test convergence".to_string(), config).unwrap();

    // Run and expect quick convergence
    quantum_mcts.recursive_improve(20).await.unwrap();

    let stats = quantum_mcts.get_quantum_statistics();
    assert!(stats.total_nodes < 50); // Should converge quickly
}

#[tokio::test]
async fn test_quantum_entanglement_effects() {
    // Setup
    let (event_tx, _) = mpsc::channel(100);

    let initial_state = CodeState {
        code: r#"
fn matrix_multiply(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let m = b[0].len();
    let p = b.len();
    let mut result = vec![vec![0.0; m]; n];
    
    for i in 0..n {
        for j in 0..m {
            for k in 0..p {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}
"#
        .to_string(),
        code_content: "matrix multiplication function with cache optimization potential"
            .to_string(),
        latency: 500.0,
        memory: 200.0,
        relevance: 90.0,
    };

    let spec = Arc::new(OptimizationSpec {
        content_type: ContentType {
            category: ContentCategory::Code,
            complexity: 0.9,
            processing_hints: vec!["matrix".to_string(), "cache_optimization".to_string()],
            format: "rust".to_string(),
            restrictions: Restrictions {
                max_memory_usage: Some(4096),
                max_processing_time: Some(5000),
                allowed_operations: vec![
                    "matrix_optimization".to_string(),
                    "cache_blocking".to_string(),
                ],
                forbidden_operations: vec![],
                security_level: SecurityLevel::Restricted,
                compiler: "rustc".to_string(),
                max_latency_increase: 0.0,
                max_memory_increase: 50.0,
                min_relevance_improvement: 0.0,
            },
        },
        baseline_metrics: initial_state.clone(),
        objective: "Optimize matrix multiplication with cache efficiency".to_string(),
        improvement_threshold: 0.2,
        constraints: vec!["cache_friendly".to_string(), "vectorizable".to_string()],
        success_criteria: vec![
            "cache_efficiency_improved".to_string(),
            "performance_boost".to_string(),
        ],
        optimization_type: OptimizationType::Efficiency,
        timeout_ms: Some(10000),
        max_iterations: Some(200),
        target_quality: 0.9,
        evolution_rules: EvolutionRules::default(),
    });

    let performance_analyzer = Arc::new(PerformanceAnalyzer::new());
    let config = QuantumConfig {
        entanglement_probability: 0.9,
        exploration_constant: 3.0,
        ..Default::default()
    };

    // Create quantum MCTS
    let mut quantum_mcts =
        QuantumMCTS::new(initial_state, "Matrix optimization".to_string(), config).unwrap();

    // Run improvement
    quantum_mcts.recursive_improve(100).await.unwrap();

    let stats = quantum_mcts.get_quantum_statistics();

    // Verify entanglement was created
    assert!(stats.total_entanglements > 0);
    assert!(stats.total_entanglements as f64 / stats.total_nodes as f64 > 0.1);
}

#[cfg(test)]
mod performance_mock {
    use super::*;

    impl PerformanceAnalyzer {
        pub fn new() -> Self {
            Self {
                // Mock implementation
            }
        }

        pub async fn estimate_reward(
            &self,
            state: &CodeState,
        ) -> Result<f64, Box<dyn std::error::Error>> {
            // Simple reward calculation for testing
            let latency_score = 100.0 / state.latency;
            let memory_score = 50.0 / state.memory;
            let relevance_score = state.relevance / 100.0;

            Ok(latency_score * 0.5 + memory_score * 0.3 + relevance_score * 0.2)
        }
    }
}
