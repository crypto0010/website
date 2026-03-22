// dcl-cli/src/cmd_sieve.rs
use clap::Args;
use dcl_core::labeling::Labeling;
use dcl_crypto::baseline::standard_safe_prime;
use dcl_crypto::bias::prime_distribution_test;
use dcl_crypto::sieve::{hdcl_sp_generate, SieveConfig};
use dcl_hypergraph::hypergraph::Hypergraph;
use rand::rngs::StdRng;
use rand::SeedableRng;

#[derive(Args)]
pub struct SieveArgs {
    /// Number of nodes in the hypergraph
    #[arg(short, long, default_value_t = 6)]
    pub nodes: usize,
    /// Hyperedge size
    #[arg(short, long, default_value_t = 3)]
    pub edge_size: usize,
    /// Miller-Rabin rounds
    #[arg(short, long, default_value_t = 40)]
    pub rounds: u32,
    /// Number of safe primes to generate (for bias test)
    #[arg(short, long, default_value_t = 20)]
    pub count: usize,
    /// Run bias distribution test
    #[arg(long, default_value_t = false)]
    pub bias_test: bool,
    /// Compare against standard baseline
    #[arg(long, default_value_t = true)]
    pub compare: bool,
}

pub fn run(args: SieveArgs) {
    println!("=== HDCL-SP Safe-Prime Sieve ===");
    println!(
        "  Nodes: {}, Edge size: {}, MR rounds: {}",
        args.nodes, args.edge_size, args.rounds
    );

    let mut h = Hypergraph::new(args.nodes);
    // Build a simple path of hyperedges
    for i in 0..args.nodes.saturating_sub(args.edge_size) {
        h.add_edge((i..i + args.edge_size).collect());
    }
    if h.edges.is_empty() {
        h.add_edge((0..args.nodes).collect());
    }

    let primes = [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    let config = SieveConfig {
        mr_rounds: args.rounds,
        delta: 1,
        max_attempts: 50_000,
    };

    let mut generated: Vec<u64> = Vec::new();
    let mut total_attempts = 0usize;
    let mut total_rejections = 0usize;

    for i in 0..args.count {
        let init_labels: Vec<u64> = (0..args.nodes)
            .map(|j| primes[(i + j) % primes.len()])
            .collect();
        let mut labels = Labeling::new(init_labels);

        let (result, metrics) = hdcl_sp_generate(&h, &mut labels, &config);
        total_attempts += metrics.attempts;
        total_rejections += metrics.rejections;

        if let Some(p) = result {
            println!(
                "  [{}] Safe prime: {} (attempts={}, time={}ms)",
                i + 1,
                p,
                metrics.attempts,
                metrics.elapsed_ms
            );
            generated.push(p);
        } else {
            println!("  [{}] No safe prime found within attempt limit", i + 1);
        }
    }

    println!("\n--- HDCL-SP Summary ---");
    println!("  Generated  : {}/{}", generated.len(), args.count);
    println!("  Total attempts: {total_attempts}");
    let sr = if total_attempts > 0 {
        (total_attempts - total_rejections) as f64 / total_attempts as f64
    } else {
        0.0
    };
    println!("  Success rate: {:.2}%", sr * 100.0);

    // Baseline comparison
    if args.compare {
        println!("\n--- Baseline Comparison (standard random) ---");
        let mut rng = StdRng::seed_from_u64(42);
        let mut baseline_attempts = 0usize;
        for i in 0..args.count.min(5) {
            let (result, metrics) = standard_safe_prime(16, &mut rng);
            baseline_attempts += metrics.attempts;
            if let Some(p) = result {
                println!(
                    "  [{}] Baseline safe prime: {} (attempts={})",
                    i + 1,
                    p,
                    metrics.attempts
                );
            }
        }
        println!(
            "  Avg baseline attempts: {}",
            baseline_attempts / args.count.min(5).max(1)
        );
    }

    // Bias test
    if args.bias_test && !generated.is_empty() {
        println!("\n--- Prime Distribution Bias Test ---");
        let result = prime_distribution_test(&generated, 5.min(generated.len()));
        println!("  χ² statistic: {:.4}", result.chi_squared);
        println!("  No bias detected: {}", result.no_bias_detected);
        for bin in &result.bins {
            println!(
                "  [{}-{}]: observed={}, expected={:.2}",
                bin.range_lo, bin.range_hi, bin.observed, bin.expected
            );
        }
    }
}
