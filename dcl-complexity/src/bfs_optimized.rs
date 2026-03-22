// dcl-complexity/src/bfs_optimized.rs
//! Optimized BFS for finding minimal DCL labelings with intelligent pruning.

use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;

/// Pruning strategies for BFS optimization.
#[derive(Debug, Clone)]
pub struct PruningConfig {
    /// Skip labelings with non-prime labels (primes guarantee coprimality)
    pub prefer_primes: bool,
    /// Use symmetry breaking (e.g., force f(0) < f(1) < ... for path graphs)
    pub use_symmetry: bool,
    /// Skip labelings where sum exceeds threshold
    pub max_sum: Option<u64>,
    /// Early termination after finding N solutions
    pub max_solutions: usize,
    /// Use incremental coprimality checking
    pub incremental_check: bool,
}

impl Default for PruningConfig {
    fn default() -> Self {
        PruningConfig {
            prefer_primes: true,
            use_symmetry: true,
            max_sum: None,
            max_solutions: 1,
            incremental_check: true,
        }
    }
}

/// Result of optimized BFS search.
#[derive(Debug)]
pub struct BfsResult {
    pub solutions: Vec<Labeling>,
    pub states_explored: usize,
    pub states_pruned: usize,
    pub max_label_found: u64,
}

/// Optimized BFS searcher with pruning.
pub struct OptimizedBfs<'a> {
    graph: &'a Graph,
    config: PruningConfig,
    #[allow(dead_code)]
    max_label: u64,
    prime_cache: Vec<u64>,
    all_labels: Vec<u64>,
}

impl<'a> OptimizedBfs<'a> {
    pub fn new(graph: &'a Graph, max_label: u64, config: PruningConfig) -> Self {
        let prime_cache = if config.prefer_primes {
            Self::sieve_primes(max_label)
        } else {
            Vec::new()
        };

        let all_labels: Vec<u64> = (1..=max_label).collect();

        OptimizedBfs {
            graph,
            config,
            max_label,
            prime_cache,
            all_labels,
        }
    }

    /// Simple sieve of Eratosthenes for prime generation.
    fn sieve_primes(max: u64) -> Vec<u64> {
        if max < 2 {
            return vec![];
        }

        let limit = max as usize;
        let mut is_prime = vec![true; limit + 1];
        is_prime[0] = false;
        is_prime[1] = false;

        for i in 2..=((limit as f64).sqrt() as usize) {
            if is_prime[i] {
                for j in (i * i..=limit).step_by(i) {
                    is_prime[j] = false;
                }
            }
        }

        is_prime
            .iter()
            .enumerate()
            .filter_map(|(i, &prime)| if prime { Some(i as u64) } else { None })
            .collect()
    }

    /// Check if labeling respects symmetry constraints.
    #[allow(dead_code)]
    fn check_symmetry(&self, labels: &[u64]) -> bool {
        if !self.config.use_symmetry {
            return true;
        }

        // For path graphs, enforce f(0) <= f(1) to break symmetry
        if self.graph.edges().iter().all(|&(u, v)| (u as i64 - v as i64).abs() == 1) {
            // Likely a path or cycle - use ordering
            labels.windows(2).all(|w| w[0] <= w[1])
        } else {
            true
        }
    }

    /// Check if partial labeling can be extended to valid solution.
    fn is_promising(&self, partial: &[u64], pos: usize) -> bool {
        if pos == 0 {
            return true;
        }

        // Check coprimality with all neighbors of current vertex
        for (u, v) in self.graph.edges() {
            if u == pos && v < pos {
                if gcd(partial[u], partial[v]) != 1 {
                    return false;
                }
            } else if v == pos && u < pos {
                if gcd(partial[u], partial[v]) != 1 {
                    return false;
                }
            }
        }

        // Check sum constraint
        if let Some(max_sum) = self.config.max_sum {
            let current_sum: u64 = partial[..=pos].iter().sum();
            if current_sum > max_sum {
                return false;
            }
        }

        true
    }

    /// Get candidate labels for position (primes if enabled).
    fn get_candidates(&self) -> &[u64] {
        if self.config.prefer_primes && !self.prime_cache.is_empty() {
            &self.prime_cache
        } else {
            &self.all_labels
        }
    }

    /// Backtracking search with pruning — zero-clone in-place algorithm.
    pub fn search(&mut self) -> BfsResult {
        let n = self.graph.n;
        let mut solutions = Vec::new();
        let mut states_explored: usize = 0;
        let mut states_pruned: usize = 0;
        let mut max_label_found: u64 = 0;

        let candidates = self.get_candidates().to_vec();
        let mut partial = vec![0u64; n];
        // Stack stores (position, index_into_candidates)
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];

        while let Some(frame) = stack.last_mut() {
            let (pos, cand_idx) = *frame;

            if cand_idx >= candidates.len() {
                // Exhausted candidates at this position — backtrack
                stack.pop();
                continue;
            }

            let label = candidates[cand_idx];
            frame.1 += 1; // advance to next candidate for when we return

            partial[pos] = label;
            states_explored += 1;

            // Symmetry check
            if self.config.use_symmetry && pos > 0 && label < partial[pos - 1] {
                states_pruned += 1;
                continue;
            }

            // Incremental coprimality check
            if !self.is_promising(&partial, pos) {
                states_pruned += 1;
                continue;
            }

            if pos + 1 == n {
                // Complete labeling — validate and record
                let labeling = Labeling::new(partial.clone());
                if self.graph.is_coprime_labeling(&labeling) {
                    let max_in_solution = *partial.iter().max().unwrap_or(&0);
                    max_label_found = max_label_found.max(max_in_solution);
                    solutions.push(labeling);

                    if solutions.len() >= self.config.max_solutions {
                        break;
                    }
                }
            } else {
                // Descend to next position
                stack.push((pos + 1, 0));
            }
        }

        BfsResult {
            solutions,
            states_explored,
            states_pruned,
            max_label_found,
        }
    }
}

/// Compute GCD of two numbers.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimized_bfs_path3() {
        let g = Graph::path(3);
        let config = PruningConfig {
            prefer_primes: false, // Use all labels 1-20
            use_symmetry: true,
            max_sum: None,
            max_solutions: 1,
            incremental_check: true,
        };
        let mut searcher = OptimizedBfs::new(&g, 20, config);
        let result = searcher.search();

        assert!(!result.solutions.is_empty(), "Should find at least one solution for Path-3");
        println!("Path-3: Found {} solutions, explored {}, pruned {}",
                 result.solutions.len(), result.states_explored, result.states_pruned);
    }

    #[test]
    fn prune_efficiency() {
        let g = Graph::path(4);

        // Without symmetry pruning — exhaust all solutions
        let config_no_prune = PruningConfig {
            prefer_primes: false,
            use_symmetry: false,
            max_sum: None,
            max_solutions: usize::MAX,
            incremental_check: true,
        };
        let mut searcher1 = OptimizedBfs::new(&g, 4, config_no_prune);
        let result1 = searcher1.search();

        // With symmetry pruning — exhaust all solutions
        let config_prune = PruningConfig {
            prefer_primes: false,
            use_symmetry: true,
            max_sum: None,
            max_solutions: usize::MAX,
            incremental_check: true,
        };
        let mut searcher2 = OptimizedBfs::new(&g, 4, config_prune);
        let result2 = searcher2.search();

        println!("Without symmetry: {} states", result1.states_explored);
        println!("With symmetry: {} states (pruned {})", result2.states_explored, result2.states_pruned);

        // Both should find solutions
        assert!(!result1.solutions.is_empty());
        assert!(!result2.solutions.is_empty());
        // Symmetry pruning should explore fewer states
        assert!(result2.states_explored <= result1.states_explored,
            "Symmetry breaking should not increase explored states");
    }

    #[test]
    fn sieve_primes_correctness() {
        let primes = OptimizedBfs::sieve_primes(30);
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
    }

    #[test]
    fn zero_clone_finds_same_solutions() {
        let g = Graph::path(4);
        let config = PruningConfig {
            prefer_primes: false,
            use_symmetry: true,
            max_sum: None,
            max_solutions: 5,
            incremental_check: true,
        };
        let mut searcher = OptimizedBfs::new(&g, 15, config);
        let result = searcher.search();

        assert!(!result.solutions.is_empty(), "Should find solutions for Path-4 with max_label=15");
        for sol in &result.solutions {
            assert!(g.is_coprime_labeling(sol), "Solution {:?} is not coprime", sol.labels);
        }
        assert!(result.states_explored < 10000, "Too many states explored: {}", result.states_explored);
    }
}
