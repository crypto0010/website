// dcl-cli/src/cmd_ramsey.rs
use crate::cmd_labeling::build_graph;
use clap::Args;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use dcl_ramsey::carmichael::{carmichael_lambda, modular_label_period};
use dcl_ramsey::ramsey::{run_ramsey_suite, verify_ramsey_conservation};

#[derive(Args)]
pub struct RamseyArgs {
    /// Graph type: path, cycle, wheel, hypercube, complete
    #[arg(short, long, default_value = "path")]
    pub graph: String,
    /// Number of vertices
    #[arg(short, long, default_value_t = 5)]
    pub n: usize,
    /// Power map exponent m
    #[arg(short, long, default_value_t = 2)]
    pub m: u32,
    /// DCL steps to verify
    #[arg(short, long, default_value_t = 50)]
    pub steps: usize,
    /// Run Carmichael λ analysis
    #[arg(long, default_value_t = false)]
    pub carmichael: bool,
    /// Run full Ramsey suite on standard graph families
    #[arg(long, default_value_t = false)]
    pub suite: bool,
}

pub fn run(args: RamseyArgs) {
    let transform = PowerMap::new(args.m);

    if args.suite {
        println!("=== Ramsey Conservation Suite ===");
        let results = run_ramsey_suite(args.steps);
        for r in results {
            r.print();
            println!();
        }
        return;
    }

    println!(
        "=== Ramsey Conservation: {}-{} | g(x)=x^{} | {} steps ===",
        args.graph, args.n, args.m, args.steps
    );

    let graph = build_graph(&args.graph, args.n);

    // Prime-based initial labeling
    let primes = [
        1u64, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
        83, 89, 97, 101, 103, 107, 109,
    ];
    let labels: Vec<u64> = (0..graph.n).map(|i| primes[i % primes.len()]).collect();
    let f0 = Labeling::new(labels);

    if !graph.is_coprime_labeling(&f0) {
        eprintln!("Warning: initial labeling is not coprime for this graph.");
        return;
    }

    let result = verify_ramsey_conservation(
        &graph,
        &transform,
        f0.clone(),
        args.steps,
        &format!("{}-{}", args.graph, args.n),
    );
    result.print();

    // Carmichael analysis
    if args.carmichael {
        println!("\n=== Carmichael Function Analysis ===");
        for &label in &f0.labels {
            let lam = carmichael_lambda(label);
            let period = modular_label_period(label, args.m, lam.max(2));
            println!(
                "  λ({label}) = {lam} | orbit period of {label} under x^{}: {period}",
                args.m
            );
        }
    }
}
