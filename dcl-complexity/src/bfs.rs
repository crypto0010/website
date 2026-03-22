// dcl-complexity/src/bfs.rs
//! DCL-BFS: breadth-first search over the partial labeling state-space.
//! Implements Algorithm 1 from "Algorithmic Complexity, Ramsey Stability,
//! and Hypergraph Generalizations" (Computational_DCL paper).
//!
//! Finds the minimum initial labeling f_0 for a given graph with max label <= k.

use dcl_core::gcd::are_coprime;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use std::collections::VecDeque;

/// Partial labeling state: maps vertex index to assigned label.
/// None means unassigned.
type PartialLabeling = Vec<Option<u64>>;

/// Result of one BFS search.
#[derive(Debug)]
pub struct BfsResult {
    pub labeling: Option<Labeling>,
    /// Total states dequeued (measure of search effort).
    pub states_explored: usize,
}

/// Maximum BFS queue size to prevent OOM on large graphs.
/// When exceeded, BFS returns None (labeling not found within resource limits).
const MAX_QUEUE_STATES: usize = 100_000;

/// Find a valid DCL initial labeling for `graph` with all labels in 1..=max_label.
/// Uses BFS over partial labeling state space (Algorithm 1 from Computational_DCL).
/// Vertices are processed in descending degree order for pruning efficiency.
/// Returns None if no labeling is found within `MAX_QUEUE_STATES` explored states.
pub fn find_dcl_bfs(graph: &Graph, max_label: u64) -> BfsResult {
    let ordered = graph.vertices_by_desc_degree();
    let n = graph.n;

    let mut queue: VecDeque<PartialLabeling> = VecDeque::new();
    queue.push_back(vec![None; n]);

    let mut states_explored = 0usize;

    while let Some(state) = queue.pop_front() {
        states_explored += 1;

        // Find the next unassigned vertex (in degree-sorted order)
        let assigned_count = state.iter().filter(|x| x.is_some()).count();
        if assigned_count == n {
            // All vertices assigned — build and return the labeling
            let labels: Vec<u64> = state.iter().map(|&x| x.unwrap()).collect();
            return BfsResult {
                labeling: Some(Labeling::new(labels)),
                states_explored,
            };
        }

        let vi = ordered[assigned_count];

        for candidate in 1..=max_label {
            // Check injectivity: candidate not already used
            if state.iter().any(|&x| x == Some(candidate)) {
                continue;
            }

            // Check coprimality with all already-assigned neighbors
            let feasible = graph.adj[vi].iter().all(|&neighbor| {
                match state[neighbor] {
                    Some(nb_label) => are_coprime(candidate, nb_label),
                    None => true, // unassigned neighbor — no constraint yet
                }
            });

            if feasible {
                let mut next = state.clone();
                next[vi] = Some(candidate);
                queue.push_back(next);
            }
        }
        if states_explored >= MAX_QUEUE_STATES {
            // Resource limit: too large, abandon
            break;
        }
    }

    BfsResult {
        labeling: None,
        states_explored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;

    #[test]
    fn bfs_finds_labeling_path5() {
        // P_5 with k=10: small enough for BFS, but has a valid coprime labeling
        let g = Graph::path(5);
        let result = find_dcl_bfs(&g, 10);
        assert!(
            result.labeling.is_some(),
            "BFS should find labeling for P_5 with k=10 (explored {} states)",
            result.states_explored
        );
        let lab = result.labeling.unwrap();
        assert!(g.is_coprime_labeling(&lab));
        assert!(lab.is_injective());
    }

    #[test]
    fn bfs_none_for_infeasible() {
        // K_3 (triangle) with max_label=2:
        // Labels must be 3 distinct values from {1,2}  — impossible
        let g = Graph::complete(3);
        let result = find_dcl_bfs(&g, 2);
        assert!(result.labeling.is_none(), "Should be infeasible");
    }

    #[test]
    fn bfs_cycle_c6() {
        let g = Graph::cycle(6);
        let result = find_dcl_bfs(&g, 15);
        assert!(result.labeling.is_some());
        let lab = result.labeling.unwrap();
        assert!(g.is_coprime_labeling(&lab));
    }

    #[test]
    fn bfs_wheel_w5() {
        let g = Graph::wheel(5);
        let result = find_dcl_bfs(&g, 15);
        assert!(result.labeling.is_some());
        let lab = result.labeling.unwrap();
        assert!(g.is_coprime_labeling(&lab));
    }
}
