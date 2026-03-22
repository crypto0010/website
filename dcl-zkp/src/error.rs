// dcl-zkp/src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ZkpError {
    #[error("Invalid proof")]
    InvalidProof,

    #[error("Invalid commitment")]
    InvalidCommitment,

    #[error("Invalid witness")]
    InvalidWitness,

    #[error("Verification failed")]
    VerificationFailed,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type ZkpResult<T> = Result<T, ZkpError>;
