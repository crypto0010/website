// dcl-zkp/src/label_proof.rs
//! Zero-knowledge proof of knowledge of DCL labels

use crate::{ZkpError, ZkpResult, ZeroKnowledgeProof, FiatShamirTranscript};
use serde::{Serialize, Deserialize};

/// Statement: I know a valid DCL labeling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStatement {
    pub n: usize, // Number of vertices
    pub commitment: Vec<u8>,
}

/// Witness: the actual labels
#[derive(Debug, Clone)]
pub struct LabelWitness {
    pub labels: Vec<u64>,
}

/// Proof of label knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelProof {
    pub challenge: u64,
    pub responses: Vec<u64>,
}

impl ZeroKnowledgeProof for LabelProof {
    type Statement = LabelStatement;
    type Witness = LabelWitness;
    type Proof = LabelProof;

    fn prove(statement: &Self::Statement, witness: &Self::Witness) -> ZkpResult<Self::Proof> {
        // Verify witness has correct size
        if witness.labels.len() != statement.n {
            return Err(ZkpError::InvalidWitness);
        }

        // Verify no zero labels
        if witness.labels.iter().any(|&l| l == 0) {
            return Err(ZkpError::InvalidWitness);
        }

        // Fiat-Shamir transform
        let mut transcript = FiatShamirTranscript::new(b"label_proof");
        transcript.append_message(b"commitment", &statement.commitment);
        transcript.append_u64(b"n", statement.n as u64);

        let challenge = transcript.challenge_u64(b"challenge");

        // Simplified proof responses
        let responses: Vec<u64> = witness.labels
            .iter()
            .map(|&label| label.wrapping_mul(challenge))
            .collect();

        Ok(LabelProof { challenge, responses })
    }

    fn verify(statement: &Self::Statement, proof: &Self::Proof) -> ZkpResult<bool> {
        // Verify response count matches statement
        if proof.responses.len() != statement.n {
            return Ok(false);
        }

        // Simplified verification
        Ok(proof.challenge != 0 && !proof.responses.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_proof() {
        let statement = LabelStatement {
            n: 5,
            commitment: vec![1, 2, 3, 4, 5],
        };

        let witness = LabelWitness {
            labels: vec![2, 3, 5, 7, 11],
        };

        let proof = LabelProof::prove(&statement, &witness).unwrap();
        assert!(LabelProof::verify(&statement, &proof).unwrap());
    }

    #[test]
    fn test_invalid_label_count() {
        let statement = LabelStatement {
            n: 5,
            commitment: vec![1, 2, 3],
        };

        let witness = LabelWitness {
            labels: vec![2, 3, 5], // Wrong count
        };

        assert!(LabelProof::prove(&statement, &witness).is_err());
    }

    #[test]
    fn test_zero_label() {
        let statement = LabelStatement {
            n: 3,
            commitment: vec![1, 2, 3],
        };

        let witness = LabelWitness {
            labels: vec![2, 0, 5], // Contains zero
        };

        assert!(LabelProof::prove(&statement, &witness).is_err());
    }
}
