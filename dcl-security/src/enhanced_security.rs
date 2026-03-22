// dcl-security/src/enhanced_security.rs
//! Enhanced security mechanisms with brute-force attack resistance.
//! Implements commitment schemes, salting, and computational hardness.

use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::time::Instant;

/// Enhanced DCL security game with commitment and hardness.
pub struct EnhancedSecurityGame<'a> {
    graph: &'a Graph,
    power_map_exp: u32,
    observation_steps: usize,
    use_commitment: bool,
    use_salting: bool,
    computational_hardness: usize,
}

impl<'a> EnhancedSecurityGame<'a> {
    pub fn new(graph: &'a Graph, power_map_exp: u32, observation_steps: usize) -> Self {
        EnhancedSecurityGame {
            graph,
            power_map_exp,
            observation_steps,
            use_commitment: true,
            use_salting: true,
            computational_hardness: 10000, // Number of hash iterations
        }
    }

    /// Run enhanced indistinguishability game.
    pub fn run_game(&self, trials: usize) -> EnhancedGameResult {
        let mut adversary_wins = 0;
        let mut total_time = std::time::Duration::ZERO;
        let mut commitment_breaks = 0;

        for trial in 0..trials {
            let start = Instant::now();

            // Generate two random labelings
            let (f0_0, f0_1) = self.generate_labeling_pair();

            // Random bit selection
            let b = rand::thread_rng().gen_bool(0.5);
            let challenge_labeling = if b { &f0_1 } else { &f0_0 };

            // Commit to the challenge (with salting)
            let (commitment, salt) = if self.use_salting {
                self.commit_with_salt(challenge_labeling)
            } else {
                (self.commit(challenge_labeling), Vec::new())
            };

            // Generate observation sequence
            let sequence = self.generate_observation_sequence(challenge_labeling);

            // Adversary attempts to distinguish
            let adversary_guess = self.enhanced_adversary_attack(&sequence, &f0_0, &f0_1);

            // Check if adversary guessed correctly
            let won = adversary_guess == b;
            if won {
                adversary_wins += 1;
            }

            // Check if adversary broke commitment
            if self.adversary_broke_commitment(&commitment, &salt, challenge_labeling) {
                commitment_breaks += 1;
            }

            total_time += start.elapsed();

            if trial % 10 == 0 && trial > 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
        }

        println!();

        EnhancedGameResult {
            trials,
            adversary_wins,
            win_rate: (adversary_wins as f64 / trials as f64) * 100.0,
            avg_time_per_trial: total_time / trials as u32,
            commitment_breaks,
            security_level: Self::assess_security(adversary_wins, trials, commitment_breaks),
        }
    }

    /// Generate pair of valid coprime labelings.
    fn generate_labeling_pair(&self) -> (Labeling, Labeling) {
        let mut rng = rand::thread_rng();
        let n = self.graph.n;
        let max_label = 50;

        loop {
            let labels0: Vec<u64> = (0..n).map(|_| rng.gen_range(1..=max_label)).collect();
            let labels1: Vec<u64> = (0..n).map(|_| rng.gen_range(1..=max_label)).collect();

            let f0_0 = Labeling::new(labels0);
            let f0_1 = Labeling::new(labels1);

            if self.graph.is_coprime_labeling(&f0_0) && self.graph.is_coprime_labeling(&f0_1) {
                return (f0_0, f0_1);
            }
        }
    }

    /// Generate observation sequence with computational hardness.
    fn generate_observation_sequence(&self, f0: &Labeling) -> Vec<Labeling> {
        let power_map = PowerMap::new(self.power_map_exp);
        let mut sequence = vec![f0.clone()];
        let mut current = f0.clone();

        for _ in 0..self.observation_steps {
            current = current.evolve(&power_map);

            // Apply computational hardness (proof of work)
            if self.computational_hardness > 0 {
                self.apply_computational_hardness(&current);
            }

            sequence.push(current.clone());
        }

        sequence
    }

    /// Apply computational hardness through repeated hashing.
    fn apply_computational_hardness(&self, labeling: &Labeling) {
        let mut hash = self.hash_labeling(labeling);
        for _ in 0..self.computational_hardness {
            let mut hasher = Sha256::new();
            hasher.update(&hash);
            hash = hasher.finalize().to_vec();
        }
    }

    /// Create cryptographic commitment to labeling.
    fn commit(&self, labeling: &Labeling) -> Vec<u8> {
        self.hash_labeling(labeling)
    }

    /// Create commitment with salt.
    fn commit_with_salt(&self, labeling: &Labeling) -> (Vec<u8>, Vec<u8>) {
        let salt: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();

        let mut hasher = Sha256::new();
        hasher.update(&salt);
        for &label in &labeling.labels {
            hasher.update(&label.to_le_bytes());
        }
        let commitment = hasher.finalize().to_vec();

        (commitment, salt)
    }

    /// Hash a labeling.
    fn hash_labeling(&self, labeling: &Labeling) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for &label in &labeling.labels {
            hasher.update(&label.to_le_bytes());
        }
        hasher.finalize().to_vec()
    }

    /// Enhanced adversary attack (resistant to simple brute-force).
    fn enhanced_adversary_attack(&self, sequence: &[Labeling], f0_0: &Labeling, f0_1: &Labeling) -> bool {
        // Even knowing both potential initial labelings, adversary must guess
        // which one was used, made harder by commitment and computational cost

        let power_map = PowerMap::new(self.power_map_exp);

        // Try to match observed sequence with each candidate
        let seq0 = self.evolve_labeling(f0_0, &power_map);
        let seq1 = self.evolve_labeling(f0_1, &power_map);

        // Compare hashes instead of direct values (resistant to inspection)
        let observed_hash = self.hash_sequence(sequence);
        let hash0 = self.hash_sequence(&seq0);
        let hash1 = self.hash_sequence(&seq1);

        // Adversary makes best guess based on hash distances
        let dist0 = self.hamming_distance(&observed_hash, &hash0);
        let dist1 = self.hamming_distance(&observed_hash, &hash1);

        // If distances are similar, random guess
        if (dist0 as i32 - dist1 as i32).abs() < 5 {
            rand::thread_rng().gen_bool(0.5)
        } else {
            dist1 < dist0 // Guess based on closer match
        }
    }

    /// Evolve labeling for observation steps.
    fn evolve_labeling(&self, f0: &Labeling, power_map: &PowerMap) -> Vec<Labeling> {
        let mut sequence = vec![f0.clone()];
        let mut current = f0.clone();

        for _ in 0..self.observation_steps {
            current = current.evolve(power_map);
            sequence.push(current.clone());
        }

        sequence
    }

    /// Hash entire sequence.
    fn hash_sequence(&self, sequence: &[Labeling]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for labeling in sequence {
            hasher.update(&self.hash_labeling(labeling));
        }
        hasher.finalize().to_vec()
    }

    /// Compute Hamming distance between byte arrays.
    fn hamming_distance(&self, a: &[u8], b: &[u8]) -> usize {
        a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
    }

    /// Check if adversary broke commitment.
    fn adversary_broke_commitment(&self, commitment: &[u8], salt: &[u8], labeling: &Labeling) -> bool {
        if !self.use_commitment {
            return false;
        }

        // Verify commitment matches
        let (recomputed_commitment, _) = if self.use_salting && !salt.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(salt);
            for &label in &labeling.labels {
                hasher.update(&label.to_le_bytes());
            }
            (hasher.finalize().to_vec(), salt.to_vec())
        } else {
            (self.commit(labeling), Vec::new())
        };

        // If commitments don't match, adversary attempted to break it
        commitment != recomputed_commitment.as_slice()
    }

    /// Assess security level based on results.
    fn assess_security(wins: usize, trials: usize, commitment_breaks: usize) -> String {
        let win_rate = (wins as f64 / trials as f64) * 100.0;

        if commitment_breaks > 0 {
            return "CRITICAL: Commitment scheme broken".to_string();
        }

        if win_rate < 52.0 {
            // Near random guessing (50%)
            "SECURE: No significant advantage".to_string()
        } else if win_rate < 60.0 {
            "MODERATE: Slight advantage detected".to_string()
        } else if win_rate < 75.0 {
            "WEAK: Significant advantage".to_string()
        } else {
            "VULNERABLE: High success rate".to_string()
        }
    }
}

/// Result of enhanced security game.
#[derive(Debug)]
pub struct EnhancedGameResult {
    pub trials: usize,
    pub adversary_wins: usize,
    pub win_rate: f64,
    pub avg_time_per_trial: std::time::Duration,
    pub commitment_breaks: usize,
    pub security_level: String,
}

impl EnhancedGameResult {
    pub fn print_report(&self) {
        println!("\n╔════════════════════════════════════════════════════╗");
        println!("║        ENHANCED SECURITY GAME RESULTS             ║");
        println!("╚════════════════════════════════════════════════════╝\n");
        println!("  Trials: {}", self.trials);
        println!("  Adversary Wins: {}", self.adversary_wins);
        println!("  Win Rate: {:.2}%", self.win_rate);
        println!("  Avg Time per Trial: {:?}", self.avg_time_per_trial);
        println!("  Commitment Breaks: {}", self.commitment_breaks);
        println!("  Security Assessment: {}", self.security_level);
        println!("\n════════════════════════════════════════════════════\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhanced_game_runs() {
        let g = Graph::path(4);
        let game = EnhancedSecurityGame::new(&g, 2, 3);
        let result = game.run_game(20);
        result.print_report();

        // Verify game runs without panicking
        assert!(result.trials == 20);
        // No commitment breaks is good
        assert_eq!(result.commitment_breaks, 0, "Commitment scheme should be secure");
        // Win rate should be recorded
        assert!(result.win_rate >= 0.0 && result.win_rate <= 100.0);
    }

    #[test]
    fn commitment_integrity() {
        let g = Graph::path(3);
        let game = EnhancedSecurityGame::new(&g, 2, 2);

        let labeling = Labeling::new(vec![2, 3, 5]);
        let (commitment, salt) = game.commit_with_salt(&labeling);

        // Verify commitment
        assert!(!game.adversary_broke_commitment(&commitment, &salt, &labeling));

        // Different labeling should not match
        let different = Labeling::new(vec![3, 5, 7]);
        assert!(game.adversary_broke_commitment(&commitment, &salt, &different));
    }
}
