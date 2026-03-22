// dcl-zkp/src/lib.rs
//! Zero-Knowledge Proof system for DCL Framework
//! Implements ZK proofs for coprimality and label knowledge
//!
//! WARNING: This is a demonstration implementation for research purposes.
//! NOT suitable for production cryptographic use without security audit.

pub mod commitment;
pub mod coprime_proof;
pub mod label_proof;
pub mod fiat_shamir;
pub mod error;

pub use commitment::{Commitment, CommitmentScheme, PedersenCommitment};
pub use coprime_proof::{CoprimeProof, CoprimeStatement};
pub use label_proof::{LabelProof, LabelStatement};
pub use fiat_shamir::FiatShamirTranscript;
pub use error::{ZkpError, ZkpResult};

/// Zero-knowledge proof interface
pub trait ZeroKnowledgeProof: Sized {
    type Statement;
    type Witness;
    type Proof: serde::Serialize + for<'de> serde::Deserialize<'de>;

    /// Generate a proof for a statement given the witness
    fn prove(statement: &Self::Statement, witness: &Self::Witness) -> ZkpResult<Self::Proof>;

    /// Verify a proof for a statement
    fn verify(statement: &Self::Statement, proof: &Self::Proof) -> ZkpResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_commitment() {
        let scheme = PedersenCommitment::new();
        let value = 42u64;
        let (commitment, opening) = scheme.commit(value);

        assert!(scheme.verify(&commitment, value, &opening));
        assert!(!scheme.verify(&commitment, 43, &opening));
    }
}
