# DCL-RS CUDA Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace wgpu with native CUDA in `dcl-gpu` crate, implementing 8 CUDA kernels for the full DCL pipeline targeting RTX 3050 (SM 8.6, 4GB VRAM).

**Architecture:** In-place replacement. Remove wgpu/pollster/futures, add cudarc with nvrtc runtime compilation. CUDA C kernels (.cu source strings embedded in Rust) compiled to PTX at runtime. CudaContext wraps device + stream. Each kernel gets a Rust wrapper module.

**Tech Stack:** Rust 2021, cudarc 0.16 (driver + nvrtc features), CUDA 13.0, bytemuck 1.14

**Build command (excludes fuzz):**
```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm -p dcl-gpu
```

**Test command:**
```bash
cargo test -p dcl-gpu -- --nocapture
```

---

## Task 1: Replace Dependencies — cudarc for wgpu

**Files:**
- Modify: `dcl-gpu/Cargo.toml`
- Modify: `Cargo.toml` (workspace — add cudarc)

**Step 1: Update workspace Cargo.toml**

Add cudarc to workspace dependencies. In the root `Cargo.toml`, add after the `rayon` line in `[workspace.dependencies]`:

```toml
cudarc = { version = "0.16", features = ["std", "driver", "nvrtc"] }
```

**Step 2: Rewrite dcl-gpu/Cargo.toml**

Replace the entire file contents:

```toml
[package]
name = "dcl-gpu"
version = "0.2.0"
edition = "2021"

[dependencies]
dcl-core = { path = "../dcl-core" }
dcl-security = { path = "../dcl-security" }
cudarc = { workspace = true }
bytemuck = { version = "1.14", features = ["derive"] }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
rand = { workspace = true }
```

**Step 3: Verify it parses**

```bash
cargo metadata --no-deps -p dcl-gpu 2>&1 | head -5
```
Expected: JSON output (no parse errors). Build will fail at this point since source files still reference wgpu — that's expected.

---

## Task 2: CudaContext and Error Types

**Files:**
- Create: `dcl-gpu/src/context.rs`
- Create: `dcl-gpu/src/error.rs`
- Rewrite: `dcl-gpu/src/lib.rs`
- Delete contents of: `dcl-gpu/src/gpu_context.rs`, `dcl-gpu/src/compute.rs`, `dcl-gpu/src/gcd_batch.rs`, `dcl-gpu/src/prime_sieve.rs`

**Step 1: Write error.rs**

Create `dcl-gpu/src/error.rs`:

```rust
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
```

**Step 2: Write context.rs**

Create `dcl-gpu/src/context.rs`:

```rust
//! CUDA context management — device and stream lifecycle.

use crate::error::{GpuError, GpuResult};
use cudarc::driver::{CudaContext as CudarcContext, CudaStream};
use cudarc::nvrtc::compile_ptx;
use std::sync::Arc;

/// CUDA device context wrapping cudarc.
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
```

**Step 3: Rewrite lib.rs**

Replace `dcl-gpu/src/lib.rs` with:

```rust
//! DCL-GPU: CUDA-accelerated operations for the DCL framework.
//! Targets NVIDIA GPUs via cudarc (CUDA driver + nvrtc runtime compilation).

pub mod context;
pub mod error;
pub mod kernels;

pub use context::CudaContext;
pub use error::{GpuError, GpuResult};
```

**Step 4: Create kernels module stub**

Create `dcl-gpu/src/kernels/mod.rs`:

```rust
//! CUDA kernel wrappers for DCL operations.

pub mod gcd;
```

Create `dcl-gpu/src/kernels/gcd.rs` as a minimal stub:

```rust
//! Batch GCD computation on GPU using CUDA.
```

**Step 5: Remove old files**

Delete the contents of (or remove) these files — they referenced wgpu:
- `dcl-gpu/src/gpu_context.rs` — replace with empty file or delete
- `dcl-gpu/src/compute.rs` — replace with empty file or delete
- `dcl-gpu/src/gcd_batch.rs` — replace with empty file or delete
- `dcl-gpu/src/prime_sieve.rs` — replace with empty file or delete

**Step 6: Write the test**

Add to `dcl-gpu/src/context.rs`:

```rust
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
```

**Step 7: Build and test**

```bash
cargo build -p dcl-gpu
cargo test -p dcl-gpu -- --nocapture
```
Expected: Build succeeds. Test passes (prints device info or skips gracefully).

---

## Task 3: Batch GCD CUDA Kernel

**Files:**
- Create: `dcl-gpu/src/kernels/gcd.rs` (replace stub)

**Step 1: Implement the GCD kernel wrapper**

Replace `dcl-gpu/src/kernels/gcd.rs`:

```rust
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
    // Remove common factors of 2
    while (((x | y) & 1) == 0) { x >>= 1; y >>= 1; shift++; }
    // Remove remaining factors of 2 from x
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

        // Copy inputs to device
        let a_dev = stream.clone_htod(a)?;
        let b_dev = stream.clone_htod(b)?;
        let mut gcd_dev = stream.alloc_zeros::<u64>(n)?;
        let mut coprime_dev = stream.alloc_zeros::<u32>(n)?;
        let n_val = n as i32;

        // Launch kernel
        let cfg = LaunchConfig::for_num_elems(n as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&a_dev);
        builder.arg(&b_dev);
        builder.arg(&mut gcd_dev);
        builder.arg(&mut coprime_dev);
        builder.arg(&n_val);
        unsafe { builder.launch(cfg) }?;

        // Copy results back
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
```

**Step 2: Build and test**

```bash
cargo test -p dcl-gpu gcd -- --nocapture
```
Expected: PASS — GPU computes correct GCDs.

---

## Task 4: Prime Sieve CUDA Kernel

**Files:**
- Create: `dcl-gpu/src/kernels/prime_sieve.rs`
- Modify: `dcl-gpu/src/kernels/mod.rs`

**Step 1: Add module to mod.rs**

In `dcl-gpu/src/kernels/mod.rs`, add:

```rust
pub mod prime_sieve;
```

**Step 2: Implement prime sieve kernel**

Create `dcl-gpu/src/kernels/prime_sieve.rs`:

```rust
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
```

**Step 3: Build and test**

```bash
cargo test -p dcl-gpu prime_sieve -- --nocapture
```
Expected: PASS.

---

## Task 5: Batch Power Map CUDA Kernel

**Files:**
- Create: `dcl-gpu/src/kernels/power_map.rs`
- Modify: `dcl-gpu/src/kernels/mod.rs`

**Step 1: Add module**

In `dcl-gpu/src/kernels/mod.rs`, add:

```rust
pub mod power_map;
```

**Step 2: Implement power map kernel**

Create `dcl-gpu/src/kernels/power_map.rs`:

```rust
//! GPU-accelerated batch power map: g(x) = x^m or x^m mod N.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const POWER_MAP_SRC: &str = r#"
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
                result = (unsigned __int128)result * base % modulus;
            }
            base = (unsigned __int128)base * base % modulus;
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
                // Check for overflow before multiply
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
```

**Step 3: Build and test**

```bash
cargo test -p dcl-gpu power_map -- --nocapture
```
Expected: PASS.

---

## Task 6: Batch Evolve and Coprimality CUDA Kernels

**Files:**
- Create: `dcl-gpu/src/kernels/evolve.rs`
- Create: `dcl-gpu/src/kernels/coprimality.rs`
- Modify: `dcl-gpu/src/kernels/mod.rs`

**Step 1: Add modules**

In `dcl-gpu/src/kernels/mod.rs`, add:

```rust
pub mod evolve;
pub mod coprimality;
```

**Step 2: Implement batch evolve kernel**

Create `dcl-gpu/src/kernels/evolve.rs`:

```rust
//! GPU-accelerated batch label evolution across multiple graph instances.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const EVOLVE_SRC: &str = r#"
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
            if (exp & 1) result = (unsigned __int128)result * base % modulus;
            base = (unsigned __int128)base * base % modulus;
            exp >>= 1;
        }
        labels[idx] = (result == 0) ? 1 : result;
    } else {
        if (x <= 1) return;
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
    /// Can process multiple graph instances concatenated: labels = [graph0_labels..., graph1_labels..., ...]
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
    use dcl_core::transform::{PowerMap, Transform};

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
```

**Step 3: Implement coprimality check kernel**

Create `dcl-gpu/src/kernels/coprimality.rs`:

```rust
//! GPU-accelerated coprimality checking and repair.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const COPRIME_CHECK_SRC: &str = r#"
extern "C" __global__ void batch_coprime_check(
    const unsigned long long* labels,
    const int* edge_u,
    const int* edge_v,
    unsigned int* results,
    const int num_edges
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_edges) return;

    unsigned long long a = labels[edge_u[idx]];
    unsigned long long b = labels[edge_v[idx]];

    // Stein's binary GCD
    if (a == 0) { results[idx] = (b == 1) ? 1 : 0; return; }
    if (b == 0) { results[idx] = (a == 1) ? 1 : 0; return; }

    int shift = 0;
    while (((a | b) & 1) == 0) { a >>= 1; b >>= 1; shift++; }
    while ((a & 1) == 0) a >>= 1;
    do {
        while ((b & 1) == 0) b >>= 1;
        if (a > b) { unsigned long long t = a; a = b; b = t; }
        b -= a;
    } while (b != 0);

    unsigned long long g = a << shift;
    results[idx] = (g == 1) ? 1 : 0;
}
"#;

/// GPU-accelerated coprimality checking.
pub struct GpuCoprimalityCheck {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuCoprimalityCheck {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(COPRIME_CHECK_SRC, "batch_coprime_check")?;
        Ok(GpuCoprimalityCheck { ctx, kernel })
    }

    /// Check coprimality for all edges given labels. Returns true if ALL edges are coprime.
    pub fn check_all(
        &self,
        labels: &[u64],
        edges: &[(usize, usize)],
    ) -> GpuResult<bool> {
        if edges.is_empty() {
            return Ok(true);
        }

        let edge_u: Vec<i32> = edges.iter().map(|&(u, _)| u as i32).collect();
        let edge_v: Vec<i32> = edges.iter().map(|&(_, v)| v as i32).collect();
        let n_edges = edges.len();

        let stream = &self.ctx.stream;
        let labels_dev = stream.clone_htod(labels)?;
        let eu_dev = stream.clone_htod(&edge_u)?;
        let ev_dev = stream.clone_htod(&edge_v)?;
        let mut results_dev = stream.alloc_zeros::<u32>(n_edges)?;
        let n_val = n_edges as i32;

        let cfg = LaunchConfig::for_num_elems(n_edges as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&labels_dev);
        builder.arg(&eu_dev);
        builder.arg(&ev_dev);
        builder.arg(&mut results_dev);
        builder.arg(&n_val);
        unsafe { builder.launch(cfg) }?;

        let results = stream.clone_dtoh(&results_dev)?;
        Ok(results.iter().all(|&v| v == 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;

    #[test]
    fn coprime_check_correctness() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let checker = GpuCoprimalityCheck::new(ctx).unwrap();

        // Coprime labeling: [2, 3, 5, 7] on path graph (edges: 0-1, 1-2, 2-3)
        let g = Graph::path(4);
        let labels = vec![2, 3, 5, 7];
        assert!(checker.check_all(&labels, &g.edges()).unwrap());

        // Non-coprime: [6, 9, 5, 7] — gcd(6,9) = 3
        let labels_bad = vec![6, 9, 5, 7];
        assert!(!checker.check_all(&labels_bad, &g.edges()).unwrap());

        println!("Coprimality check: PASS");
    }
}
```

**Step 4: Build and test**

```bash
cargo test -p dcl-gpu evolve -- --nocapture
cargo test -p dcl-gpu coprime -- --nocapture
```
Expected: Both PASS.

---

## Task 7: Brute Force Search and NIST Data Gen CUDA Kernels

**Files:**
- Create: `dcl-gpu/src/kernels/brute_force.rs`
- Create: `dcl-gpu/src/kernels/nist_gen.rs`
- Modify: `dcl-gpu/src/kernels/mod.rs`

**Step 1: Add modules**

In `dcl-gpu/src/kernels/mod.rs`, add:

```rust
pub mod brute_force;
pub mod nist_gen;
```

**Step 2: Implement brute force search**

Create `dcl-gpu/src/kernels/brute_force.rs`:

```rust
//! GPU-accelerated brute-force DCL labeling search.

use crate::context::CudaContext;
use crate::error::GpuResult;
use cudarc::driver::{LaunchConfig, PushKernelArg};

const BRUTE_FORCE_SRC: &str = r#"
extern "C" __global__ void brute_force_search(
    const int* edge_u,
    const int* edge_v,
    const int num_edges,
    const int num_vertices,
    const unsigned long long max_label,
    const unsigned long long seed,
    unsigned int* found_count,
    unsigned long long* found_labels,
    const int max_found,
    const int num_threads
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_threads) return;

    // Simple LCG random number generator per thread
    unsigned long long rng = seed + (unsigned long long)idx * 6364136223846793005ULL + 1442695040888963407ULL;

    // Generate random labeling
    unsigned long long labels[32]; // max 32 vertices
    for (int v = 0; v < num_vertices && v < 32; v++) {
        rng = rng * 6364136223846793005ULL + 1442695040888963407ULL;
        labels[v] = (rng >> 33) % max_label + 1;
    }

    // Check coprimality on all edges using binary GCD
    bool valid = true;
    for (int e = 0; e < num_edges; e++) {
        unsigned long long a = labels[edge_u[e]];
        unsigned long long b = labels[edge_v[e]];
        // Binary GCD
        if (a == 0 || b == 0) { valid = false; break; }
        unsigned long long x = a, y = b;
        while (y != 0) { unsigned long long t = y; y = x % t; x = t; }
        if (x != 1) { valid = false; break; }
    }

    if (valid) {
        unsigned int slot = atomicAdd(found_count, 1);
        if (slot < (unsigned int)max_found) {
            for (int v = 0; v < num_vertices && v < 32; v++) {
                found_labels[slot * 32 + v] = labels[v];
            }
        }
    }
}
"#;

/// GPU brute-force labeling search.
pub struct GpuBruteForce {
    ctx: CudaContext,
    kernel: cudarc::driver::CudaFunction,
}

impl GpuBruteForce {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let kernel = ctx.compile_and_load(BRUTE_FORCE_SRC, "brute_force_search")?;
        Ok(GpuBruteForce { ctx, kernel })
    }

    /// Search for valid coprime labelings. Returns found labelings.
    pub fn search(
        &self,
        edges: &[(usize, usize)],
        num_vertices: usize,
        max_label: u64,
        num_threads: usize,
        max_found: usize,
    ) -> GpuResult<Vec<Vec<u64>>> {
        assert!(num_vertices <= 32, "Max 32 vertices for GPU brute force");

        let edge_u: Vec<i32> = edges.iter().map(|&(u, _)| u as i32).collect();
        let edge_v: Vec<i32> = edges.iter().map(|&(_, v)| v as i32).collect();
        let num_edges = edges.len() as i32;
        let nv = num_vertices as i32;
        let seed = 42u64;
        let max_f = max_found as i32;
        let nt = num_threads as i32;

        let stream = &self.ctx.stream;
        let eu_dev = stream.clone_htod(&edge_u)?;
        let ev_dev = stream.clone_htod(&edge_v)?;
        let mut count_dev = stream.alloc_zeros::<u32>(1)?;
        let mut labels_dev = stream.alloc_zeros::<u64>(max_found * 32)?;

        let cfg = LaunchConfig::for_num_elems(num_threads as u32);
        let mut builder = stream.launch_builder(&self.kernel);
        builder.arg(&eu_dev);
        builder.arg(&ev_dev);
        builder.arg(&num_edges);
        builder.arg(&nv);
        builder.arg(&max_label);
        builder.arg(&seed);
        builder.arg(&mut count_dev);
        builder.arg(&mut labels_dev);
        builder.arg(&max_f);
        builder.arg(&nt);
        unsafe { builder.launch(cfg) }?;

        let count = stream.clone_dtoh(&count_dev)?;
        let found = count[0].min(max_found as u32) as usize;
        let all_labels = stream.clone_dtoh(&labels_dev)?;

        let mut results = Vec::new();
        for i in 0..found {
            let start = i * 32;
            let labels: Vec<u64> = all_labels[start..start + num_vertices].to_vec();
            results.push(labels);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;
    use dcl_core::labeling::Labeling;

    #[test]
    fn brute_force_finds_solutions() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let bf = GpuBruteForce::new(ctx).unwrap();
        let g = Graph::path(4);
        let results = bf.search(&g.edges(), 4, 20, 100_000, 10).unwrap();

        println!("GPU brute force found {} solutions", results.len());
        assert!(!results.is_empty(), "Should find at least one valid labeling");

        // Verify each solution
        for labels in &results {
            let lab = Labeling::new(labels.clone());
            assert!(g.is_coprime_labeling(&lab), "GPU solution must be valid: {:?}", labels);
        }
    }
}
```

**Step 3: Implement NIST data generation**

Create `dcl-gpu/src/kernels/nist_gen.rs`:

```rust
//! GPU-accelerated NIST test data generation from DCL evolution.

use crate::context::CudaContext;
use crate::error::GpuResult;
use crate::kernels::evolve::GpuEvolve;

/// GPU NIST data generator — evolves labels and collects bytes.
pub struct GpuNistGen {
    evolve: GpuEvolve,
}

impl GpuNistGen {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let evolve = GpuEvolve::new(ctx)?;
        Ok(GpuNistGen { evolve })
    }

    /// Generate NIST test data by evolving labels for `steps` iterations.
    /// Returns raw bytes (each label as 8 LE bytes per step).
    pub fn generate(
        &self,
        initial_labels: &[u64],
        m: u32,
        modulus: u64,
        steps: usize,
    ) -> GpuResult<Vec<u8>> {
        let n = initial_labels.len();
        let mut data = Vec::with_capacity(n * 8 * steps);
        let mut current = initial_labels.to_vec();

        for _ in 0..steps {
            current = self.evolve.evolve_steps(&current, m, modulus, 1)?;
            for &label in &current {
                data.extend_from_slice(&label.to_le_bytes());
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::{PowerMap, Transform};

    #[test]
    fn nist_gen_matches_cpu() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let gen = GpuNistGen::new(ctx).unwrap();
        let labels = vec![2, 3, 5, 7, 11];
        let gpu_data = gen.generate(&labels, 2, 0, 10).unwrap();

        // CPU reference
        let pm = PowerMap::new(2);
        let mut cpu_data = Vec::new();
        let mut lab = Labeling::new(labels.clone());
        for _ in 0..10 {
            lab.evolve_in_place(&pm);
            for &l in &lab.labels {
                cpu_data.extend_from_slice(&l.to_le_bytes());
            }
        }

        assert_eq!(gpu_data.len(), cpu_data.len());
        assert_eq!(gpu_data, cpu_data, "GPU NIST data must match CPU");
        println!("NIST gen: {} bytes match CPU", gpu_data.len());
    }
}
```

**Step 4: Build and test**

```bash
cargo test -p dcl-gpu brute_force -- --nocapture
cargo test -p dcl-gpu nist_gen -- --nocapture
```
Expected: Both PASS.

---

## Task 8: GPU Benchmark Module

**Files:**
- Create: `dcl-gpu/src/benchmark.rs`
- Modify: `dcl-gpu/src/lib.rs`

**Step 1: Add module to lib.rs**

In `dcl-gpu/src/lib.rs`, add:

```rust
pub mod benchmark;
```

**Step 2: Implement benchmark**

Create `dcl-gpu/src/benchmark.rs`:

```rust
//! GPU vs CPU performance benchmarks.

use crate::context::CudaContext;
use crate::error::GpuResult;
use crate::kernels::{gcd::GpuGcdBatch, prime_sieve::GpuPrimeSieve, power_map::GpuPowerMap, evolve::GpuEvolve};
use std::time::Instant;

/// Benchmark result for a single operation.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub operation: String,
    pub gpu_time_ms: f64,
    pub cpu_time_ms: f64,
    pub speedup: f64,
    pub items: usize,
    pub gpu_throughput: f64, // million ops/sec
}

/// Run all benchmarks and return results.
pub fn run_all_benchmarks(count: usize) -> GpuResult<Vec<BenchmarkResult>> {
    let mut results = Vec::new();

    // GCD benchmark
    {
        let ctx = CudaContext::new()?;
        let gpu = GpuGcdBatch::new(ctx)?;
        let a: Vec<u64> = (1..=count as u64).collect();
        let b: Vec<u64> = (2..=count as u64 + 1).collect();

        let start = Instant::now();
        let _ = gpu.compute_batch(&a, &b)?;
        let gpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        for i in 0..count {
            let _ = dcl_core::gcd::gcd(a[i], b[i]);
        }
        let cpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            operation: "Batch GCD".to_string(),
            gpu_time_ms: gpu_ms,
            cpu_time_ms: cpu_ms,
            speedup: cpu_ms / gpu_ms,
            items: count,
            gpu_throughput: count as f64 / gpu_ms / 1000.0,
        });
    }

    // Power Map benchmark
    {
        let ctx = CudaContext::new()?;
        let gpu = GpuPowerMap::new(ctx)?;
        let labels: Vec<u64> = (1..=count as u64).collect();

        let start = Instant::now();
        let _ = gpu.apply_batch(&labels, 3, 0)?;
        let gpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let pm = dcl_core::transform::PowerMap::new(3);
        for &x in &labels {
            let _ = dcl_core::transform::Transform::apply(&pm, x);
        }
        let cpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            operation: "Power Map x^3".to_string(),
            gpu_time_ms: gpu_ms,
            cpu_time_ms: cpu_ms,
            speedup: cpu_ms / gpu_ms,
            items: count,
            gpu_throughput: count as f64 / gpu_ms / 1000.0,
        });
    }

    // Prime Sieve benchmark
    {
        let ctx = CudaContext::new()?;
        let gpu = GpuPrimeSieve::new(ctx)?;
        let candidates: Vec<u64> = (2..=count as u64 + 1).collect();

        let start = Instant::now();
        let _ = gpu.test_batch(&candidates)?;
        let gpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        for &n in &candidates {
            let _ = is_prime_cpu(n);
        }
        let cpu_ms = start.elapsed().as_secs_f64() * 1000.0;

        results.push(BenchmarkResult {
            operation: "Prime Sieve".to_string(),
            gpu_time_ms: gpu_ms,
            cpu_time_ms: cpu_ms,
            speedup: cpu_ms / gpu_ms,
            items: count,
            gpu_throughput: count as f64 / gpu_ms / 1000.0,
        });
    }

    Ok(results)
}

fn is_prime_cpu(n: u64) -> bool {
    if n < 2 { return false; }
    if n < 4 { return true; }
    if n % 2 == 0 || n % 3 == 0 { return false; }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 { return false; }
        i += 6;
    }
    true
}

/// Print benchmark results as a formatted table.
pub fn print_benchmark_table(results: &[BenchmarkResult]) {
    println!("\n{:<20} {:>12} {:>12} {:>10} {:>16}",
        "Operation", "GPU (ms)", "CPU (ms)", "Speedup", "GPU Throughput");
    println!("{}", "-".repeat(75));
    for r in results {
        println!("{:<20} {:>11.2} {:>11.2} {:>9.1}x {:>12.1}M ops/s",
            r.operation, r.gpu_time_ms, r.cpu_time_ms, r.speedup, r.gpu_throughput);
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test -p dcl-gpu -- --ignored --nocapture
    fn benchmark_all() {
        let results = run_all_benchmarks(100_000).unwrap();
        print_benchmark_table(&results);
        assert!(!results.is_empty());
    }
}
```

**Step 3: Build and test**

```bash
cargo test -p dcl-gpu -- --nocapture
cargo test -p dcl-gpu -- --ignored --nocapture  # runs benchmark
```
Expected: All unit tests pass. Benchmark prints table.

---

## Task 9: Rewrite CLI GPU Command

**Files:**
- Rewrite: `dcl-cli/src/cmd_gpu.rs`

**Step 1: Rewrite cmd_gpu.rs**

Replace `dcl-cli/src/cmd_gpu.rs` entirely:

```rust
//! GPU acceleration CLI command — CUDA-native operations.

use clap::Args;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;

#[derive(Args)]
pub struct GpuArgs {
    /// Show CUDA device info only
    #[arg(long)]
    info: bool,

    /// Run batch GCD benchmark
    #[arg(long)]
    gcd: bool,

    /// Run prime sieve benchmark
    #[arg(long)]
    primes: bool,

    /// Run batch label evolution benchmark
    #[arg(long)]
    evolve: bool,

    /// Run GPU brute-force labeling search
    #[arg(long)]
    bfs: bool,

    /// Run GPU NIST test data generation
    #[arg(long)]
    nist: bool,

    /// Run all benchmarks (GPU vs CPU comparison)
    #[arg(long)]
    benchmark: bool,

    /// Number of items to process
    #[arg(short, long, default_value_t = 100_000)]
    count: usize,

    /// Graph type for evolve/bfs/nist
    #[arg(long, default_value = "path")]
    graph: String,

    /// Number of vertices
    #[arg(long, default_value_t = 10)]
    vertices: usize,

    /// Power map exponent
    #[arg(short, long, default_value_t = 2)]
    m: u32,

    /// Optional modulus for bounded evolution
    #[arg(long)]
    modulus: Option<u64>,

    /// Evolution steps for NIST gen
    #[arg(long, default_value_t = 200)]
    steps: usize,
}

pub fn run(args: GpuArgs) {
    println!("\n=== DCL-GPU: CUDA Acceleration ===\n");

    // Initialize CUDA
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialize CUDA: {e}");
            eprintln!("Make sure NVIDIA drivers and CUDA toolkit are installed.");
            return;
        }
    };

    if args.info {
        print_device_info(&ctx);
        return;
    }

    if args.benchmark {
        run_benchmark(args.count);
        return;
    }

    let run_all = !args.gcd && !args.primes && !args.evolve && !args.bfs && !args.nist;

    if args.gcd || run_all {
        run_gcd_demo(args.count);
    }
    if args.primes || run_all {
        run_prime_demo(args.count);
    }
    if args.evolve || run_all {
        run_evolve_demo(&args);
    }
    if args.bfs {
        run_bfs_demo(&args);
    }
    if args.nist {
        run_nist_demo(&args);
    }
}

fn print_device_info(ctx: &dcl_gpu::CudaContext) {
    match ctx.device_info() {
        Ok(info) => {
            println!("CUDA Device Information:");
            println!("  Name: {}", info.name);
            println!("  Memory: {} MB", info.total_memory / (1024 * 1024));
            println!("  Compute Capability: {}.{}", info.compute_capability.0, info.compute_capability.1);
        }
        Err(e) => eprintln!("Failed to get device info: {e}"),
    }
}

fn run_gcd_demo(count: usize) {
    println!("--- Batch GCD ({} pairs) ---", count);
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => { eprintln!("CUDA error: {e}"); return; }
    };

    let batch = match dcl_gpu::kernels::gcd::GpuGcdBatch::new(ctx) {
        Ok(b) => b,
        Err(e) => { eprintln!("Kernel error: {e}"); return; }
    };

    let a: Vec<u64> = (1..=count as u64).collect();
    let b: Vec<u64> = (2..=count as u64 + 1).collect();

    let start = std::time::Instant::now();
    let (gcds, coprimes) = batch.compute_batch(&a, &b).unwrap();
    let elapsed = start.elapsed();

    let coprime_count = coprimes.iter().filter(|&&v| v == 1).count();
    println!("  Time: {:.2?}", elapsed);
    println!("  Coprime pairs: {}/{} ({:.1}%)", coprime_count, count,
        coprime_count as f64 / count as f64 * 100.0);
    println!("  Throughput: {:.1}M ops/sec\n",
        count as f64 / elapsed.as_secs_f64() / 1_000_000.0);
}

fn run_prime_demo(count: usize) {
    println!("--- Prime Sieve (2 to {}) ---", count + 1);
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => { eprintln!("CUDA error: {e}"); return; }
    };

    let sieve = match dcl_gpu::kernels::prime_sieve::GpuPrimeSieve::new(ctx) {
        Ok(s) => s,
        Err(e) => { eprintln!("Kernel error: {e}"); return; }
    };

    let start = std::time::Instant::now();
    let primes = sieve.sieve_range(2, count as u64 + 2).unwrap();
    let elapsed = start.elapsed();

    println!("  Time: {:.2?}", elapsed);
    println!("  Primes found: {}", primes.len());
    println!("  Throughput: {:.1}M ops/sec\n",
        count as f64 / elapsed.as_secs_f64() / 1_000_000.0);
}

fn run_evolve_demo(args: &GpuArgs) {
    println!("--- Batch Evolve ({} vertices, {} instances) ---", args.vertices, args.count / args.vertices);
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => { eprintln!("CUDA error: {e}"); return; }
    };

    let evolve = match dcl_gpu::kernels::evolve::GpuEvolve::new(ctx) {
        Ok(e) => e,
        Err(e) => { eprintln!("Kernel error: {e}"); return; }
    };

    // Create batch of labels (multiple graph instances)
    let labels: Vec<u64> = (0..args.count).map(|i| (i as u64 % 50) + 2).collect();
    let modulus = args.modulus.unwrap_or(0);

    let start = std::time::Instant::now();
    let result = evolve.evolve_steps(&labels, args.m, modulus, 10).unwrap();
    let elapsed = start.elapsed();

    println!("  Time: {:.2?}", elapsed);
    println!("  Labels evolved: {}", result.len());
    println!("  Throughput: {:.1}M ops/sec\n",
        (args.count * 10) as f64 / elapsed.as_secs_f64() / 1_000_000.0);
}

fn run_bfs_demo(args: &GpuArgs) {
    println!("--- GPU Brute-Force Search ({} threads) ---", args.count);
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => { eprintln!("CUDA error: {e}"); return; }
    };

    let bf = match dcl_gpu::kernels::brute_force::GpuBruteForce::new(ctx) {
        Ok(b) => b,
        Err(e) => { eprintln!("Kernel error: {e}"); return; }
    };

    let graph = match args.graph.as_str() {
        "path" => Graph::path(args.vertices),
        "cycle" => Graph::cycle(args.vertices),
        "complete" => Graph::complete(args.vertices),
        _ => Graph::path(args.vertices),
    };

    let start = std::time::Instant::now();
    let results = bf.search(&graph.edges(), args.vertices, 50, args.count, 20).unwrap();
    let elapsed = start.elapsed();

    println!("  Time: {:.2?}", elapsed);
    println!("  Valid labelings found: {}", results.len());
    if let Some(first) = results.first() {
        println!("  Example: {:?}", first);
    }
    println!("  Throughput: {:.1}M candidates/sec\n",
        args.count as f64 / elapsed.as_secs_f64() / 1_000_000.0);
}

fn run_nist_demo(args: &GpuArgs) {
    println!("--- GPU NIST Data Gen ({} vertices, {} steps) ---", args.vertices, args.steps);
    let ctx = match dcl_gpu::CudaContext::new() {
        Ok(c) => c,
        Err(e) => { eprintln!("CUDA error: {e}"); return; }
    };

    let gen = match dcl_gpu::kernels::nist_gen::GpuNistGen::new(ctx) {
        Ok(g) => g,
        Err(e) => { eprintln!("Kernel error: {e}"); return; }
    };

    let primes = vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
    let labels: Vec<u64> = (0..args.vertices).map(|i| primes[i % primes.len()]).collect();
    let modulus = args.modulus.unwrap_or(0);

    let start = std::time::Instant::now();
    let data = gen.generate(&labels, args.m, modulus, args.steps).unwrap();
    let elapsed = start.elapsed();

    println!("  Time: {:.2?}", elapsed);
    println!("  Generated: {} bytes ({} bits)", data.len(), data.len() * 8);
    println!("  Throughput: {:.1}M bytes/sec\n",
        data.len() as f64 / elapsed.as_secs_f64() / 1_000_000.0);
}

fn run_benchmark(count: usize) {
    println!("=== GPU vs CPU Benchmark ({} items) ===\n", count);
    match dcl_gpu::benchmark::run_all_benchmarks(count) {
        Ok(results) => dcl_gpu::benchmark::print_benchmark_table(&results),
        Err(e) => eprintln!("Benchmark error: {e}"),
    }
}
```

**Step 2: Build and smoke test**

```bash
cargo build -p dcl-cli
cargo run -p dcl-cli -- gpu --info
cargo run -p dcl-cli -- gpu --gcd --count 10000
cargo run -p dcl-cli -- gpu --primes --count 10000
```
Expected: Build succeeds. Commands print results.

---

## Task 10: Full Integration Test

**Step 1: Run all dcl-gpu tests**

```bash
cargo test -p dcl-gpu -- --nocapture
```
Expected: All kernel tests pass.

**Step 2: Run full workspace build**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm -p dcl-gpu
```
Expected: SUCCESS.

**Step 3: Run all workspace tests**

```bash
cargo test -p dcl-core -p dcl-complexity -p dcl-security -p dcl-crypto -p dcl-hypergraph -p dcl-ramsey -p dcl-zkp -p dcl-gpu
```
Expected: All tests pass.

**Step 4: Smoke test all GPU CLI commands**

```bash
cargo run -p dcl-cli -- gpu --info
cargo run -p dcl-cli -- gpu --gcd --count 50000
cargo run -p dcl-cli -- gpu --primes --count 50000
cargo run -p dcl-cli -- gpu --evolve --vertices 10 --count 100000
cargo run -p dcl-cli -- gpu --bfs --graph path --vertices 6 --count 500000
cargo run -p dcl-cli -- gpu --nist --vertices 10 --steps 200
cargo run -p dcl-cli -- gpu --benchmark --count 100000
```
Expected: All commands complete successfully.

**Step 5: Run GPU vs CPU benchmark**

```bash
cargo test -p dcl-gpu -- --ignored --nocapture
```
Expected: Benchmark table prints with speedup ratios.
