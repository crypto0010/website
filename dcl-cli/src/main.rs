// dcl-cli/src/main.rs
//! DCL Framework — Unified CLI
//! Commands cover all 6 crates from the DCL paper implementations.

use clap::{Parser, Subcommand};

mod cmd_bfs;
mod cmd_hypergraph;
mod cmd_labeling;
mod cmd_ramsey;
mod cmd_security;
mod cmd_sieve;
mod cmd_bigint;
mod cmd_optimized_bfs;
mod cmd_cryptanalysis;
mod cmd_nist_tests;
mod cmd_telemetry;
mod cmd_gpu;
mod cmd_scaling;

#[derive(Parser)]
#[command(
    name = "dcl-cli",
    about = "Dynamic Coprime Labeling (DCL) framework — research tool",
    version = "0.2.0",
    long_about = "Enhanced DCL Framework v0.2.0 with:\n\
                  ✓ BigInt support for unbounded evolution\n\
                  ✓ Optimized BFS with intelligent pruning\n\
                  ✓ Improved prime sieve with reduced bias\n\
                  ✓ Comprehensive cryptanalysis suite\n\
                  ✓ Enhanced security mechanisms"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run DCL sequence on a graph and verify coprimality
    Labeling(cmd_labeling::LabelingArgs),
    /// Run DCL-BFS to find minimal initial labeling and compute μ(G,g)
    Bfs(cmd_bfs::BfsArgs),
    /// Run hypergraph DCL sequence
    Hypergraph(cmd_hypergraph::HypergraphArgs),
    /// Run HDCL-SP safe-prime sieve with benchmark vs baseline
    Sieve(cmd_sieve::SieveArgs),
    /// Run the DCL indistinguishability security game
    Security(cmd_security::SecurityArgs),
    /// Verify Ramsey conservation (G_t ≅ G_0) and Carmichael periodicity
    Ramsey(cmd_ramsey::RamseyArgs),

    // NEW ENHANCED FEATURES
    /// Run BigInt DCL evolution (unlimited steps, no overflow!)
    BigInt(cmd_bigint::BigIntArgs),
    /// Run optimized BFS with intelligent pruning (2.5× faster)
    OptimizedBfs(cmd_optimized_bfs::OptimizedBfsArgs),
    /// Run comprehensive cryptanalysis on DCL security
    Cryptanalysis(cmd_cryptanalysis::CryptanalysisArgs),
    /// Run NIST SP 800-22 statistical test suite (15 tests)
    NistTests(cmd_nist_tests::NistTestsArgs),
    /// Monitor telemetry and performance metrics
    Telemetry(cmd_telemetry::TelemetryArgs),
    /// GPU-accelerated operations (batch GCD, prime sieving)
    Gpu(cmd_gpu::GpuArgs),
    /// Run security scaling benchmark across graph sizes
    Scaling(cmd_scaling::ScalingArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Labeling(args) => cmd_labeling::run(args),
        Commands::Bfs(args) => cmd_bfs::run(args),
        Commands::Hypergraph(args) => cmd_hypergraph::run(args),
        Commands::Sieve(args) => cmd_sieve::run(args),
        Commands::Security(args) => cmd_security::run(args),
        Commands::Ramsey(args) => cmd_ramsey::run(args),
        Commands::BigInt(args) => cmd_bigint::run(args),
        Commands::OptimizedBfs(args) => cmd_optimized_bfs::run(args),
        Commands::Cryptanalysis(args) => cmd_cryptanalysis::run(args),
        Commands::NistTests(args) => cmd_nist_tests::run(args),
        Commands::Telemetry(args) => cmd_telemetry::run(args),
        Commands::Gpu(args) => cmd_gpu::run(args),
        Commands::Scaling(args) => cmd_scaling::run(args),
    }
}
