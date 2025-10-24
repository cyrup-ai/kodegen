//! JSON constrained generation using finite state machine validation
//!
//! This module implements sophisticated JSON constraint validation that ensures
//! generated tokens conform to valid JSON grammar using a stack-based state machine.

use super::GenerationConstraint;
use anyhow::{Context, Result as AnyResult};
use tokenizers::Tokenizer;

const MAX_DEPTH: usize = 32;

#[derive(Clone, Copy, PartialEq, Debug)]
enum JsonStackItem {
    Object,
    Array,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum NumberState {
    AfterSign,
    AfterZero,
    AfterIntDigit,
    AfterDot,
    AfterFracDigit,
    AfterE,
    AfterExpSign,
    AfterExpDigit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum JsonCurrentState {
    ExpectValue,
    ExpectObjectKey,
    ExpectColon,
    ExpectCommaOrObjectEnd,
    ExpectCommaOrArrayEnd,
    InString { escape: bool, is_key: bool },
    InNumber { state: NumberState },
    InTrue { pos: u8 },
    InFalse { pos: u8 },
    InNull { pos: u8 },
}

/// State tracker for JSON constraint validation during generation.
///
/// Maintains a stack of nested JSON structures (objects/arrays) and tracks
/// the current parsing state to ensure valid JSON syntax during token generation.
#[derive(Clone, Debug)]
pub struct JsonState {
    stack: [Option<JsonStackItem>; MAX_DEPTH],
    stack_len: usize,
    current: JsonCurrentState,
}

impl JsonState {
    /// Creates a new JSON state in the initial value-expecting state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for JsonState {
    fn default() -> Self {
        JsonState {
            stack: [None; MAX_DEPTH],
            stack_len: 0,
            current: JsonCurrentState::ExpectValue,
        }
    }
}

impl JsonState {
    fn push_stack(&mut self, item: JsonStackItem) -> AnyResult<()> {
        if self.stack_len >= MAX_DEPTH {
            return Err(anyhow::anyhow!("JSON depth exceeds maximum"));
        }
        self.stack[self.stack_len] = Some(item);
        self.stack_len += 1;
        Ok(())
    }

    fn pop_stack(&mut self) -> Option<JsonStackItem> {
        if self.stack_len == 0 {
            return None;
        }
        self.stack_len -= 1;
        self.stack[self.stack_len]
    }

    fn top_stack(&self) -> Option<JsonStackItem> {
        if self.stack_len == 0 {
            return None;
        }
        self.stack[self.stack_len - 1]
    }

    fn set_after_value(&mut self) {
        self.current = match self.top_stack() {
            Some(JsonStackItem::Object) => JsonCurrentState::ExpectCommaOrObjectEnd,
            Some(JsonStackItem::Array) => JsonCurrentState::ExpectCommaOrArrayEnd,
            None => JsonCurrentState::ExpectValue,
        };
    }

    fn is_end_char(b: u8) -> bool {
        matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}')
    }

    fn advance(&mut self, b: u8) -> AnyResult<()> {
        use JsonCurrentState as S;
        self.current = match self.current {
            S::ExpectValue => match b {
                b' ' | b'\t' | b'\n' | b'\r' => S::ExpectValue,
                b'{' => {
                    self.push_stack(JsonStackItem::Object)?;
                    S::ExpectObjectKey
                }
                b'[' => {
                    self.push_stack(JsonStackItem::Array)?;
                    S::ExpectValue
                }
                b'"' => S::InString {
                    escape: false,
                    is_key: false,
                },
                b't' => S::InTrue { pos: 1 },
                b'f' => S::InFalse { pos: 1 },
                b'n' => S::InNull { pos: 1 },
                b'-' => S::InNumber {
                    state: NumberState::AfterSign,
                },
                b'0' => S::InNumber {
                    state: NumberState::AfterZero,
                },
                b'1'..=b'9' => S::InNumber {
                    state: NumberState::AfterIntDigit,
                },
                _ => return Err(anyhow::anyhow!("Invalid value start: {}", b as char)),
            },
            S::ExpectObjectKey => match b {
                b' ' | b'\t' | b'\n' | b'\r' => S::ExpectObjectKey,
                b'"' => S::InString {
                    escape: false,
                    is_key: true,
                },
                b'}' => {
                    if self.pop_stack() != Some(JsonStackItem::Object) {
                        return Err(anyhow::anyhow!("Mismatched object close"));
                    }
                    match self.top_stack() {
                        Some(JsonStackItem::Object) => S::ExpectCommaOrObjectEnd,
                        Some(JsonStackItem::Array) => S::ExpectCommaOrArrayEnd,
                        None => S::ExpectValue,
                    }
                }
                _ => return Err(anyhow::anyhow!("Invalid key start: {}", b as char)),
            },
            S::ExpectColon => match b {
                b' ' | b'\t' | b'\n' | b'\r' => S::ExpectColon,
                b':' => S::ExpectValue,
                _ => return Err(anyhow::anyhow!("Expected colon, got: {}", b as char)),
            },
            S::ExpectCommaOrObjectEnd => match b {
                b' ' | b'\t' | b'\n' | b'\r' => S::ExpectCommaOrObjectEnd,
                b',' => S::ExpectObjectKey,
                b'}' => {
                    if self.pop_stack() != Some(JsonStackItem::Object) {
                        return Err(anyhow::anyhow!("Mismatched object close"));
                    }
                    match self.top_stack() {
                        Some(JsonStackItem::Object) => S::ExpectCommaOrObjectEnd,
                        Some(JsonStackItem::Array) => S::ExpectCommaOrArrayEnd,
                        None => S::ExpectValue,
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected comma or object end: {}",
                        b as char
                    ));
                }
            },
            S::ExpectCommaOrArrayEnd => match b {
                b' ' | b'\t' | b'\n' | b'\r' => S::ExpectCommaOrArrayEnd,
                b',' => S::ExpectValue,
                b']' => {
                    if self.pop_stack() != Some(JsonStackItem::Array) {
                        return Err(anyhow::anyhow!("Mismatched array close"));
                    }
                    match self.top_stack() {
                        Some(JsonStackItem::Object) => S::ExpectCommaOrObjectEnd,
                        Some(JsonStackItem::Array) => S::ExpectCommaOrArrayEnd,
                        None => S::ExpectValue,
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected comma or array end: {}",
                        b as char
                    ));
                }
            },
            S::InString { escape, is_key } => {
                if escape {
                    match b {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' | b'u' => {
                            S::InString {
                                escape: false,
                                is_key,
                            }
                        }
                        _ => return Err(anyhow::anyhow!("Invalid escape char: {}", b as char)),
                    }
                } else {
                    match b {
                        b'\\' => S::InString {
                            escape: true,
                            is_key,
                        },
                        b'"' => {
                            if is_key {
                                S::ExpectColon
                            } else {
                                self.set_after_value();
                                self.current
                            }
                        }
                        b if (32..=126).contains(&b) => S::InString {
                            escape: false,
                            is_key,
                        },
                        _ => return Err(anyhow::anyhow!("Invalid string char: {}", b as char)),
                    }
                }
            }
            S::InNumber { state } => match state {
                NumberState::AfterSign => match b {
                    b'0' => S::InNumber {
                        state: NumberState::AfterZero,
                    },
                    b'1'..=b'9' => S::InNumber {
                        state: NumberState::AfterIntDigit,
                    },
                    _ => return Err(anyhow::anyhow!("Expected digit after sign: {}", b as char)),
                },
                NumberState::AfterZero => match b {
                    b'0'..=b'9' => return Err(anyhow::anyhow!("No leading zeros")),
                    b'.' => S::InNumber {
                        state: NumberState::AfterDot,
                    },
                    b'e' | b'E' => S::InNumber {
                        state: NumberState::AfterE,
                    },
                    _ if Self::is_end_char(b) => {
                        self.set_after_value();
                        self.advance(b)?;
                        self.current
                    }
                    _ => return Err(anyhow::anyhow!("Invalid after zero: {}", b as char)),
                },
                NumberState::AfterIntDigit => match b {
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterIntDigit,
                    },
                    b'.' => S::InNumber {
                        state: NumberState::AfterDot,
                    },
                    b'e' | b'E' => S::InNumber {
                        state: NumberState::AfterE,
                    },
                    _ if Self::is_end_char(b) => {
                        self.set_after_value();
                        self.advance(b)?;
                        self.current
                    }
                    _ => return Err(anyhow::anyhow!("Invalid after int digit: {}", b as char)),
                },
                NumberState::AfterDot => match b {
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterFracDigit,
                    },
                    _ => return Err(anyhow::anyhow!("Expected digit after dot: {}", b as char)),
                },
                NumberState::AfterFracDigit => match b {
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterFracDigit,
                    },
                    b'e' | b'E' => S::InNumber {
                        state: NumberState::AfterE,
                    },
                    _ if Self::is_end_char(b) => {
                        self.set_after_value();
                        self.advance(b)?;
                        self.current
                    }
                    _ => return Err(anyhow::anyhow!("Invalid after frac digit: {}", b as char)),
                },
                NumberState::AfterE => match b {
                    b'+' | b'-' => S::InNumber {
                        state: NumberState::AfterExpSign,
                    },
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterExpDigit,
                    },
                    _ => return Err(anyhow::anyhow!("Expected exp sign or digit: {}", b as char)),
                },
                NumberState::AfterExpSign => match b {
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterExpDigit,
                    },
                    _ => return Err(anyhow::anyhow!("Expected exp digit: {}", b as char)),
                },
                NumberState::AfterExpDigit => match b {
                    b'0'..=b'9' => S::InNumber {
                        state: NumberState::AfterExpDigit,
                    },
                    _ if Self::is_end_char(b) => {
                        self.set_after_value();
                        self.advance(b)?;
                        self.current
                    }
                    _ => return Err(anyhow::anyhow!("Invalid after exp digit: {}", b as char)),
                },
            },
            S::InTrue { pos } => {
                let expected = b"true"[pos as usize];
                if b == expected {
                    if pos == 3 {
                        self.set_after_value();
                        self.current
                    } else {
                        S::InTrue { pos: pos + 1 }
                    }
                } else {
                    return Err(anyhow::anyhow!("Invalid 'true' sequence"));
                }
            }
            S::InFalse { pos } => {
                let expected = b"false"[pos as usize];
                if b == expected {
                    if pos == 4 {
                        self.set_after_value();
                        self.current
                    } else {
                        S::InFalse { pos: pos + 1 }
                    }
                } else {
                    return Err(anyhow::anyhow!("Invalid 'false' sequence"));
                }
            }
            S::InNull { pos } => {
                let expected = b"null"[pos as usize];
                if b == expected {
                    if pos == 3 {
                        self.set_after_value();
                        self.current
                    } else {
                        S::InNull { pos: pos + 1 }
                    }
                } else {
                    return Err(anyhow::anyhow!("Invalid 'null' sequence"));
                }
            }
        };
        Ok(())
    }
}

/// JSON constraint validator that ensures generated tokens form valid JSON.
///
/// Uses byte-level tokenizer analysis to allow/disallow tokens based on the
/// current JSON parsing state, ensuring syntactically correct JSON output.
#[derive(Debug, Clone)]
pub struct JsonConstraint<'a> {
    token_bytes: Vec<Vec<u8>>,
    tokens_per_start_byte: [Vec<u32>; 256],
    /// Phantom data to maintain lifetime parameter
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> JsonConstraint<'a> {
    /// Creates a new JSON constraint from a tokenizer vocabulary.
    ///
    /// # Errors
    ///
    /// Returns an error if tokenizer vocabulary cannot be processed
    pub fn new(tokenizer: &'a Tokenizer) -> AnyResult<Self> {
        let vocab_size = tokenizer.get_vocab_size(false);
        let mut token_bytes = vec![vec![]; vocab_size];
        for (i, bytes) in token_bytes.iter_mut().enumerate().take(vocab_size) {
            if let Some(s) = tokenizer.id_to_token(i as u32) {
                *bytes = s.into_bytes();
            }
        }
        let mut tokens_per_start_byte: [Vec<u32>; 256] = std::array::from_fn(|_| vec![]);
        for (i, bytes) in token_bytes.iter().enumerate().take(vocab_size) {
            if !bytes.is_empty() {
                let first = bytes[0] as usize;
                tokens_per_start_byte[first].push(i as u32);
            }
        }
        Ok(Self {
            token_bytes,
            tokens_per_start_byte,
            _phantom: std::marker::PhantomData,
        })
    }

    fn possible_next_bytes(&self, state: &JsonState) -> [bool; 256] {
        // Common JSON bytes - covers 99%+ of valid JSON characters
        static COMMON_JSON_BYTES: &[u8] = b"{}[]\":, \t\n\r\0abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-+eE\\/_";

        let mut poss = [false; 256];
        let mut found_valid = false;

        // First, check common JSON bytes (much faster than full range)
        for &byte_val in COMMON_JSON_BYTES {
            let mut s = state.clone();
            if s.advance(byte_val).is_ok() {
                poss[byte_val as usize] = true;
                found_valid = true;
            }
        }

        // Only check remaining bytes if no common bytes worked or for completeness in edge cases
        // This handles rare cases like extended Unicode escapes or other special cases
        if !found_valid {
            // Check remaining bytes not in common set
            for byte_val in 0u8..=255u8 {
                if !COMMON_JSON_BYTES.contains(&byte_val) {
                    let mut s = state.clone();
                    if s.advance(byte_val).is_ok() {
                        poss[byte_val as usize] = true;
                    }
                }
            }
        }

        poss
    }
}

impl<'a> GenerationConstraint for JsonConstraint<'a> {
    type State = JsonState;

    fn new_state(&self) -> Self::State {
        JsonState::new()
    }

    fn update(&self, state: &mut Self::State, token: u32) -> AnyResult<bool> {
        let bytes = &self.token_bytes[token as usize];
        for &b in bytes {
            state
                .advance(b)
                .context("Failed to advance state in update")?;
        }
        Ok(self.is_done(state))
    }

    fn try_next(&self, state: &Self::State, token: u32) -> AnyResult<bool> {
        let mut s = state.clone();
        let bytes = &self.token_bytes[token as usize];
        for &b in bytes {
            if s.advance(b).is_err() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn is_done(&self, state: &Self::State) -> bool {
        state.stack_len == 0 && matches!(state.current, JsonCurrentState::ExpectValue)
    }

    fn get_deterministic_sequence(&self, state: &Self::State) -> AnyResult<Vec<u32>> {
        let mut seq = Vec::new();
        let mut current = state.clone();
        loop {
            let poss_bytes = self.possible_next_bytes(&current);
            let mut count = 0;
            let mut the_token: Option<u32> = None;
            'outer: for (byte_idx, &possible) in poss_bytes.iter().enumerate() {
                if !possible {
                    continue;
                }
                for &t in &self.tokens_per_start_byte[byte_idx] {
                    let mut s = current.clone();
                    let bytes = &self.token_bytes[t as usize];
                    let mut valid = true;
                    for &b in bytes {
                        if s.advance(b).is_err() {
                            valid = false;
                            break;
                        }
                    }
                    if valid {
                        if count == 0 {
                            the_token = Some(t);
                        }
                        count += 1;
                        if count > 1 {
                            break 'outer;
                        }
                    }
                }
            }
            if count == 1 {
                let t = the_token.ok_or(anyhow::anyhow!("No token found despite count 1"))?;
                seq.push(t);
                let bytes = &self.token_bytes[t as usize];
                for &b in bytes {
                    current.advance(b)?;
                }
                if self.is_done(&current) {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(seq)
    }
}

// Helper functions for schema constraint integration
impl JsonConstraint<'_> {
    /// Update JSON state with token bytes (helper for schema constraint)
    pub fn update_json_state(state: &mut JsonState, token_bytes: &[u8]) -> anyhow::Result<()> {
        for &byte in token_bytes {
            state
                .advance(byte)
                .context("Failed to advance JSON state")?;
        }
        Ok(())
    }

    /// Check if JSON parsing is complete (helper for schema constraint)
    pub fn is_json_done(state: &JsonState) -> bool {
        state.stack_len == 0 && matches!(state.current, JsonCurrentState::ExpectValue)
    }

    /// Get deterministic sequence for JSON constraints (helper for schema constraint)
    pub fn get_json_deterministic_sequence(
        state: &JsonState,
        token_bytes: &[Vec<u8>],
        tokens_per_start_byte: &[Vec<u32>; 256],
    ) -> anyhow::Result<Vec<u32>> {
        let mut seq = Vec::new();
        let mut current = state.clone();

        loop {
            let poss_bytes = Self::get_possible_next_bytes(&current);
            let mut count = 0;
            let mut the_token: Option<u32> = None;

            'outer: for byte_idx in 0..256 {
                if !poss_bytes[byte_idx] {
                    continue;
                }
                for &token_id in &tokens_per_start_byte[byte_idx] {
                    let mut temp_state = current.clone();
                    let bytes = &token_bytes[token_id as usize];
                    let mut valid = true;

                    for &b in bytes {
                        if temp_state.advance(b).is_err() {
                            valid = false;
                            break;
                        }
                    }

                    if valid {
                        if count == 0 {
                            the_token = Some(token_id);
                        }
                        count += 1;
                        if count > 1 {
                            break 'outer;
                        }
                    }
                }
            }

            if count == 1 {
                let token_id =
                    the_token.ok_or_else(|| anyhow::anyhow!("No token found despite count 1"))?;
                seq.push(token_id);
                let bytes = &token_bytes[token_id as usize];
                for &b in bytes {
                    current.advance(b)?;
                }
                if Self::is_json_done(&current) {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(seq)
    }

    /// Get possible next bytes for current JSON state (helper function)
    fn get_possible_next_bytes(state: &JsonState) -> [bool; 256] {
        let mut possible = [false; 256];
        for b in 0u8..=255 {
            let mut temp_state = state.clone();
            if temp_state.advance(b).is_ok() {
                possible[b as usize] = true;
            }
        }
        possible
    }
}
