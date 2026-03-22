// dcl-cli/src/cmd_security.rs
use crate::cmd_labeling::build_graph;
use clap::Args;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use dcl_security::adversary::{BruteForceAdversary, RandomAdversary};
use dcl_security::dl_experiment::{run_dl_experiment, DlInstance};
use dcl_security::game::{run_game, GameConfig};

#[derive(Args)]
pub struct SecurityArgs {
    /// Graph type: path, cycle, wheel, complete
    #[arg(short, long, default_value = "path")]
    pub graph: String,
    /// Number of vertices
    #[arg(short, long, default_value_t = 4)]
    pub n: usize,
    /// Power map exponent
    #[arg(short, long, default_value_t = 2)]
    pub m: u32,
    /// Number of DCL steps the adversary observes
    #[arg(short, long, default_value_t = 5)]
    pub steps: usize,
    /// Number of game trials
    #[arg(short, long, default_value_t = 20)]
    pub trials: usize,
    /// Adversary type: random or brute
    #[arg(short, long, default_value = "random")]
    pub adversary: String,
    /// Run DL-hardness experiment
    #[arg(long, default_value_t = false)]
    pub dl_experiment: bool,
    /// Prime modulus for DL experiment
    #[arg(long, default_value_t = 65537)]
    pub dl_p: u64,
    /// Generator for DL experiment
    #[arg(long, default_value_t = 3)]
    pub dl_g: u64,
}

pub fn run(args: SecurityArgs) {
    println!("=== DCL Security Analysis ===");
    let graph = build_graph(&args.graph, args.n);
    let transform = PowerMap::new(args.m);

    // Build set of f_0 labelings (prime-based, distinct)
    let primes = [1u64, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    let f0_set: Vec<Labeling> = (0..args.trials)
        .map(|i| {
            let mut used = std::collections::HashSet::new();
            let labels: Vec<u64> = (0..args.n)
                .map(|j| {
                    let mut v = primes[(i + j * 3) % primes.len()];
                    while used.contains(&v) {
                        v += 1;
                    }
                    used.insert(v);
                    v
                })
                .collect();
            Labeling::new(labels)
        })
        .collect();

    let game_config = GameConfig {
        steps: args.steps,
        trials: args.trials,
    };

    // Filter to only valid initial labelings
    let valid_f0: Vec<Labeling> = f0_set
        .into_iter()
        .filter(|f| graph.is_coprime_labeling(f))
        .collect();

    println!("  Valid f_0 labelings: {}/{}", valid_f0.len(), args.trials);

    if valid_f0.is_empty() {
        eprintln!("No valid coprime labelings found. Try a simpler graph.");
        return;
    }

    // Run game
    let result = match args.adversary.as_str() {
        "brute" => {
            let adv = BruteForceAdversary { max_label: 50 };
            run_game(&graph, &transform, &valid_f0, &game_config, &adv)
        }
        _ => {
            let adv = RandomAdversary { max_label: 50 };
            run_game(&graph, &transform, &valid_f0, &game_config, &adv)
        }
    };
    result.print();

    // DL experiment
    if args.dl_experiment {
        println!("\n=== DL-Hardness Experiment ===");
        // x = 7 (secret), h = g^7 mod p
        let x = 7u64;
        let dl = DlInstance {
            p: args.dl_p,
            g: args.dl_g,
            h: {
                let mut r: u128 = 1;
                let mut b: u128 = args.dl_g as u128;
                let m = args.dl_p as u128;
                let mut e = x;
                while e > 0 {
                    if e & 1 == 1 {
                        r = r * b % m;
                    }
                    b = b * b % m;
                    e >>= 1;
                }
                r as u64
            },
            x,
        };
        println!(
            "  DL instance: h = {}^{} mod {} = {}",
            dl.g, dl.x, dl.p, dl.h
        );
        let exp_result = run_dl_experiment(&graph, &transform, &dl, args.steps);
        println!("  f_0 recovered: {}", exp_result.f0_recovered);
        println!("  DL solved:     {}", exp_result.dl_solved);
        println!("  Recovery time: {} ms", exp_result.recovery_time_ms);
        if exp_result.dl_solved {
            println!("  ⚠ Recovery succeeded — power map is INVERTIBLE at this scale");
        } else {
            println!("  ✓ Recovery failed — m-th root is hard at this scale");
        }
    }
}
