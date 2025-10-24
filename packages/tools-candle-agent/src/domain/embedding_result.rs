//! Embedding result type for builder pattern
//!
//! Simple domain type representing the result of an embedding operation.

use crate::domain::collections::ZeroOneOrMany;

/// Embedding result containing document text and computed vector
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The original document text that was embedded
    pub document: String,
    
    /// The computed embedding vector(s)
    /// Can be None, single vector, or multiple vectors
    pub vec: ZeroOneOrMany<f32>,
}

impl Embedding {
    /// Create a new embedding result
    pub fn new(document: String, vec: Vec<f32>) -> Self {
        Self {
            document,
            vec: ZeroOneOrMany::One(vec),
        }
    }
    
    /// Create embedding with no vector (placeholder)
    pub fn placeholder(document: String) -> Self {
        Self {
            document,
            vec: ZeroOneOrMany::None,
        }
    }
    
    /// Get the embedding vector as a slice, if available
    pub fn as_vec(&self) -> Option<&Vec<f32>> {
        match &self.vec {
            ZeroOneOrMany::One(v) => Some(v),
            _ => None,
        }
    }
    
    /// Get the embedding vector as a slice (convenience method)
    pub fn as_slice(&self) -> &[f32] {
        match &self.vec {
            ZeroOneOrMany::One(v) => v.as_slice(),
            _ => &[],
        }
    }
    
    /// Get the dimensionality of the embedding vector
    pub fn dimensions(&self) -> usize {
        match &self.vec {
            ZeroOneOrMany::One(v) => v.len(),
            _ => 0,
        }
    }
}
