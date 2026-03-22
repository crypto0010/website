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

    // Check coprimality on all edges using Euclidean GCD
    bool valid = true;
    for (int e = 0; e < num_edges; e++) {
        unsigned long long a = labels[edge_u[e]];
        unsigned long long b = labels[edge_v[e]];
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

        // Use smaller block size to avoid CUDA_ERROR_LAUNCH_OUT_OF_RESOURCES
        // due to the large per-thread labels[32] array consuming many registers.
        let block_size = 128u32;
        let grid_size = (num_threads as u32 + block_size - 1) / block_size;
        let cfg = LaunchConfig {
            grid_dim: (grid_size, 1, 1),
            block_dim: (block_size, 1, 1),
            shared_mem_bytes: 0,
        };
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
