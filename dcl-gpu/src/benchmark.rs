//! GPU vs CPU performance benchmarks.

use crate::context::CudaContext;
use crate::error::GpuResult;
use crate::kernels::{gcd::GpuGcdBatch, prime_sieve::GpuPrimeSieve, power_map::GpuPowerMap};
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
