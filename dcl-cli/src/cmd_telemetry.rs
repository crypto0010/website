// dcl-cli/src/cmd_telemetry.rs
//! Telemetry and monitoring CLI command

use clap::Args;
use dcl_security::telemetry::{
    init_telemetry, TelemetryConfig, TelemetryEvent, EventLevel,
    record_counter, record_gauge, record_histogram, get_snapshot, instrumented,
};
use dcl_crypto::sieve_improved::ImprovedSieve;
use dcl_crypto::miller_rabin::miller_rabin;
use dcl_security::side_channel;
use dcl_core::gcd;
use rand::thread_rng;
use std::time::Instant;

#[derive(Args)]
pub struct TelemetryArgs {
    /// Enable JSON output format
    #[arg(long)]
    json: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    level: String,

    /// Use production configuration
    #[arg(short, long)]
    production: bool,

    /// Number of operations to run for demo
    #[arg(short = 'n', long, default_value = "100")]
    operations: usize,

    /// Run performance benchmark
    #[arg(short = 'b', long)]
    benchmark: bool,

    /// Export telemetry snapshot to file
    #[arg(short = 'o', long)]
    output: Option<String>,
}

pub fn run(args: TelemetryArgs) {
    // Initialize telemetry
    let config = if args.production {
        TelemetryConfig::production()
    } else {
        TelemetryConfig {
            log_level: args.level.clone(),
            json_output: args.json,
            ..TelemetryConfig::development()
        }
    };

    if let Err(e) = init_telemetry(config) {
        eprintln!("Failed to initialize telemetry: {}", e);
        return;
    }

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║           DCL FRAMEWORK TELEMETRY & MONITORING           ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Record initial event
    dcl_security::telemetry::record_event(
        TelemetryEvent::new(EventLevel::Info, "system", "Telemetry system started")
            .with_metadata("operations", &args.operations.to_string())
            .with_metadata("mode", if args.production { "production" } else { "development" })
    );

    if args.benchmark {
        run_benchmark(args.operations);
    } else {
        run_demo(args.operations);
    }

    // Get and display snapshot
    let snapshot = get_snapshot();

    if args.json {
        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            println!("{}", json);
        }
    } else {
        snapshot.print_summary();
    }

    // Export to file if requested
    if let Some(output_path) = args.output {
        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            if let Err(e) = std::fs::write(&output_path, json) {
                eprintln!("Failed to write to {}: {}", output_path, e);
            } else {
                println!("Telemetry snapshot exported to: {}", output_path);
            }
        }
    }
}

fn run_demo(operations: usize) {
    println!("Running telemetry demo with {} operations...\n", operations);

    // Demo 1: Prime generation with instrumentation
    println!("1. Prime Generation (instrumented)");
    let sieve = ImprovedSieve::default();
    let primes = instrumented::prime_generation(operations / 10, || {
        let mut rng = thread_rng();
        sieve.generate_safe_prime(&mut rng, 40).ok().map(|(p, _)| p)
    });
    println!("   Generated {} primes", primes.len());

    // Demo 2: GCD computations
    println!("2. GCD Computations");
    for i in 0..operations {
        let a = (i * 17 + 13) as u64;
        let b = (i * 23 + 19) as u64;
        let result = gcd::gcd(a, b);
        instrumented::gcd_computation(a, b, result);
    }
    record_gauge("gcd_demo_completed", 1.0);

    // Demo 3: Side-channel operations
    println!("3. Side-Channel Operations");
    for i in 0..operations {
        let a = (i * 31 + 29) as u64;
        let b = (i * 37 + 41) as u64;
        let _ = side_channel::gcd_constant_time(a, b);
        instrumented::side_channel_operation("gcd_constant_time", true);
    }

    // Demo 4: NIST test simulation
    println!("4. NIST Test Execution (simulated)");
    let test_names = ["frequency", "runs", "dft", "rank", "entropy"];
    for name in &test_names {
        let passed = rand::random::<f64>() > 0.1; // 90% pass rate for demo
        let p_value = rand::random::<f64>() * 0.5 + 0.01;
        instrumented::nist_test_execution(name, passed, p_value);
    }

    // Custom metrics
    record_counter("demo_operations_completed", operations as u64);
    record_gauge("system_health", 0.95);
    record_histogram("operation_distribution", operations as f64);

    println!("\nDemo completed. See telemetry snapshot below.\n");
}

fn run_benchmark(operations: usize) {
    println!("Running performance benchmark with {} operations...\n", operations);

    // Benchmark 1: Standard GCD
    println!("Benchmark 1: Standard GCD");
    let start = Instant::now();
    for i in 0..operations {
        let a = (i * 1000 + 123) as u64;
        let b = (i * 500 + 456) as u64;
        let _ = gcd::gcd(a, b);
    }
    let gcd_time = start.elapsed();
    println!("   {} GCD operations in {:.2?}", operations, gcd_time);
    record_histogram("benchmark_gcd_time_ms", gcd_time.as_secs_f64() * 1000.0);

    // Benchmark 2: Constant-time GCD
    println!("Benchmark 2: Constant-Time GCD");
    let start = Instant::now();
    for i in 0..operations {
        let a = (i * 1000 + 123) as u64;
        let b = (i * 500 + 456) as u64;
        let _ = side_channel::gcd_constant_time(a, b);
    }
    let ct_gcd_time = start.elapsed();
    println!("   {} constant-time GCD operations in {:.2?}", operations, ct_gcd_time);
    record_histogram("benchmark_ct_gcd_time_ms", ct_gcd_time.as_secs_f64() * 1000.0);

    // Benchmark 3: Miller-Rabin primality test
    println!("Benchmark 3: Miller-Rabin Primality Testing");
    let test_candidates: Vec<u64> = (0..operations)
        .map(|i| (i * 1000 + 997) as u64)
        .collect();

    let start = Instant::now();
    let mut prime_count = 0;
    for &n in &test_candidates {
        if miller_rabin(n, 20) {
            prime_count += 1;
        }
    }
    let mr_time = start.elapsed();
    println!("   {} primality tests in {:.2?}", operations, mr_time);
    println!("   Found {} probable primes", prime_count);
    record_histogram("benchmark_miller_rabin_time_ms", mr_time.as_secs_f64() * 1000.0);
    record_gauge("benchmark_prime_ratio", prime_count as f64 / operations as f64);

    // Benchmark 4: Byte comparison (constant-time vs standard)
    println!("Benchmark 4: Byte Comparison");
    let data1 = vec![0x42u8; 256];
    let data2 = vec![0x42u8; 256];

    let start = Instant::now();
    for _ in 0..operations {
        let _ = side_channel::compare_bytes_constant_time(&data1, &data2);
    }
    let ct_cmp_time = start.elapsed();
    println!("   {} constant-time comparisons in {:.2?}", operations, ct_cmp_time);
    record_histogram("benchmark_ct_compare_time_ms", ct_cmp_time.as_secs_f64() * 1000.0);

    // Summary metrics
    let total_time = gcd_time + ct_gcd_time + mr_time + ct_cmp_time;
    println!("\nBenchmark Summary:");
    println!("   Total Time: {:.2?}", total_time);
    println!("   GCD Overhead: {:.2}x", ct_gcd_time.as_secs_f64() / gcd_time.as_secs_f64());
    println!("   Operations/sec: {:.2}", (operations * 4) as f64 / total_time.as_secs_f64());

    record_gauge("benchmark_total_time_ms", total_time.as_secs_f64() * 1000.0);
    record_gauge("benchmark_ops_per_second", (operations * 4) as f64 / total_time.as_secs_f64());
    record_counter("benchmark_total_operations", (operations * 4) as u64);

    println!("\nBenchmark completed. See detailed metrics below.\n");
}
