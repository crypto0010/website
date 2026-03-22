// dcl-security/src/dl_experiment.rs
//! Discrete Logarithm Hardness Experiment.
//! Constructs a DCL label sequence that embeds a DL instance,
//! then attempts recovery by integer m-th root inversion.
//! Demonstrates the reduction: breaking DCL → solving integer m-th roots.

use dcl_core::dcl::DclSequence;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;

/// Result of a DL hardness experiment.
#[derive(Debug)]
pub struct DlExperimentResult {
    pub f0_recovered: bool,
    pub dl_solved: bool,
    pub recovery_time_ms: u64,
    pub dl_instance: DlInstance,
}

#[derive(Debug, Clone)]
pub struct DlInstance {
    pub p: u64, // prime modulus
    pub g: u64, // generator
    pub h: u64, // h = g^x mod p (public)
    pub x: u64, // secret exponent
}

fn mod_pow(base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result: u128 = 1;
    let mut b: u128 = (base as u128) % modulus as u128;
    let m = modulus as u128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u64
}

/// Build an f_0 that embeds a DL instance:
///   f_0(0) = g^x mod p = h  (secret vertex)
///   f_0(v) = g^v mod p      (public vertices)
pub fn build_embedded_f0(graph: &Graph, dl: &DlInstance) -> Labeling {
    let labels: Vec<u64> = (0..graph.n)
        .map(|v| {
            let exp = if v == 0 { dl.x } else { v as u64 };
            // Ensure label > 0 (mod result could be 0 for degenerate cases)
            let val = mod_pow(dl.g, exp, dl.p);
            if val == 0 {
                1
            } else {
                val
            }
        })
        .collect();
    Labeling::new(labels)
}

/// Run the DL hardness experiment:
/// 1. Embed DL instance into f_0.
/// 2. Evolve DCL sequence for `steps`.
/// 3. Attempt to recover f_0(0) from f_1(0) = f_0(0)^m via integer m-th root.
/// 4. Check if recovered value equals h (meaning DL is solved).
pub fn run_dl_experiment(
    graph: &Graph,
    transform: &PowerMap,
    dl: &DlInstance,
    steps: usize,
) -> DlExperimentResult {
    let start = std::time::Instant::now();

    let f0 = build_embedded_f0(graph, dl);

    // Run DCL sequence — only feasible if graph is coprime-labeled by f0
    // (may not always hold; we run anyway and record f_1)
    let mut seq = DclSequence::new(graph, transform, f0.clone());
    let mut observed: Vec<Labeling> = Vec::with_capacity(steps);
    for _ in 0..steps {
        seq.advance();
        observed.push(seq.current.clone());
    }

    let f1 = &observed[0];
    // Attempt recovery: f_1(0) = (f_0(0))^m; try integer m-th root
    let recovered = try_integer_mth_root(f1.get(0), transform.m);
    let f0_recovered = recovered.map(|r| r == f0.get(0)).unwrap_or(false);
    let dl_solved = recovered.map(|r| r == dl.h).unwrap_or(false);

    DlExperimentResult {
        f0_recovered,
        dl_solved,
        recovery_time_ms: start.elapsed().as_millis() as u64,
        dl_instance: dl.clone(),
    }
}

/// Attempt exact integer m-th root of n: returns Some(r) if r^m == n exactly.
fn try_integer_mth_root(n: u64, m: u32) -> Option<u64> {
    if m == 1 {
        return Some(n);
    }
    if n == 0 {
        return Some(0);
    }
    if n == 1 {
        return Some(1);
    }
    let mut lo = 1u64;
    let mut hi = (n as f64).powf(1.0 / m as f64).ceil() as u64 + 2;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let power = (mid as u128).pow(m);
        match power.cmp(&(n as u128)) {
            std::cmp::Ordering::Equal => return Some(mid),
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;
    use dcl_core::transform::PowerMap;

    #[test]
    fn dl_experiment_runs_no_panic() {
        let g = Graph::path(3);
        let pm = PowerMap::new(2);
        let h = mod_pow(5, 7, 23); // 5^7 mod 23 = 17
        let dl = DlInstance {
            p: 23,
            g: 5,
            h,
            x: 7,
        };
        assert_eq!(dl.h, 17);
        let result = run_dl_experiment(&g, &pm, &dl, 3);
        println!(
            "f0_recovered={}, dl_solved={}, time={}ms",
            result.f0_recovered, result.dl_solved, result.recovery_time_ms
        );
    }

    #[test]
    fn mth_root_exact() {
        assert_eq!(try_integer_mth_root(9, 2), Some(3));
        assert_eq!(try_integer_mth_root(27, 3), Some(3));
        assert_eq!(try_integer_mth_root(10, 2), None);
    }
}
