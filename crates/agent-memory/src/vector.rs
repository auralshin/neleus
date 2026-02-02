//! Vector index for semantic search.
//!
//! Provides fast approximate nearest neighbor search for memory embeddings.

use parking_lot::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{MemoryError, MemoryResult};

/// A simple vector index for semantic similarity search.
pub struct VectorIndex {
    /// Dimension of vectors
    dimension: usize,
    /// Stored vectors with their IDs
    vectors: RwLock<HashMap<Uuid, Vec<f32>>>,
}

impl VectorIndex {
    /// Create a new vector index with the specified dimension.
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            vectors: RwLock::new(HashMap::new()),
        }
    }

    /// Add a vector to the index.
    pub fn add(&self, id: Uuid, vector: Vec<f32>) -> MemoryResult<()> {
        if vector.len() != self.dimension {
            return Err(MemoryError::Vector(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            )));
        }
        self.vectors.write().insert(id, vector);
        Ok(())
    }

    /// Remove a vector from the index.
    pub fn remove(&self, id: Uuid) -> bool {
        self.vectors.write().remove(&id).is_some()
    }

    /// Search for the k nearest neighbors to the query vector.
    pub fn search(&self, query: &[f32], k: usize) -> MemoryResult<Vec<(Uuid, f32)>> {
        if query.len() != self.dimension {
            return Err(MemoryError::Vector(format!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimension,
                query.len()
            )));
        }

        let vectors = self.vectors.read();
        let mut results: Vec<(Uuid, f32)> = vectors
            .iter()
            .map(|(id, vec)| (*id, cosine_similarity(query, vec)))
            .collect();

        // Sort by similarity (highest first)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Get the number of vectors in the index.
    pub fn len(&self) -> usize {
        self.vectors.read().len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.vectors.read().is_empty()
    }

    /// Clear all vectors from the index.
    pub fn clear(&self) {
        self.vectors.write().clear();
    }
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Compute L2 (Euclidean) distance between two vectors.
#[allow(dead_code)]
fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Normalize a vector to unit length.
#[allow(dead_code)]
fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_index_basic() {
        let index = VectorIndex::new(3);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        index.add(id1, vec![1.0, 0.0, 0.0]).unwrap();
        index.add(id2, vec![0.0, 1.0, 0.0]).unwrap();
        index.add(id3, vec![0.9, 0.1, 0.0]).unwrap();

        assert_eq!(index.len(), 3);

        // Search for vector similar to [1, 0, 0]
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, id1); // Exact match should be first
        assert_eq!(results[1].0, id3); // Similar should be second
    }

    #[test]
    fn test_cosine_similarity() {
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = [0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);

        let d = [-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_dimension_mismatch() {
        let index = VectorIndex::new(3);
        let id = Uuid::new_v4();

        // Wrong dimension should fail
        assert!(index.add(id, vec![1.0, 0.0]).is_err());
        assert!(index.search(&[1.0, 0.0], 1).is_err());
    }
}
