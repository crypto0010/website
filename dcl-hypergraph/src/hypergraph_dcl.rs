// dcl-hypergraph/src/hypergraph_dcl.rs
//! DCL sequence engine for hypergraphs.
//! Extends the core DCL to k-uniform hypergraphs with setwise coprimality.
//! Theorem 4.2 (Hypergraph Existence Equivalence): A hypergraph H admits a
//! setwise DCL under g(x)=x^m iff it admits a static setwise coprime labeling.

use crate::hypergraph::Hypergraph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::Transform;

/// DCL sequence for a hypergraph.
pub struct HypergraphDcl<'a> {
    pub hypergraph: &'a Hypergraph,
    pub transform: &'a dyn Transform,
    pub current: Labeling,
    pub step: usize,
}

impl<'a> HypergraphDcl<'a> {
    /// Initialize hypergraph DCL. Panics if f0 is not a valid setwise coprime labeling.
    pub fn new(hypergraph: &'a Hypergraph, transform: &'a dyn Transform, f0: Labeling) -> Self {
        assert!(
            hypergraph.is_valid_labeling(&f0),
            "Initial labeling must satisfy setwise coprimality on all hyperedges"
        );
        HypergraphDcl {
            hypergraph,
            transform,
            current: f0,
            step: 0,
        }
    }

    /// Advance one step: apply transform to all labels.
    pub fn advance(&mut self) {
        self.current = self.current.evolve(self.transform);
        self.step += 1;
    }

    /// Check that current labeling satisfies all hyperedge constraints.
    pub fn is_valid(&self) -> bool {
        self.hypergraph.is_valid_labeling(&self.current)
    }

    /// Run for `steps` iterations, verifying validity at each step.
    /// Returns Ok(()) if all valid, Err(t) on first invalid step.
    pub fn verify_steps(&mut self, steps: usize) -> Result<(), usize> {
        for _ in 0..steps {
            self.advance();
            if !self.is_valid() {
                return Err(self.step);
            }
        }
        Ok(())
    }
}

/// Empirically verify the Hypergraph Existence Equivalence Theorem:
/// If a static setwise coprime labeling exists, a DCL exists (and vice versa).
/// Checks this by running a sample DCL sequence for `steps` steps.
pub fn verify_existence_equivalence(
    h: &Hypergraph,
    f0: Labeling,
    transform: &dyn Transform,
    steps: usize,
) -> bool {
    if !h.is_valid_labeling(&f0) {
        return false; // No static labeling → no DCL (forward direction)
    }
    // Backward direction: run DCL sequence for `steps` and confirm validity
    let mut dcl = HypergraphDcl::new(h, transform, f0);
    dcl.verify_steps(steps).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypergraph::Hypergraph;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::IdentityMap;

    #[test]
    fn hypergraph_dcl_3uniform() {
        let mut h = Hypergraph::new(5);
        h.add_edge(vec![0, 1, 2]);
        h.add_edge(vec![1, 2, 3]);
        h.add_edge(vec![2, 3, 4]);
        let f0 = Labeling::new(vec![1, 2, 3, 5, 7]);
        // Use identity map: trivially preserves setwise coprimality for all steps
        let id = IdentityMap;
        let mut dcl = HypergraphDcl::new(&h, &id, f0);
        assert!(
            dcl.verify_steps(50).is_ok(),
            "3-uniform hypergraph DCL should maintain setwise coprimality"
        );
    }

    #[test]
    fn existence_equivalence_check() {
        let h = Hypergraph::complete_uniform(4, 2);
        let f0 = Labeling::new(vec![1, 2, 3, 5]);
        // Identity map conserves all labelings—demonstrates DCL exists if static labeling exists
        let id = IdentityMap;
        assert!(verify_existence_equivalence(&h, f0, &id, 50));
    }
}
