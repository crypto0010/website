//! GPU-accelerated batch label evolution across multiple graph instances.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const EVOLVE_SRC: &str = r#"
// Modular multiplication without __int128 (portable across Windows/Linux NVRTC).
// Uses the Russian peasant / binary multiplication approach: a * b mod m.
__device__ unsigned long long mulmod(unsigned long long a, unsigned long long b, unsigned long long m) {
    unsigned long long result = 0;
    a %= m;
    while (b > 0) {
        if (b & 1) {
            result = (result + a) % m;
        }
        a = (a * 2) % m;
        b >>= 1;
    }
    return result;
}

extern "C" __global__ void batch_evolve(
    unsigned long long* labels,
    const unsigned int m,
    const unsigned long long modulus,
    const int num_labels
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_labels) return;

    unsigned long long x = labels[idx];

    if (modulus > 0) {
        unsigned long long result = 1;
        unsigned long long base = x % modulus;
        unsigned int exp = m;
        while (exp > 0) {
            if (exp & 1) result = mulmod(result, base, modulus);
            base = mulmod(base, base, modulus);
            exp >>= 1;
        }
        labels[idx] = (result == 0) ? 1 : result;
    } else {
        if (x <= 1) { return; }
        unsigned long long result = 1;
        unsigned long long base = x;
        unsigned int exp = m;
        unsigned long long limit = 0xFFFFFFFFFFFFFFFFULL;
        while (exp > 0) {
            if (exp & 1) {
                if (result > limit / base) { labels[idx] = limit; return; }
                result *= base;
            }
            exp >>= 1;
            if (exp > 0) {
                if (base > limit / base) { labels[idx] = limit; return; }
                base *= base;
            }
        }
        labels[idx] = result;
    }
}
"#;

/// GPU-accelerated batch label evolution (in-place).
pub struct GpuEvolve {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuEvolve {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(EVOLVE_SRC, "batch_evolve")?;
        Ok(GpuEvolve { ctx, kernel })
    }

    /// Evolve labels in-place on GPU for `steps` iterations.
    pub fn evolve_steps(
        &self,
        labels: &[u64],
        m: u32,
        modulus: u64,
        steps: usize,
    ) -> GpuResult<Vec<u64>> {
        let n = labels.len();
        if n == 0 {
            return Ok(vec![]);
        }

        let stream = &self.ctx.stream;
        let mut dev = stream.clone_htod(labels)?;
        let n_val = n as i32;
        let cfg = LaunchConfig::for_num_elems(n as u32);

        for _ in 0..steps {
            let mut builder = stream.launch_builder(&self.kernel);
            builder.arg(&mut dev);
            builder.arg(&m);
            builder.arg(&modulus);
            builder.arg(&n_val);
            unsafe { builder.launch(cfg) }?;
        }

        let result = stream.clone_dtoh(&dev)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::PowerMap;

    #[test]
    fn evolve_matches_cpu() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let gpu = GpuEvolve::new(ctx).unwrap();
        let labels = vec![2, 3, 5, 7, 11];
        let steps = 3;

        // GPU evolve
        let gpu_result = gpu.evolve_steps(&labels, 2, 0, steps).unwrap();

        // CPU evolve
        let pm = PowerMap::new(2);
        let mut lab = Labeling::new(labels.clone());
        for _ in 0..steps {
            lab.evolve_in_place(&pm);
        }

        assert_eq!(gpu_result, lab.labels, "GPU evolve must match CPU evolve");
        println!("Evolve GPU vs CPU after {} steps: MATCH", steps);
    }
}
