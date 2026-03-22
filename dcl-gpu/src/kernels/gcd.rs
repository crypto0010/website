//! Batch GCD computation on GPU using CUDA.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::LaunchConfig;
use cudarc::driver::PushKernelArg;

const GCD_KERNEL_SRC: &str = r#"
extern "C" __global__ void batch_gcd(
    const unsigned long long* a,
    const unsigned long long* b,
    unsigned long long* gcd_out,
    unsigned int* coprime_out,
    const int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;

    unsigned long long x = a[idx];
    unsigned long long y = b[idx];

    // Stein's binary GCD
    if (x == 0) { gcd_out[idx] = y; coprime_out[idx] = (y == 1) ? 1 : 0; return; }
    if (y == 0) { gcd_out[idx] = x; coprime_out[idx] = (x == 1) ? 1 : 0; return; }

    int shift = 0;
    while (((x | y) & 1) == 0) { x >>= 1; y >>= 1; shift++; }
    while ((x & 1) == 0) x >>= 1;

    do {
        while ((y & 1) == 0) y >>= 1;
        if (x > y) { unsigned long long t = x; x = y; y = t; }
        y -= x;
    } while (y != 0);

    unsigned long long result = x << shift;
    gcd_out[idx] = result;
    coprime_out[idx] = (result == 1) ? 1 : 0;
}
"#;

/// GPU-accelerated batch GCD computation.
pub struct GpuGcdBatch {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuGcdBatch {
    /// Create a new batch GCD processor. Compiles the CUDA kernel.
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(GCD_KERNEL_SRC, "batch_gcd")?;
        Ok(GpuGcdBatch { ctx, kernel })
    }

    /// Compute GCD for each pair (a[i], b[i]). Returns (gcd_results, coprime_flags).
    pub fn compute_batch(&self, a: &[u64], b: &[u64]) -> GpuResult<(Vec<u64>, Vec<u32>)> {
        assert_eq!(a.len(), b.len(), "Input arrays must have equal length");
        let n = a.len();
        if n == 0 {
            return Ok((vec![], vec![]));
        }

        let stream = &self.ctx.stream;

        let a_dev = stream.clone_htod(a)?;
        let b_dev = stream.clone_htod(b)?;
        let mut gcd_dev = stream.alloc_zeros::<u64>(n)?;
        let mut coprime_dev = stream.alloc_zeros::<u32>(n)?;
        let n_val = n as i32;

        let cfg = LaunchConfig::for_num_elems(n as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&a_dev);
        builder.arg(&b_dev);
        builder.arg(&mut gcd_dev);
        builder.arg(&mut coprime_dev);
        builder.arg(&n_val);
        unsafe { builder.launch(cfg) }?;

        let gcd_host = stream.clone_dtoh(&gcd_dev)?;
        let coprime_host = stream.clone_dtoh(&coprime_dev)?;

        Ok((gcd_host, coprime_host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_gcd_correctness() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let batch = GpuGcdBatch::new(ctx).unwrap();

        let a = vec![12, 35, 17, 100, 7, 1];
        let b = vec![8,  25, 13, 75,  7, 1];
        // Expected GCDs: 4, 5, 1, 25, 7, 1
        // Expected coprime: 0, 0, 1, 0, 0, 1

        let (gcds, coprimes) = batch.compute_batch(&a, &b).unwrap();

        assert_eq!(gcds, vec![4, 5, 1, 25, 7, 1]);
        assert_eq!(coprimes, vec![0, 0, 1, 0, 0, 1]);
        println!("Batch GCD test passed: {:?}", gcds);
    }
}
