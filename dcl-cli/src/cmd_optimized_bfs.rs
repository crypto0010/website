// dcl-cli/src/cmd_optimized_bfs.rs
//! Optimized BFS with intelligent pruning strategies.

use clap::Args;
use dcl_complexity::bfs_optimized::{OptimizedBfs, PruningConfig};
use dcl_core::graph::Graph;

#[derive(Args)]
pub struct OptimizedBfsArgs {
    /// Graph type: path, cycle, wheel, hypercube, complete
    #[arg(short, long, default_value = "path")]
    graph: String,

    /// Number of vertices
    #[arg(short, long, default_value = "4")]
    n: usize,

    /// Maximum label value to search
    #[arg(long, default_value = "30")]
    max_label: u64,

    /// Prefer prime labels (coprime-friendly)
    #[arg(long, default_value = "true")]
    prefer_primes: bool,

    /// Use symmetry breaking
    #[arg(long, default_value = "true")]
    use_symmetry: bool,

    /// Maximum sum of labels
    #[arg(long)]
    max_sum: Option<u64>,

    /// Maximum number of solutions to find
    #[arg(long, default_value = "5")]
    max_solutions: usize,

    /// Run both naive and pruned, print comparison table
    #[arg(long)]
    compare: bool,
}

pub fn run(args: OptimizedBfsArgs) {
    println!("=== Optimized BFS Search ===");
    println!("Graph: {}-{}", args.graph, args.n);
    println!("Max Label: {}", args.max_label);
    println!("Pruning: primes={}, symmetry={}, max_sum={:?}\n",
             args.prefer_primes, args.use_symmetry, args.max_sum);

    // Build graph
    let graph = match args.graph.as_str() {
        "path" => Graph::path(args.n),
        "cycle" => Graph::cycle(args.n),
        "wheel" => Graph::wheel(args.n),
        "hypercube" => Graph::hypercube(args.n),
        "complete" => Graph::complete(args.n),
        _ => {
            eprintln!("Unknown graph type: {}", args.graph);
            return;
        }
    };

    // Configure pruning
    let config = PruningConfig {
        prefer_primes: args.prefer_primes,
        use_symmetry: args.use_symmetry,
        max_sum: args.max_sum,
        max_solutions: args.max_solutions,
        incremental_check: true,
    };

    // Run search
    let mut searcher = OptimizedBfs::new(&graph, args.max_label, config);

    println!("Searching...");
    let result = searcher.search();

    println!("\n=== Results ===");
    println!("Solutions Found: {}", result.solutions.len());
    println!("States Explored: {}", result.states_explored);
    println!("States Pruned: {} ({:.1}% pruned)",
             result.states_pruned,
             (result.states_pruned as f64 / (result.states_explored + result.states_pruned) as f64) * 100.0);

    if result.solutions.is_empty() {
        println!("\n❌ No solutions found within max_label={}", args.max_label);
        println!("   Try increasing --max-label or relaxing pruning constraints");
    } else {
        println!("\n✅ Found Solutions:");
        for (i, sol) in result.solutions.iter().enumerate().take(5) {
            println!("  [{}] {:?}", i + 1, sol.labels);
        }

        if result.solutions.len() > 5 {
            println!("  ... and {} more", result.solutions.len() - 5);
        }

        println!("\nMax Label Used: {}", result.max_label_found);
    }

    // Performance summary
    let efficiency = if result.states_explored > 0 {
        result.states_pruned as f64 / result.states_explored as f64
    } else {
        0.0
    };

    println!("\nPruning Efficiency: {:.2}× reduction", 1.0 + efficiency);

    if args.compare {
        println!("\n=== Naive vs Pruned Comparison ===\n");

        // Run naive (no pruning)
        let naive_config = PruningConfig {
            prefer_primes: false,
            use_symmetry: false,
            max_sum: None,
            max_solutions: args.max_solutions,
            incremental_check: false,
        };
        let mut naive_searcher = OptimizedBfs::new(&graph, args.max_label, naive_config);
        let naive_result = naive_searcher.search();

        // Run pruned (all enabled)
        let pruned_config = PruningConfig {
            prefer_primes: true,
            use_symmetry: true,
            max_sum: args.max_sum,
            max_solutions: args.max_solutions,
            incremental_check: true,
        };
        let mut pruned_searcher = OptimizedBfs::new(&graph, args.max_label, pruned_config);
        let pruned_result = pruned_searcher.search();

        let naive_total = naive_result.states_explored + naive_result.states_pruned;
        let pruned_total = pruned_result.states_explored + pruned_result.states_pruned;

        println!("{:<20} {:>12} {:>12}", "Metric", "Naive", "Pruned");
        println!("{}", "-".repeat(46));
        println!("{:<20} {:>12} {:>12}", "States explored",
            naive_result.states_explored, pruned_result.states_explored);
        println!("{:<20} {:>12} {:>12}", "States pruned",
            naive_result.states_pruned, pruned_result.states_pruned);
        println!("{:<20} {:>12} {:>12}", "Total visited",
            naive_total, pruned_total);
        println!("{:<20} {:>12} {:>12}", "Solutions found",
            naive_result.solutions.len(), pruned_result.solutions.len());

        if naive_total > 0 && pruned_total > 0 {
            let reduction = 1.0 - (pruned_total as f64 / naive_total as f64);
            let speedup = naive_total as f64 / pruned_total as f64;
            println!("\nPruning reduction: {:.1}%", reduction * 100.0);
            println!("Speedup factor: {:.2}x", speedup);
        }
    }
}
