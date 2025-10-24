//! DFA-based index for efficient schema constraint validation
//!
//! This module implements high-performance constraint validation using deterministic
//! finite automata (DFA) built from regex patterns. Based on the outlines-core approach
//! with zero-allocation optimizations for blazing-fast token validation.

use anyhow::{Context, Result as AnyResult};
use regex_automata::{
    Anchored, MatchKind,
    dfa::{
        Automaton,
        dense::{Builder, Config, DFA},
    },
    util::primitives::StateID as AutomataStateId,
};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::sync::Arc;
use tokenizers::Tokenizer;

use crate::logits::constraints::GenerationConstraint;

/// Type alias for state identifiers in the constraint system
pub type StateId = u32;

/// Type alias for token identifiers
pub type TokenId = u32;

/// High-performance index for DFA-based token validation
///
/// The `SchemaIndex` efficiently maps vocabulary tokens to state transitions
/// using a precomputed transition table for `O(1)` token validation.
#[derive(Clone, Debug)]
pub struct SchemaIndex {
    /// The initial state of the DFA
    initial_state: StateId,
    /// Set of accepting/final states
    final_states: HashSet<StateId>,
    /// State transition table: current_state -> (token_id -> next_state)
    transitions: HashMap<StateId, HashMap<TokenId, StateId>>,
    /// Size of the vocabulary
    vocab_size: usize,
    /// End-of-sequence token identifier
    eos_token_id: TokenId,
    /// Dead state identifier for invalid transitions
    dead_state: StateId,
}

impl SchemaIndex {
    /// Build a new `SchemaIndex` from regex pattern and vocabulary
    pub fn new(regex_pattern: &str, vocabulary: &SchemaVocabulary) -> AnyResult<Self> {
        // Build DFA from regex pattern with optimized settings
        let dfa = Builder::new()
            .configure(
                Config::new()
                    .match_kind(MatchKind::All)
                    .minimize(true)
                    .byte_classes(true),
            )
            .build(regex_pattern)
            .context("Failed to build DFA from regex pattern")?;

        let mut index = Self {
            initial_state: 0,
            final_states: HashSet::default(),
            transitions: HashMap::default(),
            vocab_size: vocabulary.vocab_size(),
            eos_token_id: vocabulary.eos_token_id(),
            dead_state: u32::MAX,
        };

        index
            .build_transitions(&dfa, vocabulary)
            .context("Failed to build transition table")?;

        Ok(index)
    }

    /// Build the complete state transition table from the DFA
    fn build_transitions(
        &mut self,
        dfa: &DFA<Vec<u32>>,
        vocabulary: &SchemaVocabulary,
    ) -> AnyResult<()> {
        let initial_state_id = dfa.universal_start_state(Anchored::Yes).unwrap_or_else(|| {
            // Fallback to creating a basic start state
            AutomataStateId::from_ne_bytes([0, 0, 0, 0]).unwrap_or(AutomataStateId::ZERO)
        });
        self.initial_state = initial_state_id.as_u32();

        // Use BFS to explore all reachable states
        let mut state_queue = vec![initial_state_id];
        let mut visited_states = HashSet::default();
        visited_states.insert(initial_state_id);

        // Set a default dead state (we'll detect it during traversal)
        self.dead_state = u32::MAX;

        while let Some(current_state) = state_queue.pop() {
            let current_id = current_state.as_u32();

            // Check if this is a final state
            if dfa.is_match_state(current_state) {
                self.final_states.insert(current_id);
            }

            // Skip dead states
            if dfa.is_dead_state(current_state) {
                continue;
            }

            let mut token_transitions = HashMap::default();

            // Test each token in vocabulary for valid transitions
            for token_id in 0..self.vocab_size as u32 {
                if let Some(token_bytes) = vocabulary.token_bytes(token_id)
                    && let Some(next_state) =
                        self.compute_transition(dfa, current_state, token_bytes)?
                {
                    let next_id = next_state.as_u32();

                    // Only add non-dead state transitions
                    if !dfa.is_dead_state(next_state) {
                        token_transitions.insert(token_id, next_id);

                        // Add to queue if not visited
                        if !visited_states.contains(&next_state) {
                            visited_states.insert(next_state);
                            state_queue.push(next_state);
                        }
                    }
                }
            }

            // Store transitions for this state
            if !token_transitions.is_empty() {
                self.transitions.insert(current_id, token_transitions);
            }
        }

        Ok(())
    }

    /// Compute the next state after consuming a token's bytes
    fn compute_transition(
        &self,
        dfa: &DFA<Vec<u32>>,
        current_state: AutomataStateId,
        token_bytes: &[u8],
    ) -> AnyResult<Option<AutomataStateId>> {
        if token_bytes.is_empty() {
            return Ok(Some(current_state));
        }

        let mut state = current_state;

        // Process each byte in the token
        for &byte in token_bytes {
            state = dfa.next_state(state, byte);

            // Stop if we hit a dead state
            if dfa.is_dead_state(state) {
                return Ok(None);
            }
        }

        Ok(Some(state))
    }

    /// Get the initial state of the DFA
    #[inline]
    #[must_use]
    pub const fn initial_state(&self) -> StateId {
        self.initial_state
    }

    /// Check if a state is a final/accepting state
    #[inline]
    #[must_use]
    pub fn is_final_state(&self, state: StateId) -> bool {
        self.final_states.contains(&state)
    }

    /// Get the next state after consuming a token
    #[inline]
    #[must_use]
    pub fn next_state(&self, current_state: StateId, token_id: TokenId) -> Option<StateId> {
        self.transitions
            .get(&current_state)?
            .get(&token_id)
            .copied()
    }

    /// Get all allowed tokens from a given state
    #[inline]
    #[must_use]
    pub fn allowed_tokens(&self, state: StateId) -> Option<&HashMap<TokenId, StateId>> {
        self.transitions.get(&state)
    }

    /// Get the vocabulary size
    #[inline]
    #[must_use]
    pub const fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Get the end-of-sequence token ID
    #[inline]
    #[must_use]
    pub const fn eos_token_id(&self) -> TokenId {
        self.eos_token_id
    }

    /// Check if a state is the dead state
    #[inline]
    #[must_use]
    pub const fn is_dead_state(&self, state: StateId) -> bool {
        state == self.dead_state
    }

    /// Get statistics about the index
    #[must_use]
    pub fn stats(&self) -> IndexStats {
        let total_transitions: usize = self
            .transitions
            .values()
            .map(|transitions| transitions.len())
            .sum();

        IndexStats {
            num_states: self.transitions.len(),
            num_final_states: self.final_states.len(),
            num_transitions: total_transitions,
            vocab_size: self.vocab_size,
            initial_state: self.initial_state,
        }
    }
}

/// Statistics about a `SchemaIndex`
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// Total number of states in the DFA
    pub num_states: usize,
    /// Number of accepting/final states
    pub num_final_states: usize,
    /// Total number of state transitions
    pub num_transitions: usize,
    /// Size of the vocabulary
    pub vocab_size: usize,
    /// Initial state ID for the DFA
    pub initial_state: StateId,
}

/// Vocabulary wrapper for efficient token-to-bytes mapping
///
/// Provides zero-allocation access to token byte representations
/// with optimized lookups for constraint validation.
#[derive(Debug, Clone)]
pub struct SchemaVocabulary {
    /// Mapping from token ID to byte representation
    token_to_bytes: Vec<Vec<u8>>,
    /// End-of-sequence token identifier
    eos_token_id: TokenId,
    /// Total vocabulary size
    vocab_size: usize,
}

impl SchemaVocabulary {
    /// Create vocabulary from a tokenizer
    pub fn from_tokenizer(tokenizer: &Tokenizer) -> AnyResult<Self> {
        let vocab_size = tokenizer.get_vocab_size(false);
        let mut token_to_bytes = Vec::with_capacity(vocab_size);

        // Build token-to-bytes mapping
        for token_id in 0..vocab_size as u32 {
            let bytes = tokenizer
                .id_to_token(token_id)
                .map(|s| s.into_bytes())
                .unwrap_or_default();
            token_to_bytes.push(bytes);
        }

        // Detect EOS token ID using common patterns
        let eos_token_id = Self::detect_eos_token(tokenizer);

        Ok(Self {
            token_to_bytes,
            eos_token_id,
            vocab_size,
        })
    }

    /// Create vocabulary from existing token mappings
    pub fn from_tokens(token_to_bytes: Vec<Vec<u8>>, eos_token_id: TokenId) -> Self {
        let vocab_size = token_to_bytes.len();
        Self {
            token_to_bytes,
            eos_token_id,
            vocab_size,
        }
    }

    /// Detect EOS token ID from tokenizer
    fn detect_eos_token(tokenizer: &Tokenizer) -> TokenId {
        // Common EOS token patterns
        let eos_patterns = [
            "<|endoftext|>",
            "</s>",
            "<eos>",
            "<|end|>",
            "[EOS]",
            "<end>",
        ];

        for pattern in eos_patterns {
            if let Some(token_id) = tokenizer.token_to_id(pattern) {
                return token_id;
            }
        }

        // Fallback: use token ID 0 or search for likely candidates
        tokenizer
            .get_vocab(false)
            .iter()
            .find(|(token, _)| {
                let token_lower = token.to_lowercase();
                token_lower.contains("eos") || token_lower.contains("end") || token_lower == "</s>"
            })
            .map(|(_, &token_id)| token_id)
            .unwrap_or(0)
    }

    /// Get byte representation of a token
    #[inline]
    #[must_use]
    pub fn token_bytes(&self, token_id: TokenId) -> Option<&[u8]> {
        self.token_to_bytes
            .get(token_id as usize)
            .map(|v| v.as_slice())
    }

    /// Get vocabulary size
    #[inline]
    #[must_use]
    pub const fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Get EOS token ID
    #[inline]
    #[must_use]
    pub const fn eos_token_id(&self) -> TokenId {
        self.eos_token_id
    }

    /// Check if a token ID is valid
    #[inline]
    #[must_use]
    pub const fn is_valid_token(&self, token_id: TokenId) -> bool {
        (token_id as usize) < self.vocab_size
    }
}

/// State for schema constraint validation
///
/// Tracks the current position in the DFA and completion status
/// with minimal memory overhead for high-performance validation.
#[derive(Clone, Debug)]
pub struct SchemaConstraintState {
    /// Current state in the DFA
    current_state: StateId,
    /// Whether we've reached a final state
    is_complete: bool,
    /// Number of tokens processed
    tokens_processed: usize,
}

impl SchemaConstraintState {
    /// Create new state at the initial DFA state
    #[inline]
    #[must_use]
    pub const fn new(initial_state: StateId) -> Self {
        Self {
            current_state: initial_state,
            is_complete: false,
            tokens_processed: 0,
        }
    }

    /// Get the current DFA state
    #[inline]
    #[must_use]
    pub const fn current_state(&self) -> StateId {
        self.current_state
    }

    /// Check if we've reached a complete/final state
    #[inline]
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get number of tokens processed
    #[inline]
    #[must_use]
    pub const fn tokens_processed(&self) -> usize {
        self.tokens_processed
    }

    /// Update state after processing a token
    #[inline]
    pub fn update_state(&mut self, new_state: StateId, is_final: bool) {
        self.current_state = new_state;
        self.is_complete = is_final;
        self.tokens_processed += 1;
    }

    /// Reset state to initial values
    #[inline]
    pub fn reset(&mut self, initial_state: StateId) {
        self.current_state = initial_state;
        self.is_complete = false;
        self.tokens_processed = 0;
    }
}

/// High-performance schema constraint implementation
///
/// Uses precomputed DFA transitions for O(1) token validation
/// with minimal memory allocations during constraint checking.
#[derive(Debug, Clone)]
pub struct SchemaConstraint {
    /// DFA-based transition index
    index: Arc<SchemaIndex>,
    /// Vocabulary for token-to-bytes mapping
    vocabulary: Arc<SchemaVocabulary>,
    /// Optional name for debugging
    name: Option<String>,
    /// Whether to allow partial/incomplete schema generation
    allow_partial: bool,
}

impl SchemaConstraint {
    /// Create new schema constraint from regex pattern
    pub fn new(
        regex_pattern: &str,
        vocabulary: Arc<SchemaVocabulary>,
        allow_partial: bool,
    ) -> AnyResult<Self> {
        let index = Arc::new(SchemaIndex::new(regex_pattern, &vocabulary)?);
        Ok(Self {
            index,
            vocabulary,
            name: None,
            allow_partial,
        })
    }

    /// Create schema constraint with a name for debugging
    #[must_use]
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Get allowed tokens for current state (zero-allocation)
    #[inline]
    #[must_use]
    pub fn get_allowed_tokens(
        &self,
        state: &SchemaConstraintState,
    ) -> Option<&HashMap<TokenId, StateId>> {
        self.index.allowed_tokens(state.current_state())
    }

    /// Check if a specific token is allowed in current state
    #[inline]
    #[must_use]
    pub fn is_token_allowed(&self, state: &SchemaConstraintState, token_id: TokenId) -> bool {
        self.index
            .next_state(state.current_state(), token_id)
            .is_some()
    }

    /// Get vocabulary reference
    #[must_use]
    pub fn vocabulary(&self) -> &SchemaVocabulary {
        &self.vocabulary
    }

    /// Get index statistics
    #[must_use]
    pub fn index_stats(&self) -> IndexStats {
        self.index.stats()
    }

    /// Get constraint name (if set)
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl GenerationConstraint for SchemaConstraint {
    type State = SchemaConstraintState;

    #[inline]
    fn new_state(&self) -> Self::State {
        SchemaConstraintState::new(self.index.initial_state())
    }

    #[inline]
    fn update(&self, state: &mut Self::State, token: u32) -> AnyResult<bool> {
        if let Some(next_state) = self.index.next_state(state.current_state(), token) {
            let is_final = self.index.is_final_state(next_state);
            state.update_state(next_state, is_final);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[inline]
    fn try_next(&self, state: &Self::State, token: u32) -> AnyResult<bool> {
        let current_state = state.current_state();
        let eos_token_id = self.vocabulary.eos_token_id();

        // Special handling for EOS token based on allow_partial mode
        if token == eos_token_id {
            if self.allow_partial {
                // Partial mode: Allow EOS in any non-dead state
                // User accepts incomplete schema
                Ok(!self.index.is_dead_state(current_state))
            } else {
                // Strict mode: Only allow EOS in final states
                // Schema must be complete
                Ok(self.index.is_final_state(current_state))
            }
        } else {
            // Regular token: check for valid DFA transition
            Ok(self.index.next_state(current_state, token).is_some())
        }
    }

    #[inline]
    fn is_done(&self, state: &Self::State) -> bool {
        // Allow completion at any state if partial matching enabled,
        // otherwise only at final states
        self.allow_partial || self.index.is_final_state(state.current_state())
    }

    fn get_deterministic_sequence(&self, state: &Self::State) -> AnyResult<Vec<u32>> {
        let mut sequence = Vec::new();
        let mut current_state = state.current_state();
        let max_sequence_length = 100; // Prevent infinite sequences

        // Follow deterministic path when only one token option is available
        for _ in 0..max_sequence_length {
            if let Some(transitions) = self.index.allowed_tokens(current_state) {
                if transitions.len() == 1 {
                    // Only one valid token - deterministic choice
                    if let Some((&token_id, &next_state)) = transitions.iter().next() {
                        sequence.push(token_id);
                        current_state = next_state;
                    } else {
                        break; // Safety: empty transitions despite len() == 1
                    }

                    // Stop if we've reached a final state
                    if self.index.is_final_state(current_state) {
                        break;
                    }
                } else {
                    // Multiple choices - stop deterministic sequence
                    break;
                }
            } else {
                // No valid transitions - dead end
                break;
            }
        }

        Ok(sequence)
    }
}

/// Utility functions for working with schema constraints
pub mod utils {
    use super::*;

    /// Create a simple string pattern constraint
    pub fn string_pattern_constraint(
        pattern: &str,
        vocabulary: Arc<SchemaVocabulary>,
    ) -> AnyResult<SchemaConstraint> {
        let regex = format!(r#""{pattern}""#);
        SchemaConstraint::new(&regex, vocabulary, false)
    }

    /// Create an enum constraint from string values
    pub fn enum_constraint(
        values: &[&str],
        vocabulary: Arc<SchemaVocabulary>,
    ) -> AnyResult<SchemaConstraint> {
        let escaped_values: Vec<String> = values
            .iter()
            .map(|v| {
                let escaped = regex::escape(v);
                format!(r#""{escaped}""#)
            })
            .collect();

        let regex = escaped_values.join("|");
        SchemaConstraint::new(&regex, vocabulary, false)
    }

    /// Create a numeric range constraint
    pub fn numeric_range_constraint(
        min: Option<i64>,
        max: Option<i64>,
        vocabulary: Arc<SchemaVocabulary>,
    ) -> AnyResult<SchemaConstraint> {
        let regex = match (min, max) {
            (Some(min_val), Some(max_val)) if min_val >= 0 && max_val <= 9999 => {
                // Small positive range - enumerate values
                let values: Vec<String> = (min_val..=max_val).map(|n| n.to_string()).collect();
                values.join("|")
            }
            (Some(0), None) => r"(0|[1-9][0-9]*)".to_string(),
            (Some(min_val), None) if min_val > 0 => r"[1-9][0-9]*".to_string(),
            _ => r"-?(0|[1-9][0-9]*)".to_string(),
        };

        SchemaConstraint::new(&regex, vocabulary, false)
    }

    /// Create a boolean constraint
    pub fn boolean_constraint(vocabulary: Arc<SchemaVocabulary>) -> AnyResult<SchemaConstraint> {
        SchemaConstraint::new("(true|false)", vocabulary, false)
    }

    /// Create a null constraint
    pub fn null_constraint(vocabulary: Arc<SchemaVocabulary>) -> AnyResult<SchemaConstraint> {
        SchemaConstraint::new("null", vocabulary, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_vocabulary() -> SchemaVocabulary {
        let token_to_bytes = vec![
            b"hello".to_vec(),
            b"world".to_vec(),
            b"true".to_vec(),
            b"false".to_vec(),
            b"null".to_vec(),
            b"123".to_vec(),
            b"\"".to_vec(),        // Quote character for strings
            b"\"hello\"".to_vec(), // Quoted strings for enum test
            b"\"world\"".to_vec(), // Quoted strings for enum test
        ];
        SchemaVocabulary::from_tokens(token_to_bytes, 0)
    }

    #[test]
    fn test_boolean_constraint() {
        let vocab = Arc::new(mock_vocabulary());
        let constraint = match utils::boolean_constraint(vocab) {
            Ok(c) => c,
            Err(e) => panic!("Should create boolean constraint: {}", e),
        };

        let state = constraint.new_state();

        // Test valid boolean tokens
        assert!(
            constraint
                .try_next(&state, 2)
                .unwrap_or_else(|e| panic!("Failed to check 'true' token: {}", e))
        ); // "true"
        assert!(
            constraint
                .try_next(&state, 3)
                .unwrap_or_else(|e| panic!("Failed to check 'false' token: {}", e))
        ); // "false"

        // Test invalid tokens
        assert!(
            !constraint
                .try_next(&state, 0)
                .unwrap_or_else(|e| panic!("Failed to check 'hello' token: {}", e))
        ); // "hello"
        assert!(
            !constraint
                .try_next(&state, 5)
                .unwrap_or_else(|e| panic!("Failed to check '123' token: {}", e))
        ); // "123"
    }

    #[test]
    fn test_enum_constraint() {
        let vocab = Arc::new(mock_vocabulary());
        let values = ["hello", "world"];
        let constraint = match utils::enum_constraint(&values, vocab) {
            Ok(c) => c,
            Err(e) => panic!("Should create enum constraint: {}", e),
        };

        let state = constraint.new_state();

        // Test allowed values (note: this is simplified, real enum would need quoted strings)
        assert!(constraint.get_allowed_tokens(&state).is_some());
    }

    #[test]
    fn test_deterministic_sequence() {
        let vocab = Arc::new(mock_vocabulary());
        let constraint = match utils::null_constraint(vocab) {
            Ok(c) => c,
            Err(e) => panic!("Should create null constraint: {}", e),
        };

        let state = constraint.new_state();
        let sequence = match constraint.get_deterministic_sequence(&state) {
            Ok(seq) => seq,
            Err(e) => panic!("Should get sequence for null constraint: {}", e),
        };

        // For null constraint, should have deterministic sequence to "null" token
        assert!(!sequence.is_empty());
    }

    #[test]
    fn test_index_stats() {
        let vocab = Arc::new(mock_vocabulary());
        let constraint = match utils::boolean_constraint(vocab) {
            Ok(c) => c,
            Err(e) => panic!("Should create constraint: {}", e),
        };

        let stats = constraint.index_stats();
        assert!(stats.num_states > 0);
        assert!(stats.vocab_size > 0);
    }
}
