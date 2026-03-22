//! GPU-accelerated prime sieve using CUDA.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const PRIME_SIEVE_SRC: &str = r#"
extern "C" __global__ void prime_sieve(
    const unsigned long long* candidates,
    unsigned int* is_prime,
    const int n
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;

    unsigned long long num = candidates[idx];
    if (num < 2) { is_prime[idx] = 0; return; }
    if (num == 2 || num == 3) { is_prime[idx] = 1; return; }
    if (num % 2 == 0 || num % 3 == 0) { is_prime[idx] = 0; return; }

    unsigned long long i = 5;
    while (i * i <= num) {
        if (num % i == 0 || num % (i + 2) == 0) {
            is_prime[idx] = 0;
            return;
        }
        i += 6;
    }
    is_prime[idx] = 1;
}
"#;

/// GPU-accelerated prime testing.
pub struct GpuPrimeSieve {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuPrimeSieve {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(PRIME_SIEVE_SRC, "prime_sieve")?;
        Ok(GpuPrimeSieve { ctx, kernel })
    }

    /// Test primality for each candidate. Returns Vec<bool>.
    pub fn test_batch(&self, candidates: &[u64]) -> GpuResult<Vec<bool>> {
        let n = candidates.len();
        if n == 0 {
            return Ok(vec![]);
        }

        let stream = &self.ctx.stream;
        let cand_dev = stream.clone_htod(candidates)?;
        let mut result_dev = stream.alloc_zeros::<u32>(n)?;
        let n_val = n as i32;

        let cfg = LaunchConfig::for_num_elems(n as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&cand_dev);
        builder.arg(&mut result_dev);
        builder.arg(&n_val);
        unsafe { builder.launch(cfg) }?;

        let result_host = stream.clone_dtoh(&result_dev)?;
        Ok(result_host.iter().map(|&v| v == 1).collect())
    }

    /// Find all primes in range [start, end).
    pub fn sieve_range(&self, start: u64, end: u64) -> GpuResult<Vec<u64>> {
        let candidates: Vec<u64> = (start..end).collect();
        let flags = self.test_batch(&candidates)?;
        Ok(candidates
            .into_iter()
            .zip(flags)
            .filter_map(|(n, is_p)| if is_p { Some(n) } else { None })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_sieve_correctness() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let sieve = GpuPrimeSieve::new(ctx).unwrap();
        let primes = sieve.sieve_range(2, 31).unwrap();
        assert_eq!(primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
        println!("Prime sieve test passed: {:?}", primes);
    }
}
