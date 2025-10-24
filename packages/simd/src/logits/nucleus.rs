//! Nucleus (top-p) sampling implementation

use rand::Rng;
use smallvec::SmallVec;

use crate::logits::LogitsResult;

/// Prepare logits for nucleus sampling
pub fn prepare_nucleus_sampling_simd(logits: &mut [f32], top_p: f64) -> LogitsResult<()> {
    if top_p <= 0.0 || top_p > 1.0 {
        return Err(crate::logits::LogitsError::SamplingError(
            "top_p must be in (0, 1]".to_string(),
        ));
    }

    if logits.is_empty() {
        return Ok(());
    }

    // Find max logit for numerical stability (zero allocation)
    let max_logit = logits.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));

    // Create SmallVec of (index, logit) pairs
    let mut sorted: SmallVec<(usize, f32), 512> =
        logits.iter().enumerate().map(|(i, &v)| (i, v)).collect();

    // Sort in descending order by logit value
    sorted.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Compute total sum of exp(shifted) without allocation
    let mut total_sum = 0.0f64;
    for &logit in logits.iter() {
        total_sum += ((logit - max_logit) as f64).exp();
    }

    // Now find cutoff using cumulative normalized prob
    let mut cumsum = 0.0f64;
    let mut cutoff = sorted.len();

    for (i, &(_, logit)) in sorted.iter().enumerate() {
        let prob = ((logit - max_logit) as f64).exp() / total_sum;
        cumsum += prob;
        if cumsum >= top_p {
            cutoff = i + 1;
            break;
        }
    }

    // Collect indices to keep (using SmallVec to avoid alloc if small)
    let keep_indices: SmallVec<usize, 512> = sorted[..cutoff].iter().map(|&(idx, _)| idx).collect();

    // Sort keep_indices for potential binary search if needed, but here we use loop
    // Mask logits not in keep (in-place, zero alloc)
    for (i, logit) in logits.iter_mut().enumerate() {
        if !keep_indices.contains(&i) {
            *logit = f32::NEG_INFINITY;
        }
    }

    Ok(())
}

/// Sample from nucleus
pub fn sample_from_nucleus<R: Rng>(probs: &[f32], rng: &mut R) -> LogitsResult<usize> {
    let sum: f32 = probs.iter().sum();
    let threshold = rng.random_range(0.0..sum);

    let mut cumsum = 0.0f32;
    for (i, &p) in probs.iter().enumerate() {
        cumsum += p;
        if cumsum >= threshold {
            return Ok(i);
        }
    }

    Ok(probs.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nucleus_sampling() {
        let mut logits = vec![0.1, 0.2, 0.3, 0.4];

        if let Err(e) = prepare_nucleus_sampling_simd(&mut logits, 0.5) {
            panic!(
                "Nucleus sampling preparation should succeed with valid inputs: {}",
                e
            );
        }

        // Verify the correct top elements are kept (indices 2,3 masked 0,1 for top_p=0.5)
        assert_eq!(
            logits[0],
            f32::NEG_INFINITY,
            "Expected index 0 to be set to -inf"
        );
        assert_eq!(
            logits[1],
            f32::NEG_INFINITY,
            "Expected index 1 to be set to -inf"
        );
        assert!(logits[2] > f32::NEG_INFINITY, "Expected index 2 to be kept");
        assert!(logits[3] > f32::NEG_INFINITY, "Expected index 3 to be kept");
    }

    #[test]
    fn test_sample_from_nucleus() {
        let probs = vec![0.1, 0.2, 0.3, 0.4];
        let mut rng = rand::rng();
        let idx = match sample_from_nucleus(&probs, &mut rng) {
            Ok(i) => i,
            Err(e) => panic!(
                "Sampling from nucleus should succeed with valid probabilities: {}",
                e
            ),
        };
        assert!(idx < probs.len());
    }
}
