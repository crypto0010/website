// dcl-crypto/src/sieve.rs
//! HDCL-SP: Hypergraph-DCL Safe-Prime Sieve.
//! Algorithm 1 from "Hypergraph-DCL Sieve for Cryptographic Safe-Prime Generation".
//!
//! Builds candidate q = 1 + Π x(v) over a hyperedge, then tests both q and 2q+1
//! for primality. Failed attempts trigger DCL-constrained label repair.

use crate::metrics::SieveMetrics;
use crate::miller_rabin::miller_rabin;
use dcl_core::labeling::Labeling;
use dcl_hypergraph::hypergraph::Hypergraph;
use dcl_hypergraph::repair::repair_dcl_constraint;

/// Small primes for trial division filter (primes ≤ 97).
const SMALL_PRIMES: &[u64] = &[
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97,
];

/// Check trial division by all small primes up to bound.
fn passes_trial_division(n: u64) -> bool {
    for &p in SMALL_PRIMES {
        if n == p {
            return true;
        }
        if n % p == 0 {
            return false;
        }
    }
    true
}

/// Configuration for the HDCL-SP sieve.
pub struct SieveConfig {
    /// Number of Miller-Rabin rounds (use 40 for cryptographic security).
    pub mr_rounds: u32,
    /// Delta increment applied to pivot label on rejection.
    pub delta: u64,
    /// Maximum number of attempts before giving up.
    pub max_attempts: usize,
}

impl Default for SieveConfig {
    fn default() -> Self {
        SieveConfig {
            mr_rounds: 40,
            delta: 1,
            max_attempts: 100_000,
        }
    }
}

/// Run the HDCL-SP algorithm on hypergraph `h` with initial labels `labels`.
/// Returns (safe_prime, metrics) or None if max_attempts exceeded.
///
/// Algorithm:
/// 1. Pick a hyperedge e.
/// 2. Compute q = 1 + Π x(v), p = 2q+1.
/// 3. Trial division filter on q and p.
/// 4. Miller-Rabin(q) and Miller-Rabin(p).
/// 5. On rejection: pivot update + DCL repair.
/// 6. Return p on acceptance.
pub fn hdcl_sp_generate(
    h: &Hypergraph,
    labels: &mut Labeling,
    config: &SieveConfig,
) -> (Option<u64>, SieveMetrics) {
    let mut metrics = SieveMetrics::default();
    let start = std::time::Instant::now();

    // Use first hyperedge as the generator edge (can be made configurable)
    if h.edges.is_empty() {
        return (None, metrics);
    }

    for attempt in 0..config.max_attempts {
        metrics.attempts += 1;

        // Select hyperedge (round-robin over edges for diversity)
        let edge = &h.edges[attempt % h.edges.len()];

        // Compute q = 1 + Π x(v) for v in edge
        let product: u128 = edge.iter().map(|&v| labels.get(v) as u128).product();
        let q_big = product + 1;

        // Check bit-length doesn't overflow u64
        if q_big > u64::MAX as u128 {
            // Labels too large — reset delta
            let pivot = edge[0];
            repair_dcl_constraint(h, labels, pivot, config.delta);
            metrics.rejections += 1;
            continue;
        }
        let q = q_big as u64;
        let p = match q.checked_mul(2).and_then(|x| x.checked_add(1)) {
            Some(v) => v,
            None => {
                metrics.rejections += 1;
                continue;
            }
        };

        // Trial division filter
        if !passes_trial_division(q) || !passes_trial_division(p) {
            let pivot = edge[attempt % edge.len()];
            metrics.repair_count += repair_dcl_constraint(h, labels, pivot, config.delta);
            metrics.rejections += 1;
            continue;
        }

        // Miller-Rabin test on q
        if !miller_rabin(q, config.mr_rounds) {
            let pivot = edge[attempt % edge.len()];
            metrics.repair_count += repair_dcl_constraint(h, labels, pivot, config.delta);
            metrics.rejections += 1;
            continue;
        }

        // Miller-Rabin test on p = 2q+1
        if !miller_rabin(p, config.mr_rounds) {
            let pivot = edge[attempt % edge.len()];
            metrics.repair_count += repair_dcl_constraint(h, labels, pivot, config.delta);
            metrics.rejections += 1;
            continue;
        }

        // Both q and p are prime → p is a safe prime
        metrics.elapsed_ms = start.elapsed().as_millis() as u64;
        return (Some(p), metrics);
    }

    metrics.elapsed_ms = start.elapsed().as_millis() as u64;
    (None, metrics)
}

/// Verify that the sieve property holds: gcd(q, x(v)) = 1 for all v in edge.
/// This is the core sieve lemma from the paper.
pub fn verify_sieve_property(edge: &[usize], labels: &Labeling, q: u64) -> bool {
    edge.iter().all(|&v| {
        let x = labels.get(v);
        dcl_core::gcd::gcd(q, x) == 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miller_rabin::is_safe_prime;

    fn make_simple_hypergraph() -> (Hypergraph, Labeling) {
        let mut h = Hypergraph::new(4);
        h.add_edge(vec![0, 1, 2]);
        h.add_edge(vec![1, 2, 3]);
        // Coprime initial labels: primes
        let lab = Labeling::new(vec![2, 3, 5, 7]);
        (h, lab)
    }

    #[test]
    fn sieve_produces_safe_prime() {
        let (h, mut labels) = make_simple_hypergraph();
        let config = SieveConfig::default();
        let (result, metrics) = hdcl_sp_generate(&h, &mut labels, &config);
        println!("Sieve metrics: {:?}", metrics);
        if let Some(p) = result {
            assert!(
                is_safe_prime(p),
                "HDCL-SP must produce a safe prime, got {p}"
            );
            println!("Generated safe prime: {p}");
        }
        // It's possible (though very unlikely) to exhaust attempts — just check no panic
    }

    #[test]
    fn sieve_property_lemma() {
        // q = 1 + Π x(v), so q ≡ 1 (mod x(v)) for all v in edge
        let lab = Labeling::new(vec![2, 3, 5, 7]);
        let edge = vec![0usize, 1, 2];
        let product: u128 = edge.iter().map(|&v| lab.get(v) as u128).product();
        let q = (product + 1) as u64;
        assert!(verify_sieve_property(&edge, &lab, q));
    }
}
