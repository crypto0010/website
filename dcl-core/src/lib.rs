// dcl-core/src/lib.rs
pub mod error;
pub mod gcd;
pub mod transform;
pub mod labeling;
pub mod labeling_bigint;
pub mod graph;
pub mod dcl;

// Re-export commonly used types
pub use error::{DclError, DclResult};
