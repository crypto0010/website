// dcl-cli/src/cmd_bigint.rs
//! BigInt DCL evolution - unlimited steps without overflow!

use clap::Args;
use dcl_core::graph::Graph;
use dcl_core::labeling_bigint::{DclSequenceBigInt, LabelingBigInt};
use num_bigint::BigUint;

#[derive(Args)]
pub struct BigIntArgs {
    /// Graph type: path, cycle, wheel, hypercube, complete
    #[arg(short, long, default_value = "path")]
    graph: String,

    /// Number of vertices
    #[arg(short, long, default_value = "5")]
    n: usize,

    /// Power map exponent m (g(x) = x^m)
    #[arg(short, long, default_value = "2")]
    m: u32,

    /// Number of DCL steps (can be large - no overflow!)
    #[arg(short, long, default_value = "20")]
    steps: usize,

    /// Use modular arithmetic (bounded evolution)
    #[arg(long)]
    modular: bool,

    /// Modulus for modular arithmetic (large prime)
    #[arg(long, default_value = "1000000007")]
    modulus: u64,
}

pub fn run(args: BigIntArgs) {
    println!("=== BigInt DCL Evolution: {}-{} | g(x)=x^{} | {} steps ===\n",
             args.graph, args.n, args.m, args.steps);

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

    // Generate initial labeling (first n primes)
    let initial_labels = generate_prime_labels(args.n);
    let f0 = LabelingBigInt::new(initial_labels);

    println!("f_0: {}", f0.to_string());

    // Create DCL sequence
    let mut seq = if args.modular {
        println!("Using modular arithmetic (mod {})\n", args.modulus);
        DclSequenceBigInt::new_modular(f0, BigUint::from(args.modulus))
    } else {
        println!("Using unlimited BigInt (no modulus)\n");
        DclSequenceBigInt::new(f0)
    };

    let edges = graph.edges();

    // Evolve for specified steps
    for i in 1..=args.steps {
        seq.advance(args.m);

        let coprime = seq.verify_coprimality(&edges);

        // Print every 5 steps or if error
        if i % 5 == 0 || i == args.steps || !coprime {
            print!("f_{}: {}", seq.step, seq.current.to_string());
            println!(" | coprime={}", coprime);
        }

        if !coprime {
            println!("\n❌ ERROR at step {}: coprimality violated!", seq.step);
            return;
        }
    }

    println!("\n✅ Coprimality maintained for all {} steps!", args.steps);

    if !args.modular {
        println!("Note: Labels are now HUGE (would have overflowed u64 around step 5)");
    }
}

/// Generate first n prime numbers.
fn generate_prime_labels(n: usize) -> Vec<u64> {
    let primes = [1, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67];
    primes.iter().take(n).copied().collect()
}
