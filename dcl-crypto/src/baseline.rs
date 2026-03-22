// dcl-crypto/src/baseline.rs
//! Standard safe-prime generation baseline (no DCL structure).
//! Used for benchmarking against HDCL-SP.

use crate::metrics::SieveMetrics;
use crate::miller_rabin::miller_rabin;
use rand::Rng;

/// Generate a safe prime by the standard method:
/// 1. Sample random odd q with appropriate bit-length.
/// 2. Test q with Miller-Rabin.
/// 3. If q is prime, test p = 2q+1.
/// 4. Repeat until success.
///
/// Works for small bit-lengths (≤ 62) to stay within u64 range.
pub fn standard_safe_prime(bit_length: u32, rng: &mut impl Rng) -> (Option<u64>, SieveMetrics) {
    assert!(bit_length <= 62, "bit_length must be ≤ 62 for u64 range");
    let mut metrics = SieveMetrics::default();
    let start = std::time::Instant::now();

    let lo = 1u64 << (bit_length - 1);
    let hi = (1u64 << bit_length) - 1;

    for _ in 0..100_000 {
        metrics.attempts += 1;
        let mut q: u64 = rng.gen_range(lo..=hi);
        if q % 2 == 0 {
            q += 1;
        }
        if !miller_rabin(q, 40) {
            metrics.rejections += 1;
            continue;
        }
        let p = match q.checked_mul(2).and_then(|x| x.checked_add(1)) {
            Some(v) => v,
            None => {
                metrics.rejections += 1;
                continue;
            }
        };
        if miller_rabin(p, 40) {
            metrics.elapsed_ms = start.elapsed().as_millis() as u64;
            return (Some(p), metrics);
        }
        metrics.rejections += 1;
    }
    metrics.elapsed_ms = start.elapsed().as_millis() as u64;
    (None, metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miller_rabin::is_safe_prime;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn standard_baseline_produces_safe_prime() {
        let mut rng = StdRng::seed_from_u64(42);
        let (result, metrics) = standard_safe_prime(16, &mut rng);
        println!("Baseline metrics: {:?}", metrics);
        assert!(
            result.is_some(),
            "Should produce a safe prime within 100k attempts"
        );
        let p = result.unwrap();
        assert!(is_safe_prime(p), "Baseline result {p} must be a safe prime");
    }
}
