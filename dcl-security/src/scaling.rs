//! Security scaling benchmark — evaluates how security metrics change
//! as graph size increases from n_min to n_max.

use crate::cryptanalysis::{Cryptanalyzer, CryptanalysisConfig};
use dcl_core::graph::Graph;
use std::time::{Duration, Instant};

/// Result of a single scaling data point.
#[derive(Debug)]
pub struct ScalingPoint {
    pub graph_type: String,
    pub n: usize,
    pub edges: usize,
    pub search_space: f64,
    pub brute_force_success: f64,
    pub preimage_success: f64,
    pub collision_rate: f64,
    pub timing_correlation: f64,
    pub overall_level: String,
    pub wall_time: Duration,
}

/// Run scaling benchmark across multiple graph sizes.
pub fn run_scaling(
    graph_type: &str,
    sizes: &[usize],
    max_label: u64,
    power_exp: u32,
    modulus: Option<u64>,
) -> Vec<ScalingPoint> {
    use rayon::prelude::*;

    let gt = graph_type.to_string();
    let mut results: Vec<ScalingPoint> = sizes.par_iter()
        .filter_map(|&n| {
            let graph = match gt.as_str() {
                "path" => Graph::path(n),
                "cycle" => {
                    if n < 3 { return None; }
                    Graph::cycle(n)
                }
                "complete" => Graph::complete(n),
                _ => Graph::path(n),
            };

            let config = CryptanalysisConfig {
                max_label,
                power_map_exp: power_exp,
                test_iterations: 20,
                brute_force_samples: 30,
                modulus,
            };

            let start = Instant::now();
            let analyzer = Cryptanalyzer::with_config(&graph, config);
            let report = analyzer.analyze();
            let wall_time = start.elapsed();

            let search_space = (max_label as f64).powi(n.min(20) as i32);

            Some(ScalingPoint {
                graph_type: gt.clone(),
                n,
                edges: graph.edges().len(),
                search_space,
                brute_force_success: report.brute_force_analysis.success_rate,
                preimage_success: report.preimage_resistance.success_rate,
                collision_rate: report.collision_resistance.collision_rate,
                timing_correlation: report.timing_analysis.correlation_coefficient,
                overall_level: format!("{:?}", report.overall_security_level),
                wall_time,
            })
        })
        .collect();

    // Sort by n for consistent output order
    results.sort_by_key(|r| r.n);
    results
}

/// Print scaling results as a formatted table.
pub fn print_scaling_table(results: &[ScalingPoint]) {
    println!("\n{:<8} {:>4} {:>6} {:>14} {:>10} {:>10} {:>8} {:>8} {:>8}",
        "Graph", "n", "|E|", "Search Space", "BF Succ%", "Pre Succ%", "Coll%", "Corr", "Level");
    println!("{}", "-".repeat(90));

    for r in results {
        println!("{:<8} {:>4} {:>6} {:>14.2e} {:>9.2}% {:>9.2}% {:>7.2}% {:>8.4} {:>8}",
            r.graph_type, r.n, r.edges, r.search_space,
            r.brute_force_success, r.preimage_success,
            r.collision_rate, r.timing_correlation, r.overall_level);
    }

    println!("\nTotal wall time: {:.2?}",
        results.iter().map(|r| r.wall_time).sum::<Duration>());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_path_small() {
        let results = run_scaling("path", &[4, 6], 20, 2, None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].n, 4);
        assert_eq!(results[1].n, 6);
        // Search space should grow with n
        assert!(results[1].search_space > results[0].search_space);
    }
}
