//! Penalty application for logits processing

use smallvec::SmallVec;

use crate::config::ProcessorConfig;
use crate::context::ProcessingContext;
use crate::logits::LogitsResult;

/// Apply penalties to logits
pub fn apply_penalties_simd(
    logits: &mut [f32],
    context: &ProcessingContext,
    config: &ProcessorConfig,
) -> LogitsResult<()> {
    if logits.is_empty() {
        return Ok(());
    }

    // Apply repetition penalty if needed
    if config.repetition_penalty != 1.0 && !context.token_history.is_empty() {
        apply_repetition_penalty(logits, &context.token_history, config.repetition_penalty)?;
    }

    // Apply frequency penalty if needed
    if config.frequency_penalty != 0.0 && !context.token_history.is_empty() {
        apply_frequency_penalty(logits, &context.token_history, config.frequency_penalty)?;
    }

    // Apply presence penalty if needed
    if config.presence_penalty != 0.0 && !context.token_history.is_empty() {
        apply_presence_penalty(logits, &context.token_history, config.presence_penalty)?;
    }

    Ok(())
}

/// Apply repetition penalty to logits
fn apply_repetition_penalty(
    logits: &mut [f32],
    token_history: &[u32],
    penalty: f32,
) -> LogitsResult<()> {
    let mut seen = std::collections::HashSet::new();
    for &token in token_history {
        if (token as usize) < logits.len() && seen.insert(token) {
            if logits[token as usize] > 0.0 {
                logits[token as usize] /= penalty;
            } else {
                logits[token as usize] *= penalty;
            }
        }
    }
    Ok(())
}

/// Apply frequency penalty to logits
fn apply_frequency_penalty(
    logits: &mut [f32],
    token_history: &[u32],
    penalty: f32,
) -> LogitsResult<()> {
    // Count token frequencies
    let mut freqs = SmallVec::<u32, 512>::from_elem(0, logits.len().min(512));
    for &token in token_history {
        if (token as usize) < freqs.len() {
            freqs[token as usize] += 1;
        }
    }

    // Apply penalty based on frequency
    for (i, &freq) in freqs.iter().enumerate() {
        if i < logits.len() && freq > 0 {
            logits[i] -= penalty * freq as f32;
        }
    }

    Ok(())
}

/// Apply presence penalty to logits
fn apply_presence_penalty(
    logits: &mut [f32],
    token_history: &[u32],
    penalty: f32,
) -> LogitsResult<()> {
    // Use a set to track unique tokens
    let mut seen = std::collections::HashSet::new();

    // Apply penalty for each unique token in history
    for &token in token_history {
        if (token as usize) < logits.len() && seen.insert(token) {
            logits[token as usize] -= penalty;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;

    #[test]
    fn test_repetition_penalty() {
        let mut logits = vec![1.0, 2.0, 3.0];
        let history = vec![0, 2, 0]; // Test with duplicate
        if let Err(e) = apply_repetition_penalty(&mut logits, &history, 2.0) {
            panic!("Repetition penalty failed: {}", e);
        }

        // First and third elements should be penalized once each
        assert_float_eq!(logits[0], 0.5, abs <= 1e-6); // 1.0 / 2.0
        assert_float_eq!(logits[1], 2.0, abs <= 1e-6); // No penalty
        assert_float_eq!(logits[2], 1.5, abs <= 1e-6); // 3.0 / 2.0
    }

    #[test]
    fn test_frequency_penalty() {
        let mut logits = vec![1.0, 2.0, 3.0];
        let history = vec![0, 0, 1]; // Token 0 appears twice, token 1 once
        if let Err(e) = apply_frequency_penalty(&mut logits, &history, 0.5) {
            panic!("Frequency penalty failed: {}", e);
        }

        // Penalty is frequency * penalty
        assert_float_eq!(logits[0], 0.0, abs <= 1e-6); // 1.0 - (2 * 0.5)
        assert_float_eq!(logits[1], 1.5, abs <= 1e-6); // 2.0 - (1 * 0.5)
        assert_float_eq!(logits[2], 3.0, abs <= 1e-6); // No penalty
    }

    #[test]
    fn test_presence_penalty() {
        let mut logits = vec![1.0, 2.0, 3.0];
        let history = vec![0, 0, 1]; // Tokens 0 and 1 (unique)
        if let Err(e) = apply_presence_penalty(&mut logits, &history, 0.5) {
            panic!("Presence penalty failed: {}", e);
        }

        // Each unique token gets penalty applied once
        assert_float_eq!(logits[0], 0.5, abs <= 1e-6); // 1.0 - 0.5
        assert_float_eq!(logits[1], 1.5, abs <= 1e-6); // 2.0 - 0.5
        assert_float_eq!(logits[2], 3.0, abs <= 1e-6); // No penalty
    }
}
