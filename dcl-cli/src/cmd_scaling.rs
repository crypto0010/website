//! Security scaling benchmark CLI subcommand.

use clap::Args;
use dcl_security::scaling;

#[derive(Args)]
pub struct ScalingArgs {
    /// Graph type: path, cycle, complete
    #[arg(short, long, default_value = "path")]
    graph: String,

    /// Minimum graph size
    #[arg(long, default_value = "4")]
    min_size: usize,

    /// Maximum graph size
    #[arg(long, default_value = "12")]
    max_size: usize,

    /// Step between sizes
    #[arg(long, default_value = "2")]
    step: usize,

    /// Maximum label value
    #[arg(long, default_value = "50")]
    max_label: u64,

    /// Power map exponent
    #[arg(short, long, default_value = "2")]
    m: u32,

    /// Optional prime modulus for bounded label evolution
    #[arg(long)]
    modulus: Option<u64>,
}

pub fn run(args: ScalingArgs) {
    println!("\n=== DCL Security Scaling Benchmark ===\n");
    println!("Graph type: {}", args.graph);
    println!("Size range: {} to {} (step {})", args.min_size, args.max_size, args.step);
    println!("Max label: {}, Power map: x^{}\n", args.max_label, args.m);

    let sizes: Vec<usize> = (args.min_size..=args.max_size)
        .step_by(args.step)
        .collect();

    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m, args.modulus);
    scaling::print_scaling_table(&results);
}
