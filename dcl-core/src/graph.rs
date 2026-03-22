// dcl-core/src/graph.rs
//! Graph and Hypergraph types with named-family constructors.
//! Used as the structural substrate for DCL sequences.

use crate::error::{DclError, DclResult};
use crate::gcd::are_coprime;
use crate::labeling::Labeling;
use rand::Rng;

/// Undirected simple graph stored as adjacency list.
#[derive(Debug, Clone)]
pub struct Graph {
    pub n: usize,
    pub adj: Vec<Vec<usize>>,
}

impl Graph {
    /// Create an empty graph with n vertices.
    pub fn new(n: usize) -> Self {
        Graph {
            n,
            adj: vec![vec![]; n],
        }
    }

    /// Add an undirected edge (u, v) with bounds checking.
    /// Returns an error if vertices are out of bounds.
    pub fn try_add_edge(&mut self, u: usize, v: usize) -> DclResult<()> {
        if u >= self.n {
            return Err(DclError::index_out_of_bounds(u, self.n));
        }
        if v >= self.n {
            return Err(DclError::index_out_of_bounds(v, self.n));
        }
        self.adj[u].push(v);
        self.adj[v].push(u);
        Ok(())
    }

    /// Add an undirected edge (u, v). Does not check for duplicates.
    /// Panics if vertices are out of bounds.
    ///
    /// For error handling use `try_add_edge()` instead.
    pub fn add_edge(&mut self, u: usize, v: usize) {
        self.try_add_edge(u, v).expect("Vertices must be in bounds")
    }

    /// Collect all edges as (u, v) pairs with u < v.
    pub fn edges(&self) -> Vec<(usize, usize)> {
        let mut edges = Vec::new();
        for u in 0..self.n {
            for &v in &self.adj[u] {
                if u < v {
                    edges.push((u, v));
                }
            }
        }
        edges
    }

    /// Degree of vertex v.
    pub fn degree(&self, v: usize) -> usize {
        self.adj[v].len()
    }

    /// Vertices sorted by descending degree (used by DCL-BFS).
    pub fn vertices_by_desc_degree(&self) -> Vec<usize> {
        let mut vs: Vec<usize> = (0..self.n).collect();
        vs.sort_by(|&a, &b| self.degree(b).cmp(&self.degree(a)));
        vs
    }

    /// Check that a given labeling is a valid coprime labeling of this graph.
    pub fn is_coprime_labeling(&self, lab: &Labeling) -> bool {
        self.edges()
            .iter()
            .all(|&(u, v)| are_coprime(lab.get(u), lab.get(v)))
    }

    /// Degree sequence (sorted descending) — used as graph isomorphism pre-filter.
    pub fn degree_sequence(&self) -> Vec<usize> {
        let mut ds: Vec<usize> = (0..self.n).map(|v| self.degree(v)).collect();
        ds.sort_unstable_by(|a, b| b.cmp(a));
        ds
    }

    // --- Named graph family constructors ---

    /// Path graph P_n: 0-1-2-...(n-1)
    pub fn path(n: usize) -> Self {
        let mut g = Graph::new(n);
        for i in 0..n.saturating_sub(1) {
            g.add_edge(i, i + 1);
        }
        g
    }

    /// Cycle graph C_n: 0-1-2-...-(n-1)-0
    pub fn cycle(n: usize) -> Self {
        assert!(n >= 3, "Cycle requires at least 3 vertices");
        let mut g = Graph::new(n);
        for i in 0..n {
            g.add_edge(i, (i + 1) % n);
        }
        g
    }

    /// Wheel graph W_n: hub (vertex 0) connected to all vertices of C_{n-1}
    pub fn wheel(n: usize) -> Self {
        assert!(n >= 4, "Wheel requires at least 4 vertices (hub + 3-cycle)");
        let mut g = Graph::new(n);
        // Outer cycle: vertices 1..n-1
        for i in 1..n {
            g.add_edge(i, if i + 1 < n { i + 1 } else { 1 });
        }
        // Hub to all rim vertices
        for i in 1..n {
            g.add_edge(0, i);
        }
        g
    }

    /// n-dimensional hypercube Q_n: 2^n vertices, edges between vertices
    /// differing in exactly one bit.
    pub fn hypercube(n: usize) -> Self {
        assert!(n <= 20, "Hypercube Q_n with n>20 has >1M vertices");
        let num_vertices = 1usize << n;
        let mut g = Graph::new(num_vertices);
        for v in 0..num_vertices {
            for bit in 0..n {
                let u = v ^ (1 << bit);
                if u > v {
                    g.add_edge(v, u);
                }
            }
        }
        g
    }

    /// Erdős-Rényi random graph G(n, p).
    pub fn erdos_renyi(n: usize, p: f64, rng: &mut impl Rng) -> Self {
        let mut g = Graph::new(n);
        for u in 0..n {
            for v in (u + 1)..n {
                if rng.gen_bool(p) {
                    g.add_edge(u, v);
                }
            }
        }
        g
    }

    /// Complete graph K_n.
    pub fn complete(n: usize) -> Self {
        let mut g = Graph::new(n);
        for u in 0..n {
            for v in (u + 1)..n {
                g.add_edge(u, v);
            }
        }
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_edges() {
        let g = Graph::path(5);
        assert_eq!(g.edges().len(), 4);
    }

    #[test]
    fn cycle_edges() {
        let g = Graph::cycle(5);
        assert_eq!(g.edges().len(), 5);
    }

    #[test]
    fn wheel_edges() {
        // W_5: hub + 4-cycle → 4 (rim) + 4 (spokes) = 8 edges
        let g = Graph::wheel(5);
        assert_eq!(g.edges().len(), 8);
    }

    #[test]
    fn hypercube_q3() {
        // Q_3: 8 vertices, 12 edges
        let g = Graph::hypercube(3);
        assert_eq!(g.n, 8);
        assert_eq!(g.edges().len(), 12);
    }

    #[test]
    fn coprime_labeling_path() {
        let g = Graph::path(4);
        // Labels 1,2,3,4: edges (1,2)gcd=1, (2,3)gcd=1, (3,4)gcd=1 — valid
        let lab = Labeling::new(vec![1, 2, 3, 4]);
        assert!(g.is_coprime_labeling(&lab));
    }
}
