// dcl-cli/src/cmd_hypergraph.rs
use clap::Args;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use dcl_hypergraph::hypergraph::Hypergraph;
use dcl_hypergraph::hypergraph_dcl::{verify_existence_equivalence, HypergraphDcl};

#[derive(Args)]
pub struct HypergraphArgs {
    /// Number of vertices
    #[arg(short, long, default_value_t = 5)]
    pub n: usize,
    /// Uniformity k (all hyperedges have k vertices)
    #[arg(short, long, default_value_t = 3)]
    pub k: usize,
    /// Use complete k-uniform hypergraph
    #[arg(long, default_value_t = true)]
    pub complete: bool,
    /// Number of DCL steps
    #[arg(short, long, default_value_t = 20)]
    pub steps: usize,
    /// Power map exponent m
    #[arg(short, long, default_value_t = 2)]
    pub m: u32,
}

pub fn run(args: HypergraphArgs) {
    let h = if args.complete {
        Hypergraph::complete_uniform(args.n, args.k)
    } else {
        eprintln!("Custom hypergraph not yet supported via CLI. Using complete.");
        Hypergraph::complete_uniform(args.n, args.k)
    };

    let primes = [1u64, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31];
    let labels: Vec<u64> = (0..args.n).map(|i| primes[i % primes.len()]).collect();
    let f0 = Labeling::new(labels);
    let pm = PowerMap::new(args.m);

    println!(
        "=== Hypergraph DCL: K^{}_{{{}}} | g(x)=x^{} | {} steps ===",
        args.k, args.n, args.m, args.steps
    );
    println!("  Hyperedges: {}", h.edges.len());
    println!("  f_0 valid : {}", h.is_valid_labeling(&f0));

    if !h.is_valid_labeling(&f0) {
        eprintln!("Initial labeling not valid. Use coprime labels.");
        return;
    }

    // Verify existence equivalence
    let equiv = verify_existence_equivalence(&h, f0.clone(), &pm, args.steps);
    println!(
        "  Existence Equivalence (Theorem 4.2): {}",
        if equiv { "✓ holds" } else { "✗ FAILED" }
    );

    // Run DCL sequence
    let mut dcl = HypergraphDcl::new(&h, &pm, f0);
    match dcl.verify_steps(args.steps) {
        Ok(()) => println!(
            "✓ Setwise coprimality maintained for all {} steps.",
            args.steps
        ),
        Err(t) => eprintln!("✗ Coprimality violated at step {t}"),
    }
}
