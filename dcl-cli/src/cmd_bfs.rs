// dcl-cli/src/cmd_bfs.rs
use crate::cmd_labeling::build_graph;
use clap::Args;
use dcl_complexity::bfs::find_dcl_bfs;
use dcl_complexity::index::minimum_dcl_index;
use dcl_core::transform::PowerMap;

#[derive(Args)]
pub struct BfsArgs {
    /// Graph type: path, cycle, wheel, hypercube, complete
    #[arg(short, long, default_value = "path")]
    pub graph: String,
    /// Number of vertices / dimension
    #[arg(short, long, default_value_t = 6)]
    pub n: usize,
    /// Power map exponent m
    #[arg(short, long, default_value_t = 2)]
    pub m: u32,
    /// Upper bound for label search
    #[arg(long, default_value_t = 50)]
    pub max_label: u64,
    /// Compute minimum DCL index μ(G,g) via binary search
    #[arg(long, default_value_t = false)]
    pub compute_mu: bool,
}

pub fn run(args: BfsArgs) {
    let graph = build_graph(&args.graph, args.n);
    let transform = PowerMap::new(args.m);

    println!(
        "=== DCL-BFS: {}-{} | g(x)=x^{} | max_label={} ===",
        args.graph, args.n, args.m, args.max_label
    );

    let result = find_dcl_bfs(&graph, args.max_label);
    println!("States explored: {}", result.states_explored);

    match result.labeling {
        Some(lab) => {
            println!("✓ Valid initial labeling found: {lab}");
            println!("  Max label used: {}", lab.max_label());
            assert!(graph.is_coprime_labeling(&lab));
            assert!(lab.is_injective());
        }
        None => {
            println!(
                "✗ No valid labeling found within max_label={}",
                args.max_label
            );
        }
    }

    if args.compute_mu {
        let mu = minimum_dcl_index(&graph, &transform, args.max_label);
        println!("μ(G, g) = {mu}");
    }
}
