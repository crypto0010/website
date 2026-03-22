// dcl-core/src/labeling.rs
//! Injective vertex labelings: map vertex index -> positive integer label.

use crate::error::{DclError, DclResult};
use crate::gcd::are_coprime;
use crate::graph::Graph;
use crate::transform::Transform;

/// An injective labeling: assigns a unique positive integer to each vertex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labeling {
    /// labels[i] = label assigned to vertex i. All values > 0.
    pub labels: Vec<u64>,
}

impl Labeling {
    /// Create a new labeling from a vector of label values.
    /// Returns an error if any label is 0 or non-positive.
    pub fn try_new(labels: Vec<u64>) -> DclResult<Self> {
        if let Some((i, _)) = labels.iter().enumerate().find(|(_, &x)| x == 0) {
            return Err(DclError::invalid_labeling(format!(
                "Label at index {} is zero (all labels must be positive)",
                i
            )));
        }
        Ok(Labeling { labels })
    }

    /// Create a new labeling from a vector of label values.
    /// Panics if any label is 0.
    ///
    /// For error handling use `try_new()` instead.
    pub fn new(labels: Vec<u64>) -> Self {
        Self::try_new(labels).expect("All labels must be positive")
    }

    /// Number of vertices.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Get the label for vertex v.
    #[inline]
    pub fn get(&self, v: usize) -> u64 {
        self.labels[v]
    }

    /// Set the label for vertex v with bounds checking.
    /// Returns an error if val is 0 or index is out of bounds.
    pub fn try_set(&mut self, v: usize, val: u64) -> DclResult<()> {
        if val == 0 {
            return Err(DclError::invalid_labeling("Label must be positive"));
        }
        if v >= self.labels.len() {
            return Err(DclError::index_out_of_bounds(v, self.labels.len()));
        }
        self.labels[v] = val;
        Ok(())
    }

    /// Set the label for vertex v.
    /// Panics if val is 0 or index is out of bounds.
    ///
    /// For error handling use `try_set()` instead.
    #[inline]
    pub fn set(&mut self, v: usize, val: u64) {
        self.try_set(v, val).expect("Invalid label or index")
    }

    /// Check that all labels are distinct (injective condition).
    pub fn is_injective(&self) -> bool {
        let mut seen = std::collections::HashSet::new();
        self.labels.iter().all(|x| seen.insert(x))
    }

    /// Check that a specific edge (u, v) satisfies the coprimality constraint.
    pub fn edge_coprime(&self, u: usize, v: usize) -> bool {
        are_coprime(self.labels[u], self.labels[v])
    }

    /// Apply a transform to produce the next labeling in the DCL sequence.
    /// f_{t+1}(v) = g(f_t(v)) for all v.
    pub fn evolve(&self, transform: &dyn Transform) -> Labeling {
        Labeling {
            labels: self.labels.iter().map(|&x| transform.apply(x)).collect(),
        }
    }

    /// Apply a transform in-place, modifying labels without allocation.
    /// f_{t+1}(v) = g(f_t(v)) for all v.
    pub fn evolve_in_place(&mut self, transform: &dyn Transform) {
        for label in &mut self.labels {
            *label = transform.apply(*label);
        }
    }

    /// Repair coprimality violations by incrementing labels on edges where gcd != 1.
    /// Returns the number of repairs performed.
    /// After repair, the labeling satisfies coprimality for all edges in `graph`.
    pub fn repair_coprimality(&mut self, graph: &Graph) -> usize {
        let mut repairs = 0;
        loop {
            let mut found_violation = false;
            for (u, v) in graph.edges() {
                while !are_coprime(self.labels[u], self.labels[v]) {
                    // Increment the larger label
                    if self.labels[u] >= self.labels[v] {
                        self.labels[u] += 1;
                    } else {
                        self.labels[v] += 1;
                    }
                    repairs += 1;
                    found_violation = true;
                }
            }
            if !found_violation {
                break;
            }
        }
        repairs
    }

    /// Maximum label value.
    pub fn max_label(&self) -> u64 {
        self.labels.iter().copied().max().unwrap_or(0)
    }
}

impl std::fmt::Display for Labeling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, &l) in self.labels.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "v{i}={l}")?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::PowerMap;

    #[test]
    fn labeling_injective() {
        let l = Labeling::new(vec![2, 3, 5, 7]);
        assert!(l.is_injective());
        let l2 = Labeling::new(vec![2, 3, 2, 7]);
        assert!(!l2.is_injective());
    }

    #[test]
    fn labeling_evolve() {
        let l = Labeling::new(vec![2, 3, 5]);
        let pm = PowerMap::new(2);
        let l2 = l.evolve(&pm);
        assert_eq!(l2.labels, vec![4, 9, 25]);
    }

    #[test]
    fn edge_coprime() {
        let l = Labeling::new(vec![2, 3, 4, 9]);
        assert!(l.edge_coprime(0, 1)); // gcd(2,3) = 1 ✓
        assert!(l.edge_coprime(2, 3)); // gcd(4,9) = 1 ✓ (4=2², 9=3², disjoint primes)
                                       // Non-coprime case: 6 and 9 share factor 3
        let l2 = Labeling::new(vec![6, 9, 5, 7]);
        assert!(!l2.edge_coprime(0, 1)); // gcd(6,9) = 3 ≠ 1
    }

    #[test]
    fn evolve_in_place_matches_evolve() {
        let original = Labeling::new(vec![2, 3, 5, 7]);
        let pm = PowerMap::new(2);

        let evolved = original.evolve(&pm);

        let mut in_place = original.clone();
        in_place.evolve_in_place(&pm);

        assert_eq!(evolved.labels, in_place.labels, "evolve and evolve_in_place must produce identical results");
        assert_eq!(in_place.labels, vec![4, 9, 25, 49]);
    }

    #[test]
    fn repair_coprimality_fixes_violations() {
        use crate::graph::Graph;

        let g = Graph::path(4);
        // Labels [6, 9, 5, 7]: gcd(6,9)=3, edge (0,1) violates coprimality
        let mut labeling = Labeling::new(vec![6, 9, 5, 7]);
        assert!(!g.is_coprime_labeling(&labeling));

        let repairs = labeling.repair_coprimality(&g);
        assert!(repairs > 0, "Should have made repairs");
        assert!(g.is_coprime_labeling(&labeling), "Should be coprime after repair");
        assert!(labeling.labels.iter().all(|&l| l >= 1));
    }

    #[test]
    fn repair_coprimality_no_op_on_valid() {
        use crate::graph::Graph;

        let g = Graph::path(3);
        let mut labeling = Labeling::new(vec![2, 3, 5]);
        assert!(g.is_coprime_labeling(&labeling));

        let repairs = labeling.repair_coprimality(&g);
        assert_eq!(repairs, 0, "No repairs needed for valid labeling");
        assert_eq!(labeling.labels, vec![2, 3, 5], "Labels should be unchanged");
    }
}
