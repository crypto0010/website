// dcl-hypergraph/src/hypergraph.rs
//! k-uniform hypergraph definition and constructors.

use dcl_core::gcd::setwise_gcd;
use dcl_core::labeling::Labeling;

/// A hypergraph H = (V, E) where each hyperedge is a set of vertices.
#[derive(Debug, Clone)]
pub struct Hypergraph {
    pub n: usize,
    pub edges: Vec<Vec<usize>>,
}

impl Hypergraph {
    pub fn new(n: usize) -> Self {
        Hypergraph {
            n,
            edges: Vec::new(),
        }
    }

    pub fn add_edge(&mut self, edge: Vec<usize>) {
        assert!(edge.iter().all(|&v| v < self.n), "Vertex out of range");
        assert!(edge.len() >= 2, "Hyperedge must have at least 2 vertices");
        self.edges.push(edge);
    }

    /// Check that the labeling satisfies setwise coprimality on all hyperedges:
    /// gcd(x(v1), ..., x(vk)) = 1 for each edge.
    pub fn is_valid_labeling(&self, lab: &Labeling) -> bool {
        self.edges.iter().all(|edge| {
            let vals: Vec<u64> = edge.iter().map(|&v| lab.get(v)).collect();
            setwise_gcd(&vals) == 1
        })
    }

    /// Return the uniformity k if all edges have the same size, else None.
    pub fn uniformity(&self) -> Option<usize> {
        let first = self.edges.first().map(|e| e.len())?;
        if self.edges.iter().all(|e| e.len() == first) {
            Some(first)
        } else {
            None
        }
    }

    /// Build a complete k-uniform hypergraph on n vertices:
    /// Every k-subset of V is a hyperedge.
    pub fn complete_uniform(n: usize, k: usize) -> Self {
        assert!(k >= 2 && k <= n);
        let mut h = Hypergraph::new(n);
        // Generate all k-subsets
        for edge in k_subsets(n, k) {
            h.edges.push(edge);
        }
        h
    }
}

/// Enumerate all k-element subsets of {0, ..., n-1}.
fn k_subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut combo = Vec::new();
    k_subsets_helper(0, n, k, &mut combo, &mut result);
    result
}

fn k_subsets_helper(
    start: usize,
    n: usize,
    k: usize,
    combo: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    if combo.len() == k {
        result.push(combo.clone());
        return;
    }
    for i in start..n {
        combo.push(i);
        k_subsets_helper(i + 1, n, k, combo, result);
        combo.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_3uniform_validity() {
        let h = Hypergraph::complete_uniform(5, 3);
        // C(5,3) = 10 edges
        assert_eq!(h.edges.len(), 10);
        let lab = Labeling::new(vec![1, 2, 3, 5, 7]);
        assert!(h.is_valid_labeling(&lab));
    }
}
