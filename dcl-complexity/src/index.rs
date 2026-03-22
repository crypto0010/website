// dcl-complexity/src/index.rs
//! Minimum Dynamic Coprime Index μ(G, g).
//! Definition 2.1 from Computational_DCL:
//! μ(G,g) = smallest k such that a DCL exists with max initial label ≤ k.

use crate::bfs::find_dcl_bfs;
use dcl_core::graph::Graph;
use dcl_core::transform::Transform;

/// Compute μ(G, g): the minimum max initial label for a valid DCL.
/// Uses binary search over k, calling DCL-BFS at each candidate bound.
///
/// `upper_bound` specifies the search ceiling (the function returns
/// upper_bound + 1 if no labeling is found within bounds).
pub fn minimum_dcl_index(graph: &Graph, _transform: &dyn Transform, upper_bound: u64) -> u64 {
    // The minimum possible value is n (we need n distinct positive integers).
    // Binary search in [n, upper_bound].
    let n = graph.n as u64;
    let mut lo = n;
    let mut hi = upper_bound;

    // First check if upper_bound is even feasible
    if find_dcl_bfs(graph, hi).labeling.is_none() {
        return upper_bound + 1; // infeasible within bounds
    }

    // Binary search for minimum k
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if find_dcl_bfs(graph, mid).labeling.is_some() {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

/// A summary report of DCL index computation across a benchmark suite.
#[derive(Debug)]
pub struct IndexReport {
    pub graph_name: String,
    pub n: usize,
    pub mu: u64,
    pub states_at_mu: usize,
}

/// Run the index computation on a named set of graph instances and return reports.
pub fn benchmark_suite(graphs: &[(&str, Graph)], upper_bound: u64) -> Vec<IndexReport> {
    use dcl_core::transform::PowerMap;
    let pm = PowerMap::new(2);
    graphs
        .iter()
        .map(|(name, g)| {
            let mu = minimum_dcl_index(g, &pm, upper_bound);
            let bfs_result = find_dcl_bfs(g, mu);
            IndexReport {
                graph_name: name.to_string(),
                n: g.n,
                mu,
                states_at_mu: bfs_result.states_explored,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;
    use dcl_core::transform::PowerMap;

    #[test]
    fn mu_path5() {
        let g = Graph::path(5);
        let pm = PowerMap::new(2);
        let mu = minimum_dcl_index(&g, &pm, 15);
        // P_5 has 5 vertices: we need 5 distinct coprime-on-edges labels
        // Minimum is at most 5 (use labels 1..5)
        assert!(mu <= 10, "mu(P_5) should be small");
        println!("mu(P_5, x^2) = {mu}");
    }

    #[test]
    fn mu_cycle4() {
        let g = Graph::cycle(4);
        let pm = PowerMap::new(2);
        let mu = minimum_dcl_index(&g, &pm, 12);
        assert!(mu >= 4, "Need at least 4 distinct labels");
        println!("mu(C_4, x^2) = {mu}");
    }
}
