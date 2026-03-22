// dcl-cli/src/cmd_cryptanalysis.rs
//! Comprehensive cryptanalysis and security testing.

use clap::Args;
use dcl_core::graph::Graph;
use dcl_security::cryptanalysis::Cryptanalyzer;

#[derive(Args)]
pub struct CryptanalysisArgs {
    /// Graph type: path, cycle, wheel, complete
    #[arg(short, long, default_value = "path")]
    graph: String,

    /// Number of vertices
    #[arg(short, long, default_value = "4")]
    n: usize,

    /// Power map exponent
    #[arg(short, long, default_value = "2")]
    m: u32,

    /// Run quick analysis (fewer tests)
    #[arg(long)]
    quick: bool,

    /// Maximum label value for search space
    #[arg(long, default_value = "20")]
    max_label: u64,

    /// Number of brute-force samples (for large graphs)
    #[arg(long, default_value = "50")]
    samples: usize,

    /// Optional prime modulus for bounded label evolution
    #[arg(long)]
    modulus: Option<u64>,
}

pub fn run(args: CryptanalysisArgs) {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║     DCL CRYPTANALYSIS - COMPREHENSIVE SECURITY      ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    println!("Configuration:");
    println!("  Graph: {}-{}", args.graph, args.n);
    println!("  Power Map: g(x) = x^{}", args.m);
    println!("  Mode: {}\n", if args.quick { "QUICK" } else { "FULL" });

    // Build graph
    let graph = match args.graph.as_str() {
        "path" => Graph::path(args.n),
        "cycle" => Graph::cycle(args.n),
        "wheel" => Graph::wheel(args.n),
        "complete" => Graph::complete(args.n),
        _ => {
            eprintln!("Unknown graph type: {}", args.graph);
            return;
        }
    };

    // Create analyzer with config
    let config = dcl_security::cryptanalysis::CryptanalysisConfig {
        max_label: args.max_label,
        power_map_exp: args.m,
        test_iterations: if args.quick { 20 } else { 100 },
        brute_force_samples: args.samples,
        modulus: args.modulus,
    };

    let analyzer = Cryptanalyzer::with_config(&graph, config);

    // Run analysis
    println!("Running comprehensive security analysis...\n");
    let report = analyzer.analyze();

    // Print detailed report
    report.print_report();

    // Additional recommendations
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║                   RECOMMENDATIONS                    ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    match report.overall_security_level {
        dcl_security::cryptanalysis::SecurityLevel::High => {
            println!("✅ Security is EXCELLENT for this configuration");
            println!("   Safe for production deployment");
        }
        dcl_security::cryptanalysis::SecurityLevel::Medium => {
            println!("⚠️  Security is ADEQUATE but could be improved");
            println!("   Recommendations:");
            println!("   - Increase graph size (n ≥ 8)");
            println!("   - Use more observation steps");
            println!("   - Enable computational hardness");
        }
        dcl_security::cryptanalysis::SecurityLevel::Low => {
            println!("⚠️  Security is WEAK for this configuration");
            println!("   Critical recommendations:");
            println!("   - Increase graph size significantly");
            println!("   - Avoid complete graphs with n < 8");
            println!("   - Implement additional security layers");
        }
        dcl_security::cryptanalysis::SecurityLevel::Vulnerable => {
            println!("❌ Security is VULNERABLE - DO NOT USE IN PRODUCTION");
            println!("   This configuration is easily broken");
            println!("   Required actions:");
            println!("   - Change graph type (avoid small complete graphs)");
            println!("   - Increase to n ≥ 8");
            println!("   - Enable all security enhancements");
        }
    }

    println!("\nFor enhanced security, use:");
    println!("  dcl-cli enhanced-security --help");
}
