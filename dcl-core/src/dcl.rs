// dcl-core/src/dcl.rs
//! DCL sequence engine and conservation verifier.
//! Implements the core Dynamic Coprime Labeling sequence (f_t)_{t>=0}.

use crate::error::{DclError, DclResult};
use crate::graph::Graph;
use crate::labeling::Labeling;
use crate::transform::Transform;

/// A single step of the DCL sequence.
/// Holds the current labeling and verifies coprimality on every edge.
pub struct DclSequence<'a> {
    pub graph: &'a Graph,
    pub transform: &'a dyn Transform,
    pub current: Labeling,
    pub step: usize,
}

impl<'a> DclSequence<'a> {
    /// Initialize DCL from an initial labeling f_0 with validation.
    /// Returns an error if the initial labeling is not coprime on all edges.
    pub fn try_new(graph: &'a Graph, transform: &'a dyn Transform, f0: Labeling) -> DclResult<Self> {
        // Check dimension match
        if f0.len() != graph.n {
            return Err(DclError::dimension_mismatch(graph.n, f0.len()));
        }

        // Check coprimality on all edges
        if !graph.is_coprime_labeling(&f0) {
            // Find which edge violates coprimality
            for &(u, v) in &graph.edges() {
                if !f0.edge_coprime(u, v) {
                    return Err(DclError::coprimality_violation(u, v, f0.get(u), f0.get(v)));
                }
            }
            return Err(DclError::invalid_labeling(
                "Initial labeling f_0 must be a valid coprime labeling".to_string()
            ));
        }

        Ok(DclSequence {
            graph,
            transform,
            current: f0,
            step: 0,
        })
    }

    /// Initialize DCL from an initial labeling f_0.
    /// Panics if the initial labeling is not coprime on all edges.
    ///
    /// For error handling use `try_new()` instead.
    pub fn new(graph: &'a Graph, transform: &'a dyn Transform, f0: Labeling) -> Self {
        Self::try_new(graph, transform, f0).expect("Initial labeling must be valid")
    }

    /// Advance one time step: f_{t+1} = g ∘ f_t.
    pub fn advance(&mut self) {
        self.current = self.current.evolve(self.transform);
        self.step += 1;
    }

    /// Check that the current labeling satisfies the coprime constraint on all edges.
    pub fn is_valid(&self) -> bool {
        self.graph.is_coprime_labeling(&self.current)
    }

    /// Run for `steps` iterations and verify coprimality is maintained at every step.
    /// Returns Ok(()) if all steps are valid, Err with detailed error if any step fails.
    pub fn verify_steps(&mut self, steps: usize) -> DclResult<()> {
        for _ in 0..steps {
            self.advance();
            if !self.is_valid() {
                // Find which edge violated coprimality
                for &(u, v) in &self.graph.edges() {
                    if !self.current.edge_coprime(u, v) {
                        return Err(DclError::coprimality_violation(
                            u, v,
                            self.current.get(u),
                            self.current.get(v)
                        ));
                    }
                }
                return Err(DclError::ComputationError(format!(
                    "Coprimality constraint violated at step {}",
                    self.step
                )));
            }
        }
        Ok(())
    }
}

/// Verify the Isomorphic Conservation Theorem (Computational_DCL Theorem 3.1):
/// For a power map g(x) = x^m, G_t ≅ G_0 for all t.
/// We verify this empirically by checking the edge set is preserved (same coprimality pattern).
///
/// Returns true if coprimality structure is conserved for all `steps` iterations.
pub fn verify_conservation(
    graph: &Graph,
    transform: &dyn Transform,
    f0: Labeling,
    steps: usize,
) -> bool {
    // Compute the reference edge set: which edges are coprime in f_0
    let reference: Vec<bool> = graph
        .edges()
        .iter()
        .map(|&(u, v)| f0.edge_coprime(u, v))
        .collect();

    let mut seq = DclSequence::new(graph, transform, f0);
    for _ in 0..steps {
        seq.advance();
        let current_pattern: Vec<bool> = graph
            .edges()
            .iter()
            .map(|&(u, v)| seq.current.edge_coprime(u, v))
            .collect();
        if current_pattern != reference {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::labeling::Labeling;
    use crate::transform::{IdentityMap, PowerMap};

    fn coprime_path_labels(n: usize) -> Labeling {
        // Use distinct primes — always coprime pairwise on a path
        let primes: Vec<u64> = [1, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]
            .iter()
            .copied()
            .take(n)
            .collect();
        Labeling::new(primes)
    }

    /// Identity transform: trivially preserves coprimality for all steps.
    #[test]
    fn dcl_path_p5_identity_100_steps() {
        let g = Graph::path(5);
        let f0 = coprime_path_labels(5);
        let id = IdentityMap;
        let mut seq = DclSequence::new(&g, &id, f0);
        assert!(
            seq.verify_steps(100).is_ok(),
            "DCL identity on P_5 failed at step {}",
            seq.step
        );
    }

    #[test]
    fn dcl_cycle_c6_identity() {
        let g = Graph::cycle(6);
        let id = IdentityMap;
        let f0 = Labeling::new(vec![1, 2, 3, 5, 7, 11]);
        assert!(g.is_coprime_labeling(&f0));
        let mut seq = DclSequence::new(&g, &id, f0);
        assert!(
            seq.verify_steps(50).is_ok(),
            "DCL identity on C_6 failed at step {}",
            seq.step
        );
    }

    #[test]
    fn dcl_wheel_w5_identity() {
        let g = Graph::wheel(5);
        let id = IdentityMap;
        // Hub=1 is coprime to everything; rim: 2,3,5,7 (primes)
        let f0 = Labeling::new(vec![1, 2, 3, 5, 7]);
        assert!(g.is_coprime_labeling(&f0));
        let mut seq = DclSequence::new(&g, &id, f0);
        assert!(
            seq.verify_steps(50).is_ok(),
            "DCL identity on W_5 failed at step {}",
            seq.step
        );
    }

    #[test]
    fn dcl_hypercube_q3_identity() {
        let g = Graph::hypercube(3); // 8 vertices
        let id = IdentityMap;
        let f0 = Labeling::new(vec![1, 2, 3, 5, 7, 11, 13, 17]);
        assert!(g.is_coprime_labeling(&f0));
        let mut seq = DclSequence::new(&g, &id, f0);
        assert!(
            seq.verify_steps(50).is_ok(),
            "DCL identity on Q_3 failed at step {}",
            seq.step
        );
    }

    /// Power map preserves coprimality for ONE step (integers, no overflow).
    /// Tests the core theorem: gcd(a,b)=1 ⟹ gcd(a^m,b^m)=1.
    #[test]
    fn dcl_power_map_one_step() {
        let g = Graph::path(4);
        // Use labels deliberately chosen to not overflow u64 after one squaring
        // and to remain coprime: all distinct primes < 100
        let f0 = Labeling::new(vec![2, 3, 5, 7]);
        let pm = PowerMap::new(2);
        assert!(g.is_coprime_labeling(&f0));
        let mut seq = DclSequence::new(&g, &pm, f0);
        // After one squaring: [4, 9, 25, 49]
        // gcd(4,9)=1, gcd(9,25)=1, gcd(25,49)=1 — all coprime
        assert!(seq.verify_steps(1).is_ok(), "Power map step 1 failed");
    }

    #[test]
    fn isomorphic_conservation_path_identity() {
        let g = Graph::path(5);
        let id = IdentityMap;
        let f0 = Labeling::new(vec![1, 2, 3, 5, 7]);
        // Identity map never changes labels ⟹ trivially conserved
        assert!(verify_conservation(&g, &id, f0, 50));
    }
}
