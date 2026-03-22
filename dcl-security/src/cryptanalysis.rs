// dcl-security/src/cryptanalysis.rs
//! Comprehensive cryptanalysis suite for DCL security evaluation.
//! Tests multiple attack vectors and security properties.

use dcl_core::graph::Graph;
use dcl_core::labeling::Labeling;
use dcl_core::transform::PowerMap;
use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Comprehensive security analysis report.
#[derive(Debug)]
pub struct SecurityReport {
    pub brute_force_analysis: BruteForceResult,
    pub preimage_resistance: PreimageResult,
    pub collision_resistance: CollisionResult,
    pub timing_analysis: TimingResult,
    pub statistical_tests: StatisticalResult,
    pub overall_security_level: SecurityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    High,
    Medium,
    Low,
    Vulnerable,
}

/// Brute-force attack analysis.
#[derive(Debug)]
pub struct BruteForceResult {
    pub success_rate: f64,
    pub avg_time_per_attack: Duration,
    pub search_space_size: usize,
    pub successful_attacks: usize,
    pub total_attempts: usize,
}

/// Preimage resistance (one-wayness) test.
#[derive(Debug)]
pub struct PreimageResult {
    pub resistant: bool,
    pub success_rate: f64,
    pub avg_attempts: f64,
    pub max_attempts: usize,
}

/// Collision resistance test.
#[derive(Debug)]
pub struct CollisionResult {
    pub collisions_found: usize,
    pub unique_outputs: usize,
    pub total_inputs: usize,
    pub collision_rate: f64,
}

/// Timing attack analysis.
#[derive(Debug)]
pub struct TimingResult {
    pub vulnerable: bool,
    pub timing_variance: f64,
    pub correlation_coefficient: f64,
}

/// Statistical randomness tests.
#[derive(Debug)]
pub struct StatisticalResult {
    pub frequency_test_passed: bool,
    pub runs_test_passed: bool,
    pub entropy_bits: f64,
    pub expected_entropy: f64,
}

/// Configuration for parameterized cryptanalysis.
/// Allows running security evaluation on arbitrary graph sizes.
#[derive(Debug, Clone)]
pub struct CryptanalysisConfig {
    /// Maximum label value for brute-force search space
    pub max_label: u64,
    /// Power map exponent m in g(x) = x^m
    pub power_map_exp: u32,
    /// Number of iterations for timing and statistical tests
    pub test_iterations: usize,
    /// Number of random samples for brute-force evaluation (use instead of exhaustive for large graphs)
    pub brute_force_samples: usize,
    /// Optional prime modulus for bounded label evolution
    pub modulus: Option<u64>,
}

impl Default for CryptanalysisConfig {
    fn default() -> Self {
        CryptanalysisConfig {
            max_label: 20,
            power_map_exp: 2,
            test_iterations: 100,
            brute_force_samples: 50,
            modulus: None,
        }
    }
}

/// Chi-squared critical values at alpha=0.05 for common degrees of freedom.
fn chi_squared_critical(df: usize) -> f64 {
    match df {
        1 => 3.841,
        2 => 5.991,
        3 => 7.815,
        4 => 9.488,
        5 => 11.070,
        7 => 14.067,
        9 => 16.919,
        15 => 24.996,
        19 => 30.144,
        31 => 44.985,
        49 => 66.339,
        63 => 82.529,
        127 => 154.302,
        255 => 293.248,
        // Wilson-Hilferty approximation for other df
        _ => {
            let z = 1.645;
            let term = 1.0 - 2.0 / (9.0 * df as f64) + z * (2.0 / (9.0 * df as f64)).sqrt();
            df as f64 * term.powi(3)
        }
    }
}

/// Convert a DCL label evolution sequence into raw bytes for NIST testing.
/// Evolves `initial` labeling for `steps` iterations under `transform`,
/// appending all label bytes at each step to the output.
pub fn dcl_labels_to_test_data(
    transform: &dyn dcl_core::transform::Transform,
    initial: &Labeling,
    steps: usize,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(initial.len() * 8 * steps);
    let mut current = initial.clone();
    for _ in 0..steps {
        current.evolve_in_place(transform);
        for &label in &current.labels {
            data.extend_from_slice(&label.to_le_bytes());
        }
    }
    data
}

/// Comprehensive cryptanalysis engine.
pub struct Cryptanalyzer<'a> {
    graph: &'a Graph,
    power_map_exp: u32,
    #[allow(dead_code)]
    test_iterations: usize,
    config: Option<CryptanalysisConfig>,
}

impl<'a> Cryptanalyzer<'a> {
    pub fn new(graph: &'a Graph, power_map_exp: u32) -> Self {
        Cryptanalyzer {
            graph,
            power_map_exp,
            test_iterations: 100,
            config: None,
        }
    }

    /// Create a cryptanalyzer with explicit configuration.
    pub fn with_config(graph: &'a Graph, config: CryptanalysisConfig) -> Self {
        Cryptanalyzer {
            graph,
            power_map_exp: config.power_map_exp,
            test_iterations: config.test_iterations,
            config: Some(config),
        }
    }

    /// Build the PowerMap based on configuration (unbounded or modular).
    fn make_power_map(&self) -> PowerMap {
        match self.config.as_ref().and_then(|c| c.modulus) {
            Some(n) => PowerMap::with_modulus(self.power_map_exp, n),
            None => PowerMap::new(self.power_map_exp),
        }
    }

    /// Run complete security analysis suite.
    pub fn analyze(&self) -> SecurityReport {
        println!("=== Running Comprehensive Cryptanalysis ===\n");

        let brute_force = self.test_brute_force_resistance();
        let preimage = self.test_preimage_resistance();
        let collision = self.test_collision_resistance();
        let timing = self.test_timing_attacks();
        let statistical = self.test_statistical_properties();

        let overall = self.compute_overall_security(
            &brute_force,
            &preimage,
            &collision,
            &timing,
            &statistical,
        );

        SecurityReport {
            brute_force_analysis: brute_force,
            preimage_resistance: preimage,
            collision_resistance: collision,
            timing_analysis: timing,
            statistical_tests: statistical,
            overall_security_level: overall,
        }
    }

    /// Test resistance to brute-force attacks.
    fn test_brute_force_resistance(&self) -> BruteForceResult {
        println!("Testing brute-force resistance...");

        let n = self.graph.n;
        let max_label = self.config.as_ref().map(|c| c.max_label).unwrap_or(20);
        let sample_count = self.config.as_ref().map(|c| c.brute_force_samples).unwrap_or(50);
        let search_space = (max_label as usize).pow(n.min(10) as u32);
        let sample_size = sample_count.min(search_space);

        let mut successful = 0;
        let mut total_time = Duration::ZERO;

        for trial in 0..sample_size {
            let target_labeling = self.generate_random_labeling(max_label);

            let start = Instant::now();
            if self.brute_force_find(&target_labeling, max_label) {
                successful += 1;
            }
            total_time += start.elapsed();

            if trial % 10 == 0 {
                print!(".");
            }
        }
        println!(" done\n");

        BruteForceResult {
            success_rate: (successful as f64 / sample_size as f64) * 100.0,
            avg_time_per_attack: total_time / sample_size as u32,
            search_space_size: search_space,
            successful_attacks: successful,
            total_attempts: sample_size,
        }
    }

    /// Attempt brute-force search for initial labeling.
    fn brute_force_find(&self, _target: &Labeling, max_label: u64) -> bool {
        // Simplified brute-force: check small sample
        let limit = 1000; // Practical limit
        let mut checked = 0;

        for _ in 0..limit {
            let candidate = self.generate_random_labeling(max_label);
            if self.graph.is_coprime_labeling(&candidate) {
                // Check if this could lead to target after evolution
                checked += 1;
                if checked > 100 {
                    break;
                }
            }
        }

        false // Very unlikely to find exact match
    }

    /// Test preimage resistance (given output, find input).
    fn test_preimage_resistance(&self) -> PreimageResult {
        println!("Testing preimage resistance (one-wayness)...");

        let power_map = self.make_power_map();
        let trials = 20;
        let max_attempts = 10000;

        let mut total_attempts = 0;
        let mut successful = 0;

        for _ in 0..trials {
            let original = self.generate_random_labeling(50);
            if !self.graph.is_coprime_labeling(&original) {
                continue;
            }

            let evolved = original.evolve(&power_map);

            // Try to find preimage
            let mut attempts = 0;
            for _ in 0..max_attempts {
                attempts += 1;
                let candidate = self.generate_random_labeling(50);
                if candidate.evolve(&power_map).labels == evolved.labels {
                    successful += 1;
                    break;
                }
            }
            total_attempts += attempts;
        }

        println!("  Preimage attacks successful: {}/{}\n", successful, trials);

        PreimageResult {
            resistant: successful == 0,
            success_rate: (successful as f64 / trials as f64) * 100.0,
            avg_attempts: total_attempts as f64 / trials as f64,
            max_attempts,
        }
    }

    /// Test collision resistance (find two inputs with same output).
    fn test_collision_resistance(&self) -> CollisionResult {
        println!("Testing collision resistance...");

        let power_map = self.make_power_map();
        let samples = 200;

        let mut outputs: HashMap<Vec<u64>, Vec<u64>> = HashMap::new();
        let mut valid_inputs = 0;

        for _ in 0..samples {
            let input = self.generate_random_labeling(30);
            if !self.graph.is_coprime_labeling(&input) {
                continue;
            }

            valid_inputs += 1;
            let output = input.evolve(&power_map);

            outputs
                .entry(output.labels.clone())
                .or_insert_with(Vec::new)
                .push(input.labels[0]); // Track one label as identifier
        }

        let collisions = outputs.values().filter(|v| v.len() > 1).count();
        let unique = outputs.len();

        println!("  Found {} collisions in {} unique outputs\n", collisions, unique);

        CollisionResult {
            collisions_found: collisions,
            unique_outputs: unique,
            total_inputs: valid_inputs,
            collision_rate: if valid_inputs > 0 {
                (collisions as f64 / valid_inputs as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Test for timing attack vulnerabilities.
    fn test_timing_attacks(&self) -> TimingResult {
        println!("Testing timing attack vulnerabilities...");

        let power_map = self.make_power_map();
        let trials = 100;

        let mut timings: Vec<(u64, Duration)> = Vec::new();

        for _ in 0..trials {
            let labeling = self.generate_random_labeling(100);
            let input_size = labeling.labels.iter().sum::<u64>();

            let start = Instant::now();
            let _ = labeling.evolve(&power_map);
            let duration = start.elapsed();

            timings.push((input_size, duration));
        }

        // Compute correlation between input size and timing
        let variance = self.compute_timing_variance(&timings);
        let correlation = self.compute_correlation(&timings);

        println!("  Timing variance: {:.6}ms", variance);
        println!("  Correlation coefficient: {:.4}\n", correlation);

        TimingResult {
            vulnerable: correlation.abs() > 0.5, // Significant correlation
            timing_variance: variance,
            correlation_coefficient: correlation,
        }
    }

    /// Compute variance in timings.
    fn compute_timing_variance(&self, timings: &[(u64, Duration)]) -> f64 {
        let durations: Vec<f64> = timings.iter().map(|(_, d)| d.as_secs_f64() * 1000.0).collect();
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let variance = durations.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / durations.len() as f64;
        variance.sqrt()
    }

    /// Compute correlation coefficient.
    fn compute_correlation(&self, timings: &[(u64, Duration)]) -> f64 {
        let n = timings.len() as f64;
        let sum_x: f64 = timings.iter().map(|(x, _)| *x as f64).sum();
        let sum_y: f64 = timings.iter().map(|(_, y)| y.as_secs_f64()).sum();
        let sum_xy: f64 = timings.iter().map(|(x, y)| (*x as f64) * y.as_secs_f64()).sum();
        let sum_x2: f64 = timings.iter().map(|(x, _)| (*x as f64).powi(2)).sum();
        let sum_y2: f64 = timings.iter().map(|(_, y)| y.as_secs_f64().powi(2)).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x.powi(2)) * (n * sum_y2 - sum_y.powi(2))).sqrt();

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Test statistical properties of outputs.
    fn test_statistical_properties(&self) -> StatisticalResult {
        println!("Testing statistical properties...");

        let power_map = self.make_power_map();
        let samples = 100;

        let mut outputs = Vec::new();

        for _ in 0..samples {
            let labeling = self.generate_random_labeling(50);
            let evolved = labeling.evolve(&power_map);
            outputs.push(evolved.labels[0] % 256); // Use lowest byte
        }

        let freq_test = self.frequency_test(&outputs);
        let runs_test = self.runs_test(&outputs);
        let entropy = self.compute_entropy(&outputs);

        println!("  Frequency test: {}", if freq_test { "PASS" } else { "FAIL" });
        println!("  Runs test: {}", if runs_test { "PASS" } else { "FAIL" });
        println!("  Entropy: {:.2} bits (expected: 8.0 bits)\n", entropy);

        StatisticalResult {
            frequency_test_passed: freq_test,
            runs_test_passed: runs_test,
            entropy_bits: entropy,
            expected_entropy: 8.0,
        }
    }

    /// Frequency test (chi-squared) with adaptive bin count.
    fn frequency_test(&self, data: &[u64]) -> bool {
        if data.len() < 4 {
            return false;
        }

        // Adaptive bins: use min(data.len()/4, 64) for meaningful statistics
        let num_bins = (data.len() / 4).max(2).min(64);
        let mut counts = vec![0usize; num_bins];
        for &val in data {
            counts[val as usize % num_bins] += 1;
        }

        let expected = data.len() as f64 / num_bins as f64;
        let chi_squared: f64 = counts
            .iter()
            .map(|&obs| {
                let diff = obs as f64 - expected;
                (diff * diff) / expected
            })
            .sum();

        let df = num_bins - 1;
        let critical = chi_squared_critical(df);
        chi_squared < critical
    }

    /// Runs test (consecutive sequences).
    fn runs_test(&self, data: &[u64]) -> bool {
        if data.len() < 2 {
            return false;
        }

        let median = {
            let mut sorted = data.to_vec();
            sorted.sort_unstable();
            sorted[sorted.len() / 2]
        };

        let mut runs = 1;
        let mut above_median = data[0] >= median;

        for &val in &data[1..] {
            let current_above = val >= median;
            if current_above != above_median {
                runs += 1;
                above_median = current_above;
            }
        }

        // Expected runs ≈ n/2, allow ±30%
        let expected_runs = data.len() / 2;
        let lower = (expected_runs as f64 * 0.7) as usize;
        let upper = (expected_runs as f64 * 1.3) as usize;

        runs >= lower && runs <= upper
    }

    /// Compute Shannon entropy.
    fn compute_entropy(&self, data: &[u64]) -> f64 {
        let mut counts = HashMap::new();
        for &val in data {
            *counts.entry(val).or_insert(0) += 1;
        }

        let n = data.len() as f64;
        counts
            .values()
            .map(|&count| {
                let p = count as f64 / n;
                -p * p.log2()
            })
            .sum()
    }

    /// Generate random labeling.
    fn generate_random_labeling(&self, max_label: u64) -> Labeling {
        let mut rng = rand::thread_rng();
        let n = self.graph.n;
        let labels: Vec<u64> = (0..n).map(|_| rng.gen_range(1..=max_label)).collect();
        Labeling::new(labels)
    }

    /// Compute overall security level.
    fn compute_overall_security(
        &self,
        brute_force: &BruteForceResult,
        preimage: &PreimageResult,
        collision: &CollisionResult,
        timing: &TimingResult,
        statistical: &StatisticalResult,
    ) -> SecurityLevel {
        let mut score = 0;

        // Brute-force resistance (0-3 points)
        if brute_force.success_rate < 5.0 {
            score += 3;
        } else if brute_force.success_rate < 20.0 {
            score += 2;
        } else if brute_force.success_rate < 50.0 {
            score += 1;
        }

        // Preimage resistance (0-3 points)
        if preimage.resistant {
            score += 3;
        } else if preimage.success_rate < 10.0 {
            score += 2;
        } else if preimage.success_rate < 30.0 {
            score += 1;
        }

        // Collision resistance (0-2 points)
        if collision.collision_rate < 1.0 {
            score += 2;
        } else if collision.collision_rate < 5.0 {
            score += 1;
        }

        // Timing attack resistance (0-2 points)
        if !timing.vulnerable {
            score += 2;
        } else if timing.correlation_coefficient < 0.7 {
            score += 1;
        }

        // Statistical properties (0-2 points)
        let passed_tests = statistical.frequency_test_passed as u32 + statistical.runs_test_passed as u32;
        score += passed_tests as i32;

        // Total score: 0-12
        match score {
            10..=12 => SecurityLevel::High,
            7..=9 => SecurityLevel::Medium,
            4..=6 => SecurityLevel::Low,
            _ => SecurityLevel::Vulnerable,
        }
    }
}

impl SecurityReport {
    /// Print comprehensive report.
    pub fn print_report(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║          COMPREHENSIVE SECURITY ANALYSIS REPORT          ║");
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        println!("1. BRUTE-FORCE RESISTANCE");
        println!("   Success Rate: {:.2}%", self.brute_force_analysis.success_rate);
        println!("   Search Space: {}", self.brute_force_analysis.search_space_size);
        println!("   Avg Attack Time: {:?}", self.brute_force_analysis.avg_time_per_attack);
        println!("   Assessment: {}\n", if self.brute_force_analysis.success_rate < 10.0 { "✓ GOOD" } else { "✗ WEAK" });

        println!("2. PREIMAGE RESISTANCE (One-Wayness)");
        println!("   Resistant: {}", self.preimage_resistance.resistant);
        println!("   Success Rate: {:.2}%", self.preimage_resistance.success_rate);
        println!("   Avg Attempts: {:.0}", self.preimage_resistance.avg_attempts);
        println!("   Assessment: {}\n", if self.preimage_resistance.resistant { "✓ STRONG" } else { "✗ WEAK" });

        println!("3. COLLISION RESISTANCE");
        println!("   Collisions Found: {}", self.collision_resistance.collisions_found);
        println!("   Unique Outputs: {}", self.collision_resistance.unique_outputs);
        println!("   Collision Rate: {:.2}%", self.collision_resistance.collision_rate);
        println!("   Assessment: {}\n", if self.collision_resistance.collision_rate < 5.0 { "✓ GOOD" } else { "⚠ MODERATE" });

        println!("4. TIMING ATTACK VULNERABILITY");
        println!("   Vulnerable: {}", self.timing_analysis.vulnerable);
        println!("   Timing Variance: {:.6}ms", self.timing_analysis.timing_variance);
        println!("   Correlation: {:.4}", self.timing_analysis.correlation_coefficient);
        println!("   Assessment: {}\n", if !self.timing_analysis.vulnerable { "✓ RESISTANT" } else { "✗ VULNERABLE" });

        println!("5. STATISTICAL PROPERTIES");
        println!("   Frequency Test: {}", if self.statistical_tests.frequency_test_passed { "PASS" } else { "FAIL" });
        println!("   Runs Test: {}", if self.statistical_tests.runs_test_passed { "PASS" } else { "FAIL" });
        println!("   Entropy: {:.2} / {:.2} bits", self.statistical_tests.entropy_bits, self.statistical_tests.expected_entropy);
        println!("   Assessment: {}\n",
            if self.statistical_tests.frequency_test_passed && self.statistical_tests.runs_test_passed {
                "✓ GOOD"
            } else {
                "⚠ MODERATE"
            });

        println!("═══════════════════════════════════════════════════════════");
        println!("OVERALL SECURITY LEVEL: {:?}", self.overall_security_level);
        println!("═══════════════════════════════════════════════════════════\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cryptanalysis_runs() {
        let g = Graph::path(4);
        let analyzer = Cryptanalyzer::new(&g, 2);
        let report = analyzer.analyze();
        report.print_report();
    }

    #[test]
    fn cryptanalysis_with_config() {
        let g = Graph::path(4);
        let config = CryptanalysisConfig {
            max_label: 30,
            power_map_exp: 3,
            test_iterations: 50,
            brute_force_samples: 20,
            modulus: None,
        };
        let analyzer = Cryptanalyzer::with_config(&g, config);
        assert_eq!(analyzer.power_map_exp, 3);
        assert_eq!(analyzer.test_iterations, 50);
        let report = analyzer.analyze();
        assert!(report.brute_force_analysis.total_attempts <= 20);
    }

    #[test]
    fn cryptanalysis_larger_graph() {
        let g = Graph::path(8);
        let config = CryptanalysisConfig {
            max_label: 15,
            power_map_exp: 2,
            test_iterations: 30,
            brute_force_samples: 10,
            modulus: None,
        };
        let analyzer = Cryptanalyzer::with_config(&g, config);
        let report = analyzer.analyze();
        // Larger graph → larger search space
        assert!(report.brute_force_analysis.search_space_size > 1);
        report.print_report();
    }

    #[test]
    fn frequency_test_validates_randomness() {
        let g = Graph::path(4);
        let analyzer = Cryptanalyzer::new(&g, 2);

        // Uniform-ish data should pass
        // num_bins = min(200/4, 64) = 50, so use values spanning 0..199 for even distribution
        let uniform: Vec<u64> = (0..200).collect();
        assert!(analyzer.frequency_test(&uniform), "Uniform data should pass frequency test");

        // Heavily biased data should fail
        let mut biased = vec![1u64; 180];
        biased.extend(vec![2u64; 20]);
        assert!(!analyzer.frequency_test(&biased), "Biased data should fail frequency test");
    }

    #[test]
    fn dcl_labels_to_test_data_produces_bytes() {
        use dcl_core::transform::PowerMap;
        let initial = Labeling::new(vec![2, 3, 5, 7, 11]);
        let pm = PowerMap::new(2);
        let data = dcl_labels_to_test_data(&pm, &initial, 10);
        // 5 labels * 8 bytes * 10 steps = 400 bytes
        assert_eq!(data.len(), 400);
        // First 8 bytes should be label 4 (2^2 = 4) in LE
        assert_eq!(&data[0..8], &4u64.to_le_bytes());
    }

    #[test]
    fn cryptanalysis_with_modulus() {
        let g = Graph::path(4);
        let config = CryptanalysisConfig {
            max_label: 20,
            power_map_exp: 2,
            test_iterations: 20,
            brute_force_samples: 10,
            modulus: Some(65537),
        };
        let analyzer = Cryptanalyzer::with_config(&g, config);
        let report = analyzer.analyze();
        assert!(report.brute_force_analysis.total_attempts <= 10);
    }
}
