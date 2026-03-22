// dcl-security/src/game.rs
//! The DCL Indistinguishability Game.
//! Formalizes cryptographic security of DCL sequences.
//!
//! Game definition (from brainstorm §3):
//! - Challenger generates (G, g, f_0, f_1, ..., f_T)
//! - Adversary sees (G, g, f_1, ..., f_T) — the initial labeling is hidden
//! - Adversary outputs a guess for f_0
//! - Adversary wins if guess matches the real f_0

use crate::adversary::Adversary;
use dcl_core::dcl::DclSequence;
use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;

/// Configuration for the indistinguishability game.
pub struct GameConfig {
    /// Number of DCL steps the adversary observes (f_1, ..., f_T).
    pub steps: usize,
    /// Number of times to run the game (for statistical win-rate).
    pub trials: usize,
}

/// Result of running the game for multiple trials.
pub struct GameResult {
    pub adversary_name: &'static str,
    pub trials: usize,
    pub wins: usize,
    pub win_rate: f64,
}

impl GameResult {
    pub fn print(&self) {
        println!("=== Indistinguishability Game ===");
        println!("  Adversary  : {}", self.adversary_name);
        println!("  Trials     : {}", self.trials);
        println!("  Wins       : {}", self.wins);
        println!("  Win rate   : {:.4}%", self.win_rate * 100.0);
        if self.win_rate < 0.1 {
            println!("  Assessment : ✓ No significant advantage (secure)");
        } else {
            println!("  Assessment : ✗ Adversary shows non-negligible advantage");
        }
    }
}

/// Run the indistinguishability game for multiple trials with a given adversary.
pub fn run_game(
    graph: &Graph,
    transform: &PowerMap,
    f0_sequence: &[Labeling], // Pre-generated f_0 labelings for each trial
    config: &GameConfig,
    adversary: &dyn Adversary,
) -> GameResult {
    let mut wins = 0;

    for (trial, f0) in f0_sequence.iter().take(config.trials).enumerate() {
        // Generate DCL sequence starting from this f_0
        let mut seq = DclSequence::new(graph, transform, f0.clone());
        let mut observed = Vec::with_capacity(config.steps);
        for _ in 0..config.steps {
            seq.advance();
            observed.push(seq.current.clone());
        }

        // Adversary guesses f_0 from observed sequence
        let guess = adversary.guess_f0(graph, transform, &observed);

        // Check if guess matches
        if guess == *f0 {
            wins += 1;
            if trial < 3 {
                println!("  Trial {trial}: adversary won! f_0={f0}");
            }
        }
    }

    let win_rate = wins as f64 / config.trials.min(f0_sequence.len()) as f64;
    GameResult {
        adversary_name: adversary.name(),
        trials: config.trials.min(f0_sequence.len()),
        wins,
        win_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adversary::{BruteForceAdversary, RandomAdversary};
    use dcl_core::graph::Graph;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::PowerMap;

    fn make_f0_set(n: usize, count: usize) -> Vec<Labeling> {
        // Use prime-based labelings as the f_0 set
        let primes = [1u64, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31];
        (0..count)
            .map(|i| {
                let labels: Vec<u64> = (0..n).map(|j| primes[(i + j) % primes.len()]).collect();
                // Ensure all distinct
                let mut used = std::collections::HashSet::new();
                let labels: Vec<u64> = labels
                    .into_iter()
                    .enumerate()
                    .map(|(idx, l)| {
                        let mut v = l;
                        while used.contains(&v) {
                            v += idx as u64 + 1;
                        }
                        used.insert(v);
                        v
                    })
                    .collect();
                Labeling::new(labels)
            })
            .collect()
    }

    #[test]
    fn random_adversary_low_win_rate() {
        let g = Graph::path(4);
        let pm = PowerMap::new(2);
        let f0_set = make_f0_set(4, 20);
        let config = GameConfig {
            steps: 5,
            trials: 20,
        };
        let adv = RandomAdversary { max_label: 20 };
        let result = run_game(&g, &pm, &f0_set, &config, &adv);
        result.print();
        // Random adversary shouldn't win consistently
        assert!(
            result.win_rate < 0.5,
            "Random adversary should have low win rate"
        );
    }

    #[test]
    fn brute_force_adversary_game() {
        let g = Graph::path(3);
        let pm = PowerMap::new(2);
        // Use a tiny labeling space so brute force can actually try inversion
        let f0_set = vec![Labeling::new(vec![1, 2, 3])];
        let config = GameConfig {
            steps: 3,
            trials: 1,
        };
        let adv = BruteForceAdversary { max_label: 10 };
        let result = run_game(&g, &pm, &f0_set, &config, &adv);
        result.print();
        // Just verify no panic — correctness depends on invertibility of power map
    }
}
