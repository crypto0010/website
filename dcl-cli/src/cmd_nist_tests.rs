// dcl-cli/src/cmd_nist_tests.rs
//! NIST SP 800-22 Statistical Test Suite CLI command

use clap::Args;
use dcl_crypto::sieve_improved::ImprovedSieve;
use dcl_security::nist_tests::{NistTester, NistTestSuite};
use sha2::{Sha256, Digest};

#[derive(Args)]
pub struct NistTestsArgs {
    /// Number of primes to generate for testing
    #[arg(short = 'c', long, default_value = "100")]
    count: usize,

    /// Miller-Rabin rounds for primality testing
    #[arg(short = 'r', long, default_value = "40")]
    mr_rounds: usize,

    /// Significance level (alpha) for statistical tests
    #[arg(short = 'a', long, default_value = "0.01")]
    alpha: f64,

    /// Use parallel prime generation
    #[arg(short = 'p', long)]
    parallel: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Run NIST tests on DCL label evolution (instead of primes)
    #[arg(long)]
    dcl_mode: bool,

    /// Graph type for DCL mode
    #[arg(long, default_value = "path")]
    dcl_graph: String,

    /// Number of vertices for DCL mode
    #[arg(long, default_value = "10")]
    dcl_n: usize,

    /// Number of evolution steps for DCL mode
    #[arg(long, default_value = "200")]
    steps: usize,

    /// Power map exponent for DCL mode
    #[arg(long, default_value = "2")]
    dcl_m: u32,
}

pub fn run(args: NistTestsArgs) {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║       NIST SP 800-22 STATISTICAL TEST SUITE             ║");
    println!("║       for DCL Framework Prime Generation                ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("Configuration:");
    println!("  Prime Count: {}", args.count);
    println!("  Miller-Rabin Rounds: {}", args.mr_rounds);
    println!("  Significance Level (α): {}", args.alpha);
    println!("  Generation Mode: {}", if args.parallel { "PARALLEL" } else { "SEQUENTIAL" });
    println!();

    let test_data = if args.dcl_mode {
        println!("Mode: DCL Label Evolution");
        println!("  Graph: {}-{}", args.dcl_graph, args.dcl_n);
        println!("  Steps: {}", args.steps);
        println!("  Power Map: x^{}", args.dcl_m);
        println!();

        use dcl_core::graph::Graph;
        use dcl_core::labeling::Labeling;
        use dcl_core::transform::PowerMap;

        let _graph = match args.dcl_graph.as_str() {
            "path" => Graph::path(args.dcl_n),
            "cycle" => Graph::cycle(args.dcl_n),
            "complete" => Graph::complete(args.dcl_n),
            _ => Graph::path(args.dcl_n),
        };

        // Generate initial coprime labeling using small primes
        let primes = vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
        let labels: Vec<u64> = (0..args.dcl_n).map(|i| primes[i % primes.len()]).collect();
        let initial = Labeling::new(labels);

        let pm = PowerMap::new(args.dcl_m);
        println!("Step 1/3: Generating DCL evolution data ({} steps)...", args.steps);
        let data = dcl_security::cryptanalysis::dcl_labels_to_test_data(&pm, &initial, args.steps);
        println!("  Generated {} bytes ({} bits) of DCL evolution data\n", data.len(), data.len() * 8);
        data
    } else {
        // Existing prime-based pipeline
        println!("Step 1/3: Generating {} safe primes...", args.count);
        let sieve = ImprovedSieve::default();

        let start = std::time::Instant::now();
        let primes = if args.parallel {
            sieve.generate_batch_parallel(args.count, args.mr_rounds)
        } else {
            sieve.generate_batch(args.count, args.mr_rounds)
        };
        let generation_time = start.elapsed();

        if primes.primes.is_empty() {
            eprintln!("Failed to generate primes!");
            return;
        }

        println!("  Generated {} primes in {:.2?}", primes.primes.len(), generation_time);
        println!("  Success Rate: {:.1}%\n", primes.success_rate() * 100.0);

        let mut test_data = Vec::new();
        for prime in &primes.primes {
            let mut hasher = Sha256::new();
            hasher.update(prime.to_le_bytes());
            let hash = hasher.finalize();
            test_data.extend_from_slice(&hash);
        }
        test_data
    };

    println!("Step 2/3: {} bytes of test data ready\n", test_data.len());

    println!("Step 3/3: Running NIST SP 800-22 test suite...\n");
    let tester = NistTester::new(args.alpha);
    let start = std::time::Instant::now();
    let results = tester.run_all_tests(&test_data);
    let test_time = start.elapsed();

    // Print results
    if args.verbose {
        results.print_summary();
    } else {
        print_compact_summary(&results);
    }

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║                     FINAL ASSESSMENT                     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let passed = results.count_passed();
    let total = 15;
    let percentage = (passed as f64 / total as f64) * 100.0;

    println!("  Tests Passed: {}/{} ({:.1}%)", passed, total, percentage);
    println!("  Execution Time: {:.2?}", test_time);
    println!();

    if results.all_passed() {
        println!("  ✅ EXCELLENT: All NIST tests passed!");
        println!("  The prime generation exhibits high-quality randomness.");
        println!("  Suitable for cryptographic applications.");
    } else if passed >= 13 {
        println!("  ✓ GOOD: {} tests passed", passed);
        println!("  The prime generation shows good randomness properties.");
        println!("  Minor deviations are within acceptable limits.");
    } else if passed >= 10 {
        println!("  ⚠️  MODERATE: {} tests passed", passed);
        println!("  The prime generation shows acceptable randomness.");
        println!("  Consider increasing sample size or adjusting parameters.");
    } else {
        println!("  ❌ WEAK: Only {} tests passed", passed);
        println!("  The prime generation may have randomness issues.");
        println!("  Recommended actions:");
        println!("    - Increase prime count (--count)");
        println!("    - Adjust generation parameters");
        println!("    - Review failed tests with --verbose");
    }

    println!();
    println!("For detailed analysis, run with --verbose flag");
    println!();
}

fn print_compact_summary(results: &NistTestSuite) {
    println!("═══════════════════════════════════════════════════════════");
    println!("  NIST SP 800-22 Test Results Summary");
    println!("═══════════════════════════════════════════════════════════\n");

    let tests = vec![
        ("Frequency (Monobit)", &results.frequency_monobit),
        ("Frequency Block", &results.frequency_block),
        ("Runs", &results.runs),
        ("Longest Run", &results.longest_run),
        ("Binary Matrix Rank", &results.binary_matrix_rank),
        ("DFT (Spectral)", &results.discrete_fourier_transform),
        ("Non-overlapping Template", &results.non_overlapping_template),
        ("Overlapping Template", &results.overlapping_template),
        ("Universal Statistical", &results.universal),
        ("Linear Complexity", &results.linear_complexity),
        ("Approximate Entropy", &results.approximate_entropy),
    ];

    for (name, result) in tests {
        let status = if result.passed { "✓" } else { "✗" };
        println!("  {} {:40} (p={:.4})", status, name, result.p_value);
    }

    println!("  {} {:40} (p1={:.4}, p2={:.4})",
        if results.serial.passed { "✓" } else { "✗" },
        "Serial",
        results.serial.p_value1,
        results.serial.p_value2);

    println!("  {} {:40} (pF={:.4}, pB={:.4})",
        if results.cumulative_sums.passed { "✓" } else { "✗" },
        "Cumulative Sums",
        results.cumulative_sums.p_value_forward,
        results.cumulative_sums.p_value_backward);

    println!("  {} {:40} ({}/{})",
        if results.random_excursions.passed { "✓" } else { "✗" },
        "Random Excursions",
        results.random_excursions.states_passed,
        results.random_excursions.states_tested);

    println!("  {} {:40} ({}/{})",
        if results.random_excursions_variant.passed { "✓" } else { "✗" },
        "Random Excursions Variant",
        results.random_excursions_variant.states_passed,
        results.random_excursions_variant.states_tested);

    println!();
}
