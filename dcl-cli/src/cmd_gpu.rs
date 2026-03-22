//! GPU acceleration CLI command — CUDA-native operations.

use clap::Args;
use crate::cmd_labeling::build_graph;

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
    #[arg(short = 'n', long, default_value_t = 100_000)]
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

    // Drop initial ctx since each demo creates its own
    drop(ctx);

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
    let (_gcds, coprimes) = batch.compute_batch(&a, &b).unwrap();
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

    let graph = build_graph(&args.graph, args.vertices);

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
