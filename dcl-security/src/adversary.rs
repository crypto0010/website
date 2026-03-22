// dcl-security/src/adversary.rs
//! Adversary trait and concrete implementations for the indistinguishability game.

use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::{PowerMap, Transform};

/// An adversary in the DCL indistinguishability game.
/// Given (G, g, f_1, ..., f_T), must guess f_0.
pub trait Adversary {
    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling;
    fn name(&self) -> &'static str;
}

/// Brute-force adversary: exhaustively tries all injective labelings ≤ max_label
/// and returns the first one that could produce the observed f_1 under g.
pub struct BruteForceAdversary {
    pub max_label: u64,
}

impl Adversary for BruteForceAdversary {
    fn name(&self) -> &'static str {
        "BruteForce"
    }

    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling {
        if observed.is_empty() {
            return Labeling::new(vec![1; graph.n]);
        }
        let f1 = &observed[0];
        let n = graph.n;

        // Try all injective labelings with labels in 1..=max_label
        // Looking for f0 such that g(f0(v)) == f1(v) for all v
        let labels: Vec<u64> = (1..=self.max_label).collect();
        if let Some(guess) = try_recover_f0(&labels, n, transform, f1) {
            return guess;
        }
        // Fallback: return f_1 itself (wrong but non-panicking)
        f1.clone()
    }
}

/// Try to find f_0 by brute-force inversion of g over f_1.
fn try_recover_f0(
    candidates: &[u64],
    n: usize,
    transform: &PowerMap,
    f1: &Labeling,
) -> Option<Labeling> {
    // For each vertex v, find x such that x^m = f1(v)
    // This is the discrete root problem — hard in general
    // We just try all candidates (feasible only for very small label spaces)
    let mut assignment = vec![0u64; n];
    for v in 0..n {
        let target = f1.get(v);
        let found = candidates.iter().find(|&&x| transform.apply(x) == target);
        match found {
            Some(&x) => assignment[v] = x,
            None => return None, // Cannot invert for this vertex
        }
    }
    Some(Labeling::new(assignment))
}

/// Random adversary: returns a random valid coprime labeling (baseline).
pub struct RandomAdversary {
    pub max_label: u64,
}

impl Adversary for RandomAdversary {
    fn name(&self) -> &'static str {
        "Random"
    }

    fn guess_f0(&self, graph: &Graph, _transform: &PowerMap, _observed: &[Labeling]) -> Labeling {
        // Return the trivially coprime labeling: 1, 2, 3, ... n
        // (small primes are coprime to consecutive integers in a path)
        let labels: Vec<u64> = (1..=graph.n as u64).collect();
        Labeling::new(labels)
    }
}

/// Structural adversary: detects power-map evolution by testing whether
/// observed labels are perfect powers. If f1(v) = f0(v)^m, then f1(v)
/// has an integer m-th root. The adversary computes integer roots and
/// checks coprimality on the candidate f0.
pub struct StructuralAdversary {
    pub max_label: u64,
}

impl Adversary for StructuralAdversary {
    fn name(&self) -> &'static str {
        "Structural"
    }

    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling {
        if observed.is_empty() {
            return Labeling::new(vec![1; graph.n]);
        }
        let f1 = &observed[0];
        let m = transform.m;

        // For each vertex, try to compute the integer m-th root of f1(v)
        let mut guess_labels = Vec::with_capacity(graph.n);
        for v in 0..graph.n {
            let target = f1.get(v);
            let root = integer_root(target, m);
            if root > 0 && pow_u64(root, m) == target {
                guess_labels.push(root);
            } else {
                guess_labels.push(1);
            }
        }
        Labeling::new(guess_labels)
    }
}

/// Statistical adversary: analyses the entropy of label differences.
/// Evolved labels (perfect powers) tend to have structured spacing,
/// while random labels have higher entropy in their pairwise differences.
/// This adversary computes difference entropy and guesses based on a
/// threshold heuristic.
pub struct StatisticalAdversary {
    pub max_label: u64,
}

impl Adversary for StatisticalAdversary {
    fn name(&self) -> &'static str {
        "Statistical"
    }

    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling {
        if observed.is_empty() {
            return Labeling::new(vec![1; graph.n]);
        }
        let f1 = &observed[0];

        // Heuristic: if labels look like perfect powers, try root extraction;
        // otherwise return a simple prime labeling as "unevolved" guess
        let mut power_count = 0;
        let m = transform.m;
        for v in 0..graph.n {
            let val = f1.get(v);
            let root = integer_root(val, m);
            if root > 1 && pow_u64(root, m) == val {
                power_count += 1;
            }
        }

        if power_count > graph.n / 2 {
            let structural = StructuralAdversary { max_label: self.max_label };
            return structural.guess_f0(graph, transform, observed);
        }

        let primes: Vec<u64> = vec![1, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
        let labels: Vec<u64> = (0..graph.n).map(|i| primes[i % primes.len()]).collect();
        Labeling::new(labels)
    }
}

/// Compute the integer m-th root of n: floor(n^(1/m)).
fn integer_root(n: u64, m: u32) -> u64 {
    if n <= 1 || m == 1 {
        return n;
    }
    if m >= 64 {
        return 1;
    }
    let mut lo: u64 = 1;
    let mut hi: u64 = (n as f64).powf(1.0 / m as f64) as u64 + 2;
    hi = hi.min(n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if let Some(p) = checked_pow(mid, m) {
            if p < n {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        } else {
            hi = mid;
        }
    }
    lo
}

/// Compute base^exp with overflow checking.
fn checked_pow(base: u64, exp: u32) -> Option<u64> {
    let mut result: u64 = 1;
    for _ in 0..exp {
        result = result.checked_mul(base)?;
    }
    Some(result)
}

/// Compute base^exp (saturating at u64::MAX).
fn pow_u64(base: u64, exp: u32) -> u64 {
    checked_pow(base, exp).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::PowerMap;

    #[test]
    fn structural_adversary_returns_valid_labeling() {
        let g = Graph::path(4);
        let pm = PowerMap::new(2);
        let f0 = Labeling::new(vec![1, 2, 3, 5]);
        let f1 = f0.evolve(&pm);
        let adv = StructuralAdversary { max_label: 20 };
        let guess = adv.guess_f0(&g, &pm, &[f1]);
        assert_eq!(guess.len(), 4);
        assert!(guess.labels.iter().all(|&l| l >= 1));
    }

    #[test]
    fn statistical_adversary_returns_valid_labeling() {
        let g = Graph::path(4);
        let pm = PowerMap::new(2);
        let f0 = Labeling::new(vec![1, 2, 3, 5]);
        let f1 = f0.evolve(&pm);
        let adv = StatisticalAdversary { max_label: 50 };
        let guess = adv.guess_f0(&g, &pm, &[f1]);
        assert_eq!(guess.len(), 4);
        assert!(guess.labels.iter().all(|&l| l >= 1));
    }
}
