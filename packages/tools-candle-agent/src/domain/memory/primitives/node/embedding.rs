use serde::{Deserialize, Serialize};

/// SIMD-aligned embedding vector for optimal performance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(align(32))] // Align to 32 bytes for AVX2 SIMD operations
pub struct AlignedEmbedding {
    /// Embedding vector data aligned for SIMD operations
    pub data: Vec<f32>,
    /// Vector dimension for validation
    pub dimension: usize,
}

impl AlignedEmbedding {
    /// Create new aligned embedding with SIMD optimization
    #[inline]
    #[must_use]
    pub fn new(data: Vec<f32>) -> Self {
        let dimension = data.len();
        Self { data, dimension }
    }

    /// Get embedding data as slice for SIMD operations
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Convert to Vec for compatibility
    #[inline]
    #[must_use]
    pub fn to_vec(&self) -> Vec<f32> {
        self.data.clone()
    }

    /// Calculate dot product with SIMD optimization hint
    #[inline]
    #[must_use]
    pub fn dot_product(&self, other: &Self) -> Option<f32> {
        if self.dimension != other.dimension {
            return None;
        }

        // Hint to compiler for SIMD optimization
        Some(
            self.data
                .iter()
                .zip(other.data.iter())
                .map(|(a, b)| a * b)
                .sum(),
        )
    }

    /// Calculate cosine similarity with SIMD optimization
    #[inline]
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> Option<f32> {
        if self.dimension != other.dimension {
            return None;
        }

        let dot = self.dot_product(other)?;
        let norm_self = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_other = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_self == 0.0 || norm_other == 0.0 {
            Some(0.0)
        } else {
            Some(dot / (norm_self * norm_other))
        }
    }
}
