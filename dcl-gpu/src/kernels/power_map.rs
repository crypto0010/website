//! GPU-accelerated batch power map: g(x) = x^m or x^m mod N.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const POWER_MAP_SRC: &str = r#"
// Multiply a * b mod m using repeated-addition doubling (no __int128 needed).
__device__ unsigned long long mulmod(unsigned long long a, unsigned long long b, unsigned long long m) {
    unsigned long long result = 0;
    a %= m;
    while (b > 0) {
        if (b & 1) {
            result = (result + a) % m;
        }
        a = (a + a) % m;
        b >>= 1;
    }
    return result;
}

extern "C" __global__ void batch_power_map(
    const unsigned long long* labels_in,
    unsigned long long* labels_out,
    const unsigned int m,
    const unsigned long long modulus,
    const int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;

    unsigned long long x = labels_in[idx];

    if (modulus > 0) {
        // Modular binary exponentiation: x^m mod N
        unsigned long long result = 1;
        unsigned long long base = x % modulus;
        unsigned int exp = m;
        while (exp > 0) {
            if (exp & 1) {
                result = mulmod(result, base, modulus);
            }
            base = mulmod(base, base, modulus);
            exp >>= 1;
        }
        labels_out[idx] = (result == 0) ? 1 : result;
    } else {
        // Unbounded binary exponentiation with saturation
        if (x <= 1) { labels_out[idx] = x; return; }
        unsigned long long result = 1;
        unsigned long long base = x;
        unsigned int exp = m;
        unsigned long long limit = 0xFFFFFFFFFFFFFFFFULL;
        while (exp > 0) {
            if (exp & 1) {
                if (result > limit / base) {
                    labels_out[idx] = limit;
                    return;
                }
                result *= base;
            }
            exp >>= 1;
            if (exp > 0) {
                if (base > limit / base) {
                    labels_out[idx] = limit;
                    return;
                }
                base *= base;
            }
        }
        labels_out[idx] = result;
    }
}
"#;

/// GPU-accelerated batch power map.
pub struct GpuPowerMap {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuPowerMap {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(POWER_MAP_SRC, "batch_power_map")?;
        Ok(GpuPowerMap { ctx, kernel })
    }

    /// Apply power map to all labels. modulus=0 means unbounded.
    pub fn apply_batch(
        &self,
        labels: &[u64],
        m: u32,
        modulus: u64,
    ) -> GpuResult<Vec<u64>> {
        let n = labels.len();
        if n == 0 {
            return Ok(vec![]);
        }

        let stream = &self.ctx.stream;
        let in_dev = stream.clone_htod(labels)?;
        let mut out_dev = stream.alloc_zeros::<u64>(n)?;
        let n_val = n as i32;

        let cfg = LaunchConfig::for_num_elems(n as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&in_dev);
        builder.arg(&mut out_dev);
        builder.arg(&m);
        builder.arg(&modulus);
        builder.arg(&n_val);
        unsafe { builder.launch(cfg) }?;

        let out_host = stream.clone_dtoh(&out_dev)?;
        Ok(out_host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::transform::{PowerMap, Transform};

    #[test]
    fn power_map_matches_cpu() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let gpu = GpuPowerMap::new(ctx).unwrap();
        let labels = vec![2, 3, 5, 7, 10, 100];

        // Test unbounded (modulus=0)
        let gpu_result = gpu.apply_batch(&labels, 3, 0).unwrap();
        let cpu_pm = PowerMap::new(3);
        let cpu_result: Vec<u64> = labels.iter().map(|&x| cpu_pm.apply(x)).collect();
        assert_eq!(gpu_result, cpu_result, "GPU must match CPU for unbounded power map");

        // Test modular (modulus=65537)
        let gpu_mod = gpu.apply_batch(&labels, 2, 65537).unwrap();
        let cpu_mod_pm = PowerMap::with_modulus(2, 65537);
        let cpu_mod: Vec<u64> = labels.iter().map(|&x| cpu_mod_pm.apply(x)).collect();
        assert_eq!(gpu_mod, cpu_mod, "GPU must match CPU for modular power map");

        println!("Power map GPU vs CPU: MATCH");
    }
}
