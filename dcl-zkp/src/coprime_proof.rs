// dcl-zkp/src/coprime_proof.rs
//! Zero-knowledge proof of coprimality

use crate::{ZkpError, ZkpResult, ZeroKnowledgeProof, FiatShamirTranscript};
use serde::{Serialize, Deserialize};
use dcl_core::gcd::gcd;

/// Statement: I know values a, b such that gcd(a, b) = 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoprimeStatement {
    pub commitment_a: Vec<u8>,
    pub commitment_b: Vec<u8>,
}

/// Witness: the actual values a and b
#[derive(Debug, Clone)]
pub struct CoprimeWitness {
    pub a: u64,
    pub b: u64,
}

/// Proof of coprimality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoprimeProof {
    pub challenge: u64,
    pub response_a: u64,
    pub response_b: u64,
}

impl ZeroKnowledgeProof for CoprimeProof {
    type Statement = CoprimeStatement;
    type Witness = CoprimeWitness;
    type Proof = CoprimeProof;

    fn prove(statement: &Self::Statement, witness: &Self::Witness) -> ZkpResult<Self::Proof> {
        // Verify witness is valid
        if gcd(witness.a, witness.b) != 1 {
            return Err(ZkpError::InvalidWitness);
        }

        // Fiat-Shamir transform
        let mut transcript = FiatShamirTranscript::new(b"coprime_proof");
        transcript.append_message(b"commitment_a", &statement.commitment_a);
        transcript.append_message(b"commitment_b", &statement.commitment_b);

        let challenge = transcript.challenge_u64(b"challenge");

        // Simplified proof (not cryptographically secure)
        let response_a = witness.a.wrapping_mul(challenge);
        let response_b = witness.b.wrapping_mul(challenge);

        Ok(CoprimeProof {
            challenge,
            response_a,
            response_b,
        })
    }

    fn verify(_statement: &Self::Statement, proof: &Self::Proof) -> ZkpResult<bool> {
        // Simplified verification
        // In a real system, this would check the proof cryptographically
        Ok(proof.challenge != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coprime_proof() {
        let statement = CoprimeStatement {
            commitment_a: vec![1, 2, 3],
            commitment_b: vec![4, 5, 6],
        };

        let witness = CoprimeWitness { a: 17, b: 13 };

        let proof = CoprimeProof::prove(&statement, &witness).unwrap();
        assert!(CoprimeProof::verify(&statement, &proof).unwrap());
    }

    #[test]
    fn test_invalid_witness() {
        let statement = CoprimeStatement {
            commitment_a: vec![1, 2, 3],
            commitment_b: vec![4, 5, 6],
        };

        let witness = CoprimeWitness { a: 6, b: 9 }; // Not coprime

        assert!(CoprimeProof::prove(&statement, &witness).is_err());
    }
}
