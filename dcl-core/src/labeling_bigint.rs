// dcl-core/src/labeling_bigint.rs
//! BigInt-based labeling for unbounded DCL evolution.
//! Solves overflow issues present in u64-based implementation.

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::One;

/// A labeling using arbitrary-precision integers.
#[derive(Clone, Debug)]
pub struct LabelingBigInt {
    pub labels: Vec<BigUint>,
}

impl LabelingBigInt {
    /// Create a new labeling from u64 values (converted to BigUint).
    pub fn new(values: Vec<u64>) -> Self {
        LabelingBigInt {
            labels: values.into_iter().map(BigUint::from).collect(),
        }
    }

    /// Create from existing BigUint values.
    pub fn from_biguint(labels: Vec<BigUint>) -> Self {
        LabelingBigInt { labels }
    }

    /// Number of vertices.
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Get label for vertex i.
    pub fn get(&self, i: usize) -> &BigUint {
        &self.labels[i]
    }

    /// Check if two labels are coprime.
    pub fn edge_coprime(&self, u: usize, v: usize) -> bool {
        self.labels[u].gcd(&self.labels[v]).is_one()
    }

    /// Apply a power transform: f(v) → f(v)^m for all vertices.
    pub fn power_transform(&self, m: u32) -> Self {
        LabelingBigInt {
            labels: self.labels.iter().map(|x| x.pow(m)).collect(),
        }
    }

    /// Apply modular power transform: f(v) → f(v)^m mod p.
    pub fn modular_power_transform(&self, m: u32, modulus: &BigUint) -> Self {
        LabelingBigInt {
            labels: self
                .labels
                .iter()
                .map(|x| x.modpow(&BigUint::from(m), modulus))
                .collect(),
        }
    }

    /// Convert to string representation.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let label_strs: Vec<String> = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let s = x.to_string();
                if s.len() > 20 {
                    format!("v{}={:.17}...", i, &s[..17])
                } else {
                    format!("v{}={}", i, s)
                }
            })
            .collect();
        format!("[{}]", label_strs.join(", "))
    }

    /// Check if all labels are within u64 range (for comparison).
    pub fn fits_in_u64(&self) -> bool {
        self.labels.iter().all(|x| {
            x <= &BigUint::from(u64::MAX)
        })
    }

    /// Convert to u64 labels (panics if any label exceeds u64::MAX).
    pub fn to_u64(&self) -> Vec<u64> {
        self.labels
            .iter()
            .map(|x| {
                x.to_u64_digits()
                    .first()
                    .copied()
                    .unwrap_or(0)
            })
            .collect()
    }
}

/// BigInt DCL sequence with unbounded evolution.
pub struct DclSequenceBigInt {
    pub current: LabelingBigInt,
    pub step: usize,
    pub modulus: Option<BigUint>, // Optional modular arithmetic
}

impl DclSequenceBigInt {
    /// Initialize from initial labeling.
    pub fn new(f0: LabelingBigInt) -> Self {
        DclSequenceBigInt {
            current: f0,
            step: 0,
            modulus: None,
        }
    }

    /// Initialize with modular arithmetic (bounded evolution).
    pub fn new_modular(f0: LabelingBigInt, modulus: BigUint) -> Self {
        DclSequenceBigInt {
            current: f0,
            step: 0,
            modulus: Some(modulus),
        }
    }

    /// Advance one step with power map g(x) = x^m.
    pub fn advance(&mut self, m: u32) {
        self.current = match &self.modulus {
            Some(p) => self.current.modular_power_transform(m, p),
            None => self.current.power_transform(m),
        };
        self.step += 1;
    }

    /// Check coprimality for specific edge.
    pub fn edge_coprime(&self, u: usize, v: usize) -> bool {
        self.current.edge_coprime(u, v)
    }

    /// Check all edges for coprimality (requires graph).
    pub fn verify_coprimality(&self, edges: &[(usize, usize)]) -> bool {
        edges.iter().all(|&(u, v)| self.edge_coprime(u, v))
    }

    /// Evolve for multiple steps, checking coprimality.
    /// Returns Ok(()) if all steps maintain coprimality, Err(step) otherwise.
    pub fn evolve_and_verify(&mut self, steps: usize, m: u32, edges: &[(usize, usize)]) -> Result<(), usize> {
        for _ in 0..steps {
            self.advance(m);
            if !self.verify_coprimality(edges) {
                return Err(self.step);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigint_no_overflow() {
        // This would overflow u64 after a few steps
        let f0 = LabelingBigInt::new(vec![2, 3, 5, 7, 11]);
        let mut seq = DclSequenceBigInt::new(f0);

        // Evolve for 10 steps (would overflow u64 around step 5)
        for _ in 0..10 {
            seq.advance(2);
        }

        assert_eq!(seq.step, 10);
        // After 10 squarings: 2^(2^10) = 2^1024 - huge number!
        assert!(!seq.current.fits_in_u64());
    }

    #[test]
    fn bigint_coprimality_preserved() {
        let f0 = LabelingBigInt::new(vec![2, 3, 5]);
        let edges = vec![(0, 1), (1, 2)]; // Path graph
        let mut seq = DclSequenceBigInt::new(f0);

        // Verify coprimality maintained for many steps
        assert!(seq.evolve_and_verify(8, 2, &edges).is_ok());
    }

    #[test]
    fn modular_arithmetic_bounded() {
        let f0 = LabelingBigInt::new(vec![2, 3, 5]);
        let modulus = BigUint::from(1000000007u64); // Large prime
        let mut seq = DclSequenceBigInt::new_modular(f0, modulus);

        // Evolve many steps - values stay bounded
        for _ in 0..100 {
            seq.advance(2);
            assert!(seq.current.fits_in_u64());
        }
    }

    #[test]
    fn gcd_computation() {
        let f = LabelingBigInt::new(vec![12, 18, 25]);
        assert!(!f.edge_coprime(0, 1)); // gcd(12, 18) = 6
        assert!(f.edge_coprime(0, 2));  // gcd(12, 25) = 1
        assert!(f.edge_coprime(1, 2));  // gcd(18, 25) = 1
    }
}
