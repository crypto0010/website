# DCL-RS CUDA Implementation Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create the implementation plan.

**Goal:** Replace wgpu-based GPU acceleration with native CUDA for maximum performance on NVIDIA RTX 3050 (4GB VRAM, SM 8.6, 2048 CUDA cores). Implement full GPU pipeline covering all DCL operations.

**Approach:** In-place replacement of `dcl-gpu` crate internals. Remove wgpu, add `cudarc` + CUDA C kernels compiled via `nvrtc`. 8 CUDA kernels covering GCD, primes, power map, evolution, coprimality, brute-force search, and NIST data generation.

**Hardware Target:** NVIDIA GeForce RTX 3050 Laptop, 4GB GDDR6, Ampere SM 8.6, CUDA 13.0, Driver 591.74.

---

## Section 1: CUDA Kernel Inventory

8 kernels covering the full DCL pipeline:

| # | Kernel | Input | Output | Parallelism |
|---|--------|-------|--------|-------------|
| 1 | `batch_gcd` | Pairs of u64 | GCD results + coprime flags | 1 thread per pair |
| 2 | `prime_sieve` | Range [lo, hi] | Primality flags | 1 thread per candidate |
| 3 | `batch_power_map` | Labels + exponent m + optional modulus | Evolved labels | 1 thread per label |
| 4 | `batch_evolve` | N graphs x labels x transform | Evolved labelings | 1 thread per vertex per graph |
| 5 | `batch_coprimality_check` | Labels + edge list | Pass/fail per edge | 1 thread per edge |
| 6 | `batch_coprimality_repair` | Labels + edge list | Repaired labels + repair count | 1 thread per edge (iterative) |
| 7 | `brute_force_search` | Graph edges + max_label + N random seeds | Valid labelings found | 1 thread per candidate |
| 8 | `nist_data_gen` | Initial labels + transform params + steps | Byte stream for NIST | 1 thread per graph instance |

All kernels use native u64 arithmetic. Shared memory for edge lists (small graphs fit in 48KB).

---

## Section 2: Crate Architecture & Dependencies

### Dependencies (dcl-gpu/Cargo.toml)

Remove: `wgpu`, `pollster`, `futures`

Add:
```toml
cudarc = { version = "0.16", features = ["std", "driver", "nvrtc"] }
```

Keep: `bytemuck`, `dcl-core`, `thiserror`, `tracing`, `rand` (dev)

Add: `dcl-security` (for CryptanalysisConfig types)

### Module Layout

```
dcl-gpu/src/
  lib.rs              # Public API
  context.rs          # CudaContext (device, stream management)
  kernels/
    mod.rs
    gcd.rs            # batch_gcd wrapper
    prime_sieve.rs    # prime_sieve wrapper
    power_map.rs      # batch_power_map wrapper
    evolve.rs         # batch_evolve wrapper
    coprimality.rs    # check + repair wrappers
    brute_force.rs    # brute_force_search wrapper
    nist_gen.rs       # nist_data_gen wrapper
  cuda/
    gcd.cu            # CUDA C kernel sources
    prime_sieve.cu
    power_map.cu
    evolve.cu
    coprimality.cu
    brute_force.cu
    nist_gen.cu
  benchmark.rs        # GPU vs CPU comparison
```

### Build Approach

Use `cudarc`'s `nvrtc` to compile .cu files to PTX at runtime. No build.rs complexity. PTX cached after first compilation.

---

## Section 3: CudaContext & Memory Management

### CudaContext

```rust
pub struct CudaContext {
    device: Arc<CudaDevice>,
    stream: CudaStream,
}
```

Replaces current `GpuContext` (wgpu). Single entry point for all GPU operations.

### Memory Budget (RTX 3050, 4GB VRAM)

| Budget | Allocation |
|--------|-----------|
| Kernel code + PTX cache | ~50 MB |
| Working buffers | ~3.5 GB available |
| Max graph size | ~10M edges (~160 MB) |
| Typical workload (n<=1000) | < 100 MB total |

### Buffer Strategy

- Device buffers (`CudaSlice<u64>`) allocated once, reused across calls
- Pinned host memory for async transfers
- Shared memory (48KB/block) for edge lists in small graphs (up to ~3000 edges)

### Launch Configuration

- Block size: 256 threads
- Grid size: ceil(N / 256)
- N = number of work items per kernel

### Error Handling

`GpuError` enum: `DeviceNotFound`, `OutOfMemory`, `KernelLaunchFailed`, `CompilationFailed`. Wraps `cudarc::DriverError`.

---

## Section 4: CUDA Kernel Designs

### Kernel 1: batch_gcd
Binary GCD (Stein's algorithm) in u64. 1 thread per pair. Same algorithm as CPU `dcl_core::gcd::gcd()`.

### Kernel 2: prime_sieve
Trial division. 1 thread per candidate. Tests n%2, n%3, then i=5, i+=6 loop up to sqrt(n).

### Kernel 3: batch_power_map
Binary exponentiation. Each thread computes x^m (or x^m mod N). Same algorithm as CPU `PowerMap::apply()`. If modulus > 0: modular. Else: saturating with u64::MAX cap.

### Kernel 4: batch_evolve
Evolves multiple graph instances in parallel. Each thread handles one vertex of one graph. For scaling: 1000 instances x 10 vertices = 10K threads. Edge list in shared memory.

### Kernel 5: batch_coprimality_check
1 thread per edge. Computes GCD via Stein's, writes pass/fail. Block-level reduction to single pass/fail per graph.

### Kernel 6: batch_coprimality_repair
Iterative: each pass checks all edges, increments violating labels. Loop on host until zero violations (1-3 passes typical). Uses atomicAdd for repair counter.

### Kernel 7: brute_force_search
Each thread generates random labeling (thread ID + seed RNG), checks coprimality for all edges. Launch 100K+ threads. atomicAdd for success count.

### Kernel 8: nist_data_gen
Each thread evolves one graph for T steps sequentially, writes label bytes. Parallelism across graph instances for statistical sampling.

### Shared Memory Optimization
All kernels load edge list into `__shared__` memory once per block since all threads reference the same graph structure.

---

## Section 5: CLI Interface

Replace existing `gpu` subcommand with CUDA-native operations:

```
dcl-cli gpu [OPTIONS]

OPTIONS:
    --info                  CUDA device info
    --gcd                   Batch GCD benchmark
    --primes                Prime sieve benchmark
    --evolve                Batch label evolution benchmark
    --cryptanalysis         GPU cryptanalysis suite
    --bfs                   GPU brute-force labeling search
    --nist                  GPU NIST test data generation
    --benchmark             All benchmarks with GPU vs CPU comparison
    -n, --count <N>         Items to process (default: 100000)
    --graph <TYPE>          Graph type (default: path)
    --vertices <N>          Vertex count (default: 10)
    -m, --exp <M>           Power map exponent (default: 2)
    --modulus <N>           Optional modulus
    --steps <T>             Evolution steps for NIST (default: 200)
```

Output: Table with GPU time, CPU time, speedup ratio, throughput.

---

## Section 6: Integration with Existing Crates

### dcl-security integration
- `Cryptanalyzer::with_cuda(graph, config, cuda_ctx)` constructor
- GPU dispatch for brute_force, collision, statistical tests
- `dcl_labels_to_test_data_gpu()` variant
- Scaling benchmark uses GPU when available

### dcl-complexity integration
- Hybrid: CPU generates BFS candidates, GPU batch-checks coprimality
- Not full GPU BFS (irregular branching maps poorly to SIMT)

### dcl-core stays CPU-only
Reference implementation. GPU code in dcl-gpu calls dcl-core types.

### Dependency direction
```
dcl-gpu -> dcl-core (types)
dcl-gpu -> dcl-security (CryptanalysisConfig)
dcl-cli -> dcl-gpu (CLI)
```

No circular dependencies.

---

## Section 7: Testing Strategy

### Unit tests (per kernel)
Compare GPU output vs CPU reference from dcl-core. Small inputs (100-1000 items).

### Correctness tests
Random inputs, GPU vs CPU cross-validation. Edge cases: u64::MAX, zero, modulus=2, exponent=1.

### Benchmark tests
GPU vs CPU timing. Marked `#[ignore]`, run with `cargo test -p dcl-gpu -- --ignored`.

### Integration test
Full pipeline: graph -> GPU evolve -> GPU coprimality -> GPU cryptanalysis -> compare with CPU.

### Fallback
No CUDA device: tests skipped, `CudaContext::new()` returns `Err(GpuError::DeviceNotFound)`.

---

## Files Changed Summary

| Crate | File | Action |
|-------|------|--------|
| dcl-gpu | Cargo.toml | Replace wgpu with cudarc |
| dcl-gpu | src/lib.rs | New public API |
| dcl-gpu | src/context.rs | CudaContext (replaces gpu_context.rs) |
| dcl-gpu | src/kernels/*.rs | 8 kernel wrappers (new) |
| dcl-gpu | src/cuda/*.cu | 8 CUDA C kernel sources (new) |
| dcl-gpu | src/benchmark.rs | GPU vs CPU benchmarks (rewrite) |
| dcl-gpu | src/gcd_batch.rs | Remove (replaced by kernels/gcd.rs) |
| dcl-gpu | src/prime_sieve.rs | Remove (replaced by kernels/prime_sieve.rs) |
| dcl-gpu | src/compute.rs | Remove (replaced by context.rs) |
| dcl-gpu | src/gpu_context.rs | Remove (replaced by context.rs) |
| dcl-cli | src/cmd_gpu.rs | Rewrite for CUDA operations |
