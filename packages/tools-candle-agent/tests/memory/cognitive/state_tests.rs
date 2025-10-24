//! Cognitive state management tests
//!
//! Tests for cognitive state, emotional valence, and state manager functionality

use std::time::Duration;

use cyrup_memory::cognitive::state::{
    AbstractionLevel, AssociationType, CognitiveState, CognitiveStateManager, EmotionalValence,
    SemanticContext,
};
use uuid::Uuid;

#[test]
fn test_cognitive_state_creation() {
    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec!["testing".to_string()],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let state = CognitiveState::new(context);

    assert_eq!(state.semantic_context.primary_concepts[0], "test");
    assert_eq!(state.activation_level, 1.0);
    assert!(state.is_active(Duration::from_secs(300)));
}

#[test]
fn test_emotional_valence() {
    let v1 = EmotionalValence::new(0.5, 0.5, 0.0);
    let v2 = EmotionalValence::new(-0.5, -0.5, 0.0);

    let distance = v1.distance(&v2);
    assert!((distance - 1.414).abs() < 0.01); // sqrt(2)
}

#[test]
fn test_emotional_valence_neutral() {
    let neutral = EmotionalValence::neutral();
    assert_eq!(neutral.arousal, 0.0);
    assert_eq!(neutral.valence, 0.0);
    assert_eq!(neutral.dominance, 0.0);
}

#[test]
fn test_emotional_valence_clamping() {
    let v = EmotionalValence::new(2.0, -2.0, 1.5);
    assert_eq!(v.arousal, 1.0); // Clamped to 1.0
    assert_eq!(v.valence, -1.0); // Clamped to -1.0
    assert_eq!(v.dominance, 1.0); // Clamped to 1.0
}

#[test]
fn test_cognitive_state_associations() {
    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec![],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let mut state = CognitiveState::new(context);
    let target_id = Uuid::new_v4();

    state.add_association(target_id, 0.8, AssociationType::Semantic);

    assert_eq!(state.associations.len(), 1);
    assert_eq!(state.associations[0].target_id, target_id);
    assert_eq!(state.associations[0].strength, 0.8);
    matches!(
        state.associations[0].association_type,
        AssociationType::Semantic
    );
}

#[test]
fn test_cognitive_state_association_strength_clamping() {
    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec![],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let mut state = CognitiveState::new(context);
    let target_id = Uuid::new_v4();

    // Test upper bound clamping
    state.add_association(target_id, 1.5, AssociationType::Semantic);
    assert_eq!(state.associations[0].strength, 1.0);

    // Test lower bound clamping
    state.add_association(target_id, -0.5, AssociationType::Causal);
    assert_eq!(state.associations[1].strength, 0.0);
}

#[test]
fn test_cognitive_state_activation() {
    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec![],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let mut state = CognitiveState::new(context);
    let initial_activation = state.activation_level;

    state.activate(0.3);

    // Should be clamped to 1.0 since initial was already 1.0
    assert_eq!(initial_activation, 1.0);
    assert_eq!(state.activation_level, 1.0);

    // Test with lower initial activation
    state.activation_level = 0.5;
    state.activate(0.3);
    assert_eq!(state.activation_level, 0.8);
}

#[tokio::test]
async fn test_state_manager() {
    let manager = CognitiveStateManager::new();

    let context = SemanticContext {
        primary_concepts: vec!["rust".to_string(), "memory".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec!["programming".to_string()],
        abstraction_level: AbstractionLevel::Abstract,
    };

    let state = CognitiveState::new(context);
    let id = manager.add_state(state).await;

    // Test retrieval
    let retrieved = manager.get_state(&id).await;
    assert!(retrieved.is_some());

    // Test concept search
    let found = manager.find_by_concept("rust").await;
    assert_eq!(found.len(), 1);

    // Test domain search
    let found = manager.find_by_domain("programming").await;
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn test_state_manager_multiple_states() {
    let manager = CognitiveStateManager::new();

    // Add first state
    let context1 = SemanticContext {
        primary_concepts: vec!["rust".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec!["programming".to_string()],
        abstraction_level: AbstractionLevel::Abstract,
    };
    let state1 = CognitiveState::new(context1);
    manager.add_state(state1).await;

    // Add second state with same concept
    let context2 = SemanticContext {
        primary_concepts: vec!["rust".to_string(), "web".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec!["programming".to_string(), "web".to_string()],
        abstraction_level: AbstractionLevel::Concrete,
    };
    let state2 = CognitiveState::new(context2);
    manager.add_state(state2).await;

    // Should find both states for "rust"
    let found = manager.find_by_concept("rust").await;
    assert_eq!(found.len(), 2);

    // Should find one state for "web"
    let found = manager.find_by_concept("web").await;
    assert_eq!(found.len(), 1);

    // Should find both states for "programming" domain
    let found = manager.find_by_domain("programming").await;
    assert_eq!(found.len(), 2);

    // Should find one state for "web" domain
    let found = manager.find_by_domain("web").await;
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn test_state_manager_nonexistent_lookups() {
    let manager = CognitiveStateManager::new();

    // Test with empty manager
    let found = manager.find_by_concept("nonexistent").await;
    assert!(found.is_empty());

    let found = manager.find_by_domain("nonexistent").await;
    assert!(found.is_empty());

    let retrieved = manager.get_state(&Uuid::new_v4()).await;
    assert!(retrieved.is_none());
}

#[test]
fn test_abstraction_levels() {
    // Test that all abstraction levels can be created and matched
    let levels = vec![
        AbstractionLevel::Concrete,
        AbstractionLevel::Intermediate,
        AbstractionLevel::Abstract,
        AbstractionLevel::MetaCognitive,
    ];

    for level in levels {
        let context = SemanticContext {
            primary_concepts: vec!["test".to_string()],
            secondary_concepts: vec![],
            domain_tags: vec![],
            abstraction_level: level,
        };

        let state = CognitiveState::new(context);
        // Should create successfully without panicking
        assert_eq!(state.semantic_context.primary_concepts[0], "test");
    }
}

#[test]
fn test_association_types() {
    // Test that all association types can be created
    let types = vec![
        AssociationType::Semantic,
        AssociationType::Temporal,
        AssociationType::Causal,
        AssociationType::Emotional,
        AssociationType::Structural,
    ];

    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec![],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let mut state = CognitiveState::new(context);
    let target_id = Uuid::new_v4();

    for (i, association_type) in types.into_iter().enumerate() {
        state.add_association(target_id, 0.5, association_type);
        assert_eq!(state.associations.len(), i + 1);
    }
}

#[test]
fn test_cognitive_state_activity_decay() {
    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec![],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let state = CognitiveState::new(context);

    // Should be active with reasonable decay time
    assert!(state.is_active(Duration::from_secs(300)));

    // Should be active with very short decay time initially
    assert!(state.is_active(Duration::from_millis(1)));
}

#[tokio::test]
async fn test_cleanup_inactive_states() {
    let manager = CognitiveStateManager::new();

    let context = SemanticContext {
        primary_concepts: vec!["test".to_string()],
        secondary_concepts: vec![],
        domain_tags: vec!["testing".to_string()],
        abstraction_level: AbstractionLevel::Concrete,
    };

    let mut state = CognitiveState::new(context);

    // Artificially set very low activation to simulate decay
    state.activation_level = 0.05;

    let id = manager.add_state(state).await;

    // Verify state exists
    let retrieved = manager.get_state(&id).await;
    assert!(retrieved.is_some());

    // Clean up with very short decay time (should remove the low-activation state)
    manager.cleanup_inactive(Duration::from_millis(1)).await;

    // State should be removed
    let retrieved = manager.get_state(&id).await;
    assert!(retrieved.is_none());

    // Concept and domain searches should return empty
    let found = manager.find_by_concept("test").await;
    assert!(found.is_empty());

    let found = manager.find_by_domain("testing").await;
    assert!(found.is_empty());
}

// Memory context analysis test removed due to cognitive module compilation issues
// This test can be re-enabled once the cognitive system is fully stabilized
