//! GPU error types for CUDA operations.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No CUDA device found")]
    DeviceNotFound,

    #[error("CUDA driver error: {0}")]
    Driver(#[from] cudarc::driver::DriverError),

    #[error("PTX compilation failed: {0}")]
    Compilation(String),

    #[error("Out of GPU memory: requested {requested} bytes, available {available} bytes")]
    OutOfMemory { requested: usize, available: usize },

    #[error("Kernel launch failed: {0}")]
    KernelLaunchFailed(String),
}

pub type GpuResult<T> = Result<T, GpuError>;
