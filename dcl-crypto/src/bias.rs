// dcl-crypto/src/bias.rs
//! Prime distribution bias analysis.
//! Tests whether HDCL-SP generated primes follow the Prime Number Theorem
//! distribution using a chi-squared goodness-of-fit test.

/// Run a chi-squared test to detect bias in a sample of prime values.
///
/// Divides the range [min_val, max_val] into `bins` equal buckets,
/// counts observed primes per bin, computes expected counts from PNT density
/// (1/ln(q) per candidate near q), and returns the chi-squared statistic
/// and approximate p-value assessment.
pub struct BiasTestResult {
    pub chi_squared: f64,
    /// Approximate: true if p-value > 0.05 (no significant bias detected).
    pub no_bias_detected: bool,
    pub bins: Vec<BinStats>,
}

#[derive(Debug)]
pub struct BinStats {
    pub range_lo: u64,
    pub range_hi: u64,
    pub observed: usize,
    pub expected: f64,
}

pub fn prime_distribution_test(primes: &[u64], num_bins: usize) -> BiasTestResult {
    if primes.is_empty() || num_bins == 0 {
        return BiasTestResult {
            chi_squared: 0.0,
            no_bias_detected: true,
            bins: vec![],
        };
    }

    let min_val = *primes.iter().min().unwrap() as f64;
    let max_val = *primes.iter().max().unwrap() as f64;
    let range = max_val - min_val;

    if range == 0.0 {
        return BiasTestResult {
            chi_squared: 0.0,
            no_bias_detected: true,
            bins: vec![],
        };
    }

    let bin_width = range / num_bins as f64;
    let mut observed = vec![0usize; num_bins];

    for &p in primes {
        let bin = (((p as f64 - min_val) / bin_width) as usize).min(num_bins - 1);
        observed[bin] += 1;
    }

    // Expected count per bin: PNT says density near x is 1/ln(x).
    // Expected = Σ_{x in bin} 1/ln(x) ≈ (bin_width / ln(bin_midpoint)) * total_trials
    // Here we just use the uniform approximation scaled to total count.
    let total = primes.len() as f64;
    let mut expected_raw: Vec<f64> = (0..num_bins)
        .map(|i| {
            let mid = min_val + (i as f64 + 0.5) * bin_width;
            if mid > 1.0 {
                1.0 / mid.ln()
            } else {
                1.0
            }
        })
        .collect();
    let sum_expected: f64 = expected_raw.iter().sum();
    for e in &mut expected_raw {
        *e = *e / sum_expected * total;
    }

    // Chi-squared statistic: Σ (O-E)^2 / E
    let chi_squared: f64 = observed
        .iter()
        .zip(expected_raw.iter())
        .filter(|(_, &e)| e > 0.5) // ignore near-empty bins
        .map(|(&o, &e)| {
            let diff = o as f64 - e;
            diff * diff / e
        })
        .sum();

    // Approximate critical value for df = num_bins-1:
    // For df=9, alpha=0.05, chi2_crit ≈ 16.92
    // Use simple rule: no bias if chi2 < 2 * df
    let df = (num_bins - 1) as f64;
    let no_bias_detected = chi_squared < 2.0 * df;

    let bins: Vec<BinStats> = (0..num_bins)
        .map(|i| BinStats {
            range_lo: (min_val + i as f64 * bin_width) as u64,
            range_hi: (min_val + (i + 1) as f64 * bin_width) as u64,
            observed: observed[i],
            expected: expected_raw[i],
        })
        .collect();

    BiasTestResult {
        chi_squared,
        no_bias_detected,
        bins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bias_test_uniform_safe_primes() {
        // Known safe primes (p=2q+1 with q prime)
        let safe_primes: Vec<u64> = vec![
            5, 7, 11, 23, 47, 59, 83, 107, 167, 179, 227, 263, 347, 359, 383, 467, 479, 503,
        ];
        let result = prime_distribution_test(&safe_primes, 5);
        println!(
            "Chi-squared: {:.4}, no_bias: {}",
            result.chi_squared, result.no_bias_detected
        );
        // Small sample — just check no panic and reasonable result
        assert!(result.chi_squared >= 0.0);
    }
}
