// dcl-cli/src/cmd_labeling.rs
use clap::Args;
use dcl_core::dcl::DclSequence;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;

#[derive(Args)]
pub struct LabelingArgs {
    /// Graph type: path, cycle, wheel, hypercube, complete
    #[arg(short, long, default_value = "path")]
    pub graph: String,
    /// Number of vertices (for path/cycle/wheel/complete) or dimension (for hypercube)
    #[arg(short, long, default_value_t = 5)]
    pub n: usize,
    /// Power map exponent m (g(x) = x^m)
    #[arg(short, long, default_value_t = 2)]
    pub m: u32,
    /// Number of DCL steps to simulate
    #[arg(short, long, default_value_t = 10)]
    pub steps: usize,
}

pub fn run(args: LabelingArgs) {
    let graph = build_graph(&args.graph, args.n);
    let transform = PowerMap::new(args.m);

    // Use prime-based initial labeling for maximum coprimality coverage
    let primes = [
        1u64, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
        83, 89, 97, 101, 103, 107, 109, 113, 127, 131, 137,
    ];
    let labels: Vec<u64> = (0..graph.n).map(|i| primes[i % primes.len()]).collect();
    let f0 = Labeling::new(labels);

    if !graph.is_coprime_labeling(&f0) {
        eprintln!(
            "Warning: initial labeling is not fully coprime for this graph. \
                   Use dcl bfs to find a valid one first."
        );
        return;
    }

    println!(
        "=== DCL Simulation: {}-{} | g(x)=x^{} | {} steps ===",
        args.graph, args.n, args.m, args.steps
    );
    println!("f_0: {f0}");

    let mut seq = DclSequence::new(&graph, &transform, f0);
    for t in 1..=args.steps {
        seq.advance();
        let valid = seq.is_valid();
        println!("f_{t}: {} | coprime={valid}", seq.current);
        if !valid {
            eprintln!("ERROR at step {t}: coprimality violated!");
            return;
        }
    }
    println!("✓ Coprimality maintained for all {} steps.", args.steps);
}

pub fn build_graph(kind: &str, n: usize) -> Graph {
    match kind {
        "path" => Graph::path(n),
        "cycle" => Graph::cycle(n),
        "wheel" => Graph::wheel(n),
        "hypercube" => Graph::hypercube(n),
        "complete" => Graph::complete(n),
        other => {
            eprintln!("Unknown graph type '{other}'. Using path.");
            Graph::path(n)
        }
    }
}
