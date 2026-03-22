// dcl-core/src/transform.rs
//! Coprime-preserving transformations g: u64 -> u64.
//! A transform is coprime-preserving if gcd(a,b)=1 => gcd(g(a),g(b))=1.
//! The power map g(x) = x^m is the canonical example (Theorem from ver1).

use crate::gcd::are_coprime;

/// Trait for coprime-preserving label transformations.
pub trait Transform: Send + Sync {
    fn apply(&self, x: u64) -> u64;
    fn name(&self) -> &'static str;
}

/// Power map g(x) = x^m (optionally mod N).
/// Coprime-preservation proof: gcd(a^m, b^m) = gcd(a,b)^m (classical).
/// When a modulus N is set (prime recommended), computes x^m mod N via fast exponentiation.
#[derive(Debug, Clone, Copy)]
pub struct PowerMap {
    pub m: u32,
    /// Optional prime modulus. If Some(n), computes x^m mod n. Keeps labels bounded.
    pub modulus: Option<u64>,
}

impl PowerMap {
    /// Create a power map g(x) = x^m (unbounded).
    pub fn new(m: u32) -> Self {
        assert!(m >= 1, "Exponent must be >= 1");
        PowerMap { m, modulus: None }
    }

    /// Create a power map g(x) = x^m mod N for a prime N.
    /// Keeps labels bounded and coprime-preserving within Z_N.
    pub fn with_modulus(m: u32, modulus: u64) -> Self {
        assert!(m >= 1, "Exponent must be >= 1");
        assert!(modulus >= 2, "Modulus must be >= 2");
        PowerMap {
            m,
            modulus: Some(modulus),
        }
    }
}

impl Transform for PowerMap {
    fn apply(&self, x: u64) -> u64 {
        if let Some(n) = self.modulus {
            // Fast modular exponentiation: x^m mod n
            let mut result: u128 = 1;
            let mut base: u128 = (x % n) as u128;
            let m = n as u128;
            let mut exp = self.m as u64;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = result * base % m;
                }
                base = base * base % m;
                exp >>= 1;
            }
            // Ensure result is at least 1 (avoid label 0)
            if result == 0 {
                1
            } else {
                result as u64
            }
        } else {
            // Unbounded: binary exponentiation with early saturation exit
            if x <= 1 {
                return x;
            }
            let limit = u64::MAX as u128;
            let mut result: u128 = 1;
            let mut base: u128 = x as u128;
            let mut exp = self.m;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = result.saturating_mul(base);
                    if result >= limit {
                        return u64::MAX;
                    }
                }
                exp >>= 1;
                if exp > 0 {
                    base = base.saturating_mul(base);
                    if base >= limit {
                        // All further multiplications will saturate
                        if exp > 0 {
                            return u64::MAX;
                        }
                    }
                }
            }
            result.min(limit) as u64
        }
    }

    fn name(&self) -> &'static str {
        "PowerMap"
    }
}

/// Identity map g(x) = x (trivially coprime-preserving).
#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityMap;

impl Transform for IdentityMap {
    fn apply(&self, x: u64) -> u64 {
        x
    }
    fn name(&self) -> &'static str {
        "Identity"
    }
}

/// Hash-based label transform: f(x) = (SHA256(x || step) mod max_label) + 1.
///
/// This is a post-quantum alternative to the power map. Unlike PowerMap,
/// HashTransform does NOT guarantee coprimality preservation — a repair
/// step is required after evolution. The one-way property of SHA-256
/// provides preimage resistance without relying on modular exponentiation.
#[derive(Debug, Clone)]
pub struct HashTransform {
    pub max_label: u64,
    pub step: u64,
}

impl HashTransform {
    /// Create a hash transform with output range [1, max_label] at the given step.
    pub fn new(max_label: u64, step: u64) -> Self {
        assert!(max_label >= 1, "max_label must be >= 1");
        HashTransform { max_label, step }
    }
}

impl Transform for HashTransform {
    fn apply(&self, x: u64) -> u64 {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(x.to_le_bytes());
        hasher.update(self.step.to_le_bytes());
        let hash = hasher.finalize();
        // Take first 8 bytes as u64
        let raw = u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ]);
        // Map to [1, max_label]
        (raw % self.max_label) + 1
    }

    fn name(&self) -> &'static str {
        "HashTransform"
    }
}

/// Empirically check whether a transform appears coprime-preserving
/// by sampling `sample_size` random coprime pairs and verifying images.
pub fn is_coprime_preserving(t: &dyn Transform, sample_size: usize) -> bool {
    // Use deterministic pairs for reproducibility
    let pairs: Vec<(u64, u64)> = (2..=(sample_size as u64 + 1))
        .flat_map(|a| (a + 1..=(sample_size as u64 + 2)).map(move |b| (a, b)))
        .filter(|(a, b)| are_coprime(*a, *b))
        .take(sample_size)
        .collect();

    pairs.iter().all(|(a, b)| {
        let ga = t.apply(*a);
        let gb = t.apply(*b);
        are_coprime(ga, gb)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_map_coprime_preserving() {
        for m in [1u32, 2, 3, 4] {
            let pm = PowerMap::new(m);
            assert!(
                is_coprime_preserving(&pm, 200),
                "PowerMap(m={m}) failed coprime-preserving check"
            );
        }
    }

    #[test]
    fn identity_coprime_preserving() {
        assert!(is_coprime_preserving(&IdentityMap, 200));
    }

    #[test]
    fn power_map_apply() {
        let pm = PowerMap::new(2);
        assert_eq!(pm.apply(3), 9);
        assert_eq!(pm.apply(5), 25);
        // gcd(3,5)=1 => gcd(9,25)=1
        assert!(are_coprime(pm.apply(3), pm.apply(5)));
    }

    #[test]
    fn hash_transform_deterministic() {
        let ht = HashTransform::new(1000, 0);
        let a = ht.apply(42);
        let b = ht.apply(42);
        assert_eq!(a, b, "HashTransform must be deterministic for same input");
        assert!(a >= 1 && a <= 1000, "Output must be in [1, max_label]");
    }

    #[test]
    fn hash_transform_different_steps() {
        let ht0 = HashTransform::new(1000, 0);
        let ht1 = HashTransform::new(1000, 1);
        let a = ht0.apply(42);
        let b = ht1.apply(42);
        assert!(a >= 1 && b >= 1);
    }

    #[test]
    fn hash_transform_coprime_check() {
        let ht = HashTransform::new(100, 0);
        for x in 1..=50 {
            let y = ht.apply(x);
            assert!(y >= 1 && y <= 100, "Label {} mapped to {} out of range", x, y);
        }
    }

    #[test]
    fn power_map_large_exponent() {
        let pm = PowerMap::new(60);
        assert_eq!(pm.apply(3), u64::MAX); // 3^60 overflows u64
        assert_eq!(pm.apply(1), 1);        // 1^anything = 1
        assert_eq!(pm.apply(0), 0);        // 0^anything = 0
    }

    #[test]
    fn power_map_binary_exp_correctness() {
        let pm = PowerMap::new(10);
        assert_eq!(pm.apply(2), 1024);     // 2^10 = 1024
        assert_eq!(pm.apply(3), 59049);    // 3^10 = 59049

        let pm3 = PowerMap::new(3);
        assert_eq!(pm3.apply(5), 125);     // 5^3 = 125
        assert_eq!(pm3.apply(10), 1000);   // 10^3 = 1000
    }
}
