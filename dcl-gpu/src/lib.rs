//! DCL-GPU: CUDA-accelerated operations for the DCL framework.
//! Targets NVIDIA GPUs via cudarc (CUDA driver + nvrtc runtime compilation).

pub mod benchmark;
pub mod context;
pub mod error;
pub mod kernels;

pub use context::CudaContext;
pub use error::{GpuError, GpuResult};
