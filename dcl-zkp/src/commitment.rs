// dcl-zkp/src/commitment.rs
//! Commitment schemes for zero-knowledge proofs

use sha2::{Sha256, Digest};
use rand::Rng;
use serde::{Serialize, Deserialize};

/// Commitment value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment(pub Vec<u8>);

/// Commitment opening (randomness used to create commitment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening(pub Vec<u8>);

/// Commitment scheme trait
pub trait CommitmentScheme {
    /// Commit to a value, returns (commitment, opening)
    fn commit(&self, value: u64) -> (Commitment, Opening);

    /// Verify a commitment
    fn verify(&self, commitment: &Commitment, value: u64, opening: &Opening) -> bool;
}

/// Pedersen-style commitment (simplified for demonstration)
pub struct PedersenCommitment;

impl PedersenCommitment {
    pub fn new() -> Self {
        PedersenCommitment
    }
}

impl Default for PedersenCommitment {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitmentScheme for PedersenCommitment {
    fn commit(&self, value: u64) -> (Commitment, Opening) {
        let mut rng = rand::thread_rng();
        let randomness: [u8; 32] = rng.gen();

        let mut hasher = Sha256::new();
        hasher.update(&value.to_le_bytes());
        hasher.update(&randomness);
        let commitment_bytes = hasher.finalize().to_vec();

        (Commitment(commitment_bytes), Opening(randomness.to_vec()))
    }

    fn verify(&self, commitment: &Commitment, value: u64, opening: &Opening) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&value.to_le_bytes());
        hasher.update(&opening.0);
        let computed = hasher.finalize().to_vec();

        commitment.0 == computed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment() {
        let scheme = PedersenCommitment::new();

        let value = 123u64;
        let (commitment, opening) = scheme.commit(value);

        assert!(scheme.verify(&commitment, value, &opening));
        assert!(!scheme.verify(&commitment, 124, &opening));
    }

    #[test]
    fn test_commitment_hiding() {
        let scheme = PedersenCommitment::new();

        let (c1, _) = scheme.commit(42);
        let (c2, _) = scheme.commit(42);

        // Different randomness should produce different commitments
        assert_ne!(c1, c2);
    }
}
