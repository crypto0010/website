// dcl-core/src/error.rs
//! Comprehensive error types for the DCL framework.
//! Provides graceful error handling instead of panics for production use.

/// Main error type for all DCL operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DclError {
    /// Labeling contains invalid values (e.g., zero or non-positive labels)
    #[error("Invalid labeling: {0}")]
    InvalidLabeling(String),

    /// Labeling is not injective (contains duplicate labels)
    #[error("Non-injective labeling: duplicate labels found")]
    NonInjectiveLabeling,

    /// Coprimality constraint violated on an edge
    #[error("Coprimality violation on edge ({u}, {v}): gcd({label_u}, {label_v}) = {gcd}")]
    CoprimalityViolation {
        u: usize,
        v: usize,
        label_u: u64,
        label_v: u64,
        gcd: u64,
    },

    /// Overflow detected during evolution
    #[error("Overflow at step {step}: {message}")]
    Overflow { step: usize, message: String },

    /// Graph configuration is invalid
    #[error("Invalid graph: {0}")]
    InvalidGraph(String),

    /// Security validation failed
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    /// Index out of bounds
    #[error("Index out of bounds: attempted to access index {index} but length is {len}")]
    IndexOutOfBounds { index: usize, len: usize },

    /// Mismatched dimensions (e.g., labeling size doesn't match graph vertices)
    #[error("Dimension mismatch: expected {expected} vertices but got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Prime generation failed
    #[error("Prime generation failed after {attempts} attempts")]
    PrimeGenerationFailed { attempts: usize },

    /// Invalid security configuration
    #[error("Invalid security configuration: {0}")]
    InvalidSecurityConfig(String),

    /// Search/optimization failed
    #[error("Search failed: {0}")]
    SearchFailed(String),

    /// Invalid transform parameters
    #[error("Invalid transform: {0}")]
    InvalidTransform(String),

    /// BigInt operation error
    #[error("BigInt operation error: {0}")]
    BigIntError(String),

    /// Generic computation error
    #[error("Computation error: {0}")]
    ComputationError(String),
}

/// Result type alias for DCL operations
pub type DclResult<T> = Result<T, DclError>;

impl DclError {
    /// Create a coprimality violation error with detailed context
    pub fn coprimality_violation(
        u: usize,
        v: usize,
        label_u: u64,
        label_v: u64,
    ) -> Self {
        use crate::gcd::gcd;
        let gcd_val = gcd(label_u, label_v);
        DclError::CoprimalityViolation {
            u,
            v,
            label_u,
            label_v,
            gcd: gcd_val,
        }
    }

    /// Create an overflow error with context
    pub fn overflow(step: usize, message: impl Into<String>) -> Self {
        DclError::Overflow {
            step,
            message: message.into(),
        }
    }

    /// Create an invalid labeling error
    pub fn invalid_labeling(message: impl Into<String>) -> Self {
        DclError::InvalidLabeling(message.into())
    }

    /// Create a dimension mismatch error
    pub fn dimension_mismatch(expected: usize, actual: usize) -> Self {
        DclError::DimensionMismatch { expected, actual }
    }

    /// Create an index out of bounds error
    pub fn index_out_of_bounds(index: usize, len: usize) -> Self {
        DclError::IndexOutOfBounds { index, len }
    }

    /// Create a security violation error
    pub fn security_violation(message: impl Into<String>) -> Self {
        DclError::SecurityViolation(message.into())
    }
}

/// Extension trait for converting Option to DclResult
pub trait DclResultExt<T> {
    /// Convert None to an error
    fn ok_or_dcl(self, err: DclError) -> DclResult<T>;
}

impl<T> DclResultExt<T> for Option<T> {
    fn ok_or_dcl(self, err: DclError) -> DclResult<T> {
        self.ok_or(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = DclError::InvalidLabeling("zero label detected".to_string());
        assert!(err.to_string().contains("Invalid labeling"));
        assert!(err.to_string().contains("zero label"));
    }

    #[test]
    fn coprimality_violation_error() {
        let err = DclError::coprimality_violation(0, 1, 6, 9);
        assert!(err.to_string().contains("edge (0, 1)"));
        assert!(err.to_string().contains("gcd(6, 9) = 3"));
    }

    #[test]
    fn overflow_error() {
        let err = DclError::overflow(5, "u64 overflow detected");
        assert!(err.to_string().contains("step 5"));
        assert!(err.to_string().contains("u64 overflow"));
    }

    #[test]
    fn dimension_mismatch_error() {
        let err = DclError::dimension_mismatch(5, 3);
        assert!(err.to_string().contains("expected 5"));
        assert!(err.to_string().contains("got 3"));
    }

    #[test]
    fn result_conversion() {
        let result: DclResult<u64> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let result: DclResult<u64> = Err(DclError::InvalidLabeling("test".to_string()));
        assert!(result.is_err());
    }
}
