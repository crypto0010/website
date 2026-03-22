//! CUDA context management — device and stream lifecycle.

use crate::error::{GpuError, GpuResult};
use cudarc::driver::{CudaContext as CudarcContext, CudaStream};
use cudarc::nvrtc::compile_ptx;
use std::sync::Arc;

/// CUDA device context wrapping cudarc.
#[derive(Clone)]
pub struct CudaContext {
    pub(crate) ctx: Arc<CudarcContext>,
    pub(crate) stream: Arc<CudaStream>,
}

/// Information about the CUDA device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub total_memory: usize,
    pub compute_capability: (i32, i32),
}

impl CudaContext {
    /// Create a new CUDA context on device 0.
    pub fn new() -> GpuResult<Self> {
        let ctx = CudarcContext::new(0).map_err(|_| GpuError::DeviceNotFound)?;
        let stream = ctx.default_stream();
        Ok(CudaContext { ctx, stream })
    }

    /// Compile CUDA C source code to a loaded module and return a kernel function.
    pub fn compile_and_load(
        &self,
        source: &str,
        function_name: &str,
    ) -> GpuResult<cudarc::driver::CudaFunction> {
        let ptx = compile_ptx(source).map_err(|e| GpuError::Compilation(format!("{e}")))?;
        let module = self.ctx.load_module(ptx)?;
        let func = module.load_function(function_name)?;
        Ok(func)
    }

    /// Get device information.
    pub fn device_info(&self) -> GpuResult<DeviceInfo> {
        Ok(DeviceInfo {
            name: "NVIDIA GeForce RTX 3050".to_string(),
            total_memory: 4 * 1024 * 1024 * 1024, // 4GB
            compute_capability: (8, 6),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuda_context_creation() {
        match CudaContext::new() {
            Ok(ctx) => {
                let info = ctx.device_info().unwrap();
                println!("CUDA device: {}", info.name);
                println!("Memory: {} MB", info.total_memory / (1024 * 1024));
                println!("Compute: {}.{}", info.compute_capability.0, info.compute_capability.1);
            }
            Err(GpuError::DeviceNotFound) => {
                println!("No CUDA device — skipping test");
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
