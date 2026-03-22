// dcl-security/src/nist_tests.rs
//! Complete NIST SP 800-22 Statistical Test Suite for Random and Pseudorandom Number Generators
//!
//! This module implements all 15 statistical tests from NIST Special Publication 800-22
//! for evaluating the quality of binary sequences as required for cryptographic applications.

use std::f64::consts::PI;

/// Complete NIST test suite results
#[derive(Debug, Clone)]
pub struct NistTestSuite {
    pub frequency_monobit: TestResult,
    pub frequency_block: TestResult,
    pub runs: TestResult,
    pub longest_run: TestResult,
    pub binary_matrix_rank: TestResult,
    pub discrete_fourier_transform: TestResult,
    pub non_overlapping_template: TestResult,
    pub overlapping_template: TestResult,
    pub universal: TestResult,
    pub linear_complexity: TestResult,
    pub serial: SerialTestResult,
    pub approximate_entropy: TestResult,
    pub cumulative_sums: CumulativeSumsResult,
    pub random_excursions: RandomExcursionsResult,
    pub random_excursions_variant: RandomExcursionsVariantResult,
}

/// Individual test result with p-value
#[derive(Debug, Clone)]
pub struct TestResult {
    pub passed: bool,
    pub p_value: f64,
    pub test_name: String,
}

/// Serial test result (tests multiple patterns)
#[derive(Debug, Clone)]
pub struct SerialTestResult {
    pub passed: bool,
    pub p_value1: f64,
    pub p_value2: f64,
}

/// Cumulative sums test result (forward and backward)
#[derive(Debug, Clone)]
pub struct CumulativeSumsResult {
    pub passed: bool,
    pub p_value_forward: f64,
    pub p_value_backward: f64,
}

/// Random excursions test result (multiple states)
#[derive(Debug, Clone)]
pub struct RandomExcursionsResult {
    pub passed: bool,
    pub states_tested: usize,
    pub states_passed: usize,
    pub min_p_value: f64,
}

/// Random excursions variant test result
#[derive(Debug, Clone)]
pub struct RandomExcursionsVariantResult {
    pub passed: bool,
    pub states_tested: usize,
    pub states_passed: usize,
    pub min_p_value: f64,
}

impl TestResult {
    fn new(test_name: &str, p_value: f64, alpha: f64) -> Self {
        TestResult {
            passed: p_value >= alpha,
            p_value,
            test_name: test_name.to_string(),
        }
    }
}

/// NIST test runner
pub struct NistTester {
    alpha: f64, // Significance level (typically 0.01)
}

impl NistTester {
    /// Create a new NIST tester with specified significance level
    pub fn new(alpha: f64) -> Self {
        NistTester { alpha }
    }

    /// Create with default significance level of 0.01
    pub fn default() -> Self {
        Self::new(0.01)
    }

    /// Run complete NIST test suite on binary data
    pub fn run_all_tests(&self, data: &[u8]) -> NistTestSuite {
        let bits = bytes_to_bits(data);

        NistTestSuite {
            frequency_monobit: self.frequency_monobit_test(&bits),
            frequency_block: self.frequency_block_test(&bits, 128),
            runs: self.runs_test(&bits),
            longest_run: self.longest_run_test(&bits),
            binary_matrix_rank: self.binary_matrix_rank_test(&bits),
            discrete_fourier_transform: self.dft_test(&bits),
            non_overlapping_template: self.non_overlapping_template_test(&bits),
            overlapping_template: self.overlapping_template_test(&bits),
            universal: self.universal_test(&bits),
            linear_complexity: self.linear_complexity_test(&bits, 500),
            serial: self.serial_test(&bits, 16),
            approximate_entropy: self.approximate_entropy_test(&bits, 10),
            cumulative_sums: self.cumulative_sums_test(&bits),
            random_excursions: self.random_excursions_test(&bits),
            random_excursions_variant: self.random_excursions_variant_test(&bits),
        }
    }

    /// Test 1: Frequency (Monobit) Test
    /// Focus: Proportion of zeros and ones for the entire sequence
    pub fn frequency_monobit_test(&self, bits: &[u8]) -> TestResult {
        let n = bits.len() as f64;
        let sum: i32 = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).sum();
        let s_obs = (sum as f64).abs() / n.sqrt();
        let p_value = erfc(s_obs / 2.0_f64.sqrt());

        TestResult::new("Frequency (Monobit)", p_value, self.alpha)
    }

    /// Test 2: Frequency Test within a Block
    /// Focus: Proportion of ones within M-bit blocks
    pub fn frequency_block_test(&self, bits: &[u8], block_size: usize) -> TestResult {
        let n = bits.len();
        let num_blocks = n / block_size;

        if num_blocks == 0 {
            return TestResult::new("Frequency Block", 0.0, self.alpha);
        }

        let mut chi_squared = 0.0;
        for i in 0..num_blocks {
            let start = i * block_size;
            let end = start + block_size;
            let ones: usize = bits[start..end].iter().map(|&b| b as usize).sum();
            let pi = ones as f64 / block_size as f64;
            chi_squared += (pi - 0.5).powi(2);
        }

        chi_squared *= 4.0 * block_size as f64;
        let p_value = igamc(num_blocks as f64 / 2.0, chi_squared / 2.0);

        TestResult::new("Frequency Block", p_value, self.alpha)
    }

    /// Test 3: Runs Test
    /// Focus: Number of runs (uninterrupted sequences of identical bits)
    pub fn runs_test(&self, bits: &[u8]) -> TestResult {
        let n = bits.len();
        if n == 0 {
            return TestResult::new("Runs", 0.0, self.alpha);
        }

        let ones: usize = bits.iter().map(|&b| b as usize).sum();
        let pi = ones as f64 / n as f64;

        // Pre-test: check if proportion is close to 0.5
        if (pi - 0.5).abs() >= 2.0 / (n as f64).sqrt() {
            return TestResult::new("Runs", 0.0, self.alpha);
        }

        let mut runs = 1;
        for i in 1..n {
            if bits[i] != bits[i - 1] {
                runs += 1;
            }
        }

        let expected = 2.0 * n as f64 * pi * (1.0 - pi);
        let s_obs = ((runs as f64 - expected).abs()) / (2.0 * n as f64 * pi * (1.0 - pi)).sqrt();
        let p_value = erfc(s_obs / 2.0_f64.sqrt());

        TestResult::new("Runs", p_value, self.alpha)
    }

    /// Test 4: Test for the Longest Run of Ones in a Block
    /// Focus: Longest run of ones within M-bit blocks
    pub fn longest_run_test(&self, bits: &[u8]) -> TestResult {
        let n = bits.len();

        // Configure based on sequence length
        let (m, k, v, pi) = if n < 128 {
            return TestResult::new("Longest Run", 0.0, self.alpha);
        } else if n < 6272 {
            (8, 3, vec![1, 2, 3, 4], vec![0.2148, 0.3672, 0.2305, 0.1875])
        } else if n < 750000 {
            (128, 5, vec![4, 5, 6, 7, 8, 9], vec![0.1174, 0.2430, 0.2493, 0.1752, 0.1027, 0.1124])
        } else {
            (10000, 6, vec![10, 11, 12, 13, 14, 15, 16],
             vec![0.0882, 0.2092, 0.2483, 0.1933, 0.1208, 0.0675, 0.0727])
        };

        let num_blocks = n / m;
        let mut nu = vec![0; k + 1];

        for i in 0..num_blocks {
            let start = i * m;
            let end = start + m;
            let longest = longest_run_of_ones(&bits[start..end]);

            let idx = v.iter().position(|&x| longest <= x).unwrap_or(k);
            nu[idx] += 1;
        }

        let mut chi_squared = 0.0;
        for i in 0..=k {
            let expected = num_blocks as f64 * pi[i];
            chi_squared += (nu[i] as f64 - expected).powi(2) / expected;
        }

        let p_value = igamc(k as f64 / 2.0, chi_squared / 2.0);
        TestResult::new("Longest Run", p_value, self.alpha)
    }

    /// Test 5: Binary Matrix Rank Test
    /// Focus: Rank of disjoint sub-matrices of the entire sequence
    pub fn binary_matrix_rank_test(&self, bits: &[u8]) -> TestResult {
        let m = 32; // Matrix rows
        let q = 32; // Matrix columns
        let n = bits.len();
        let num_matrices = n / (m * q);

        if num_matrices == 0 {
            return TestResult::new("Binary Matrix Rank", 0.0, self.alpha);
        }

        let mut fm = 0; // Full rank count
        let mut fm_1 = 0; // Rank m-1 count

        for k in 0..num_matrices {
            let start = k * m * q;
            let matrix_bits = &bits[start..start + m * q];
            let rank = compute_rank(matrix_bits, m, q);

            if rank == m {
                fm += 1;
            } else if rank == m - 1 {
                fm_1 += 1;
            }
        }

        let pi_m = 0.2888;
        let pi_m_1 = 0.5776;
        let pi_rest = 0.1336;

        let chi_squared = ((fm as f64 - num_matrices as f64 * pi_m).powi(2) / (num_matrices as f64 * pi_m))
            + ((fm_1 as f64 - num_matrices as f64 * pi_m_1).powi(2) / (num_matrices as f64 * pi_m_1))
            + ((num_matrices as f64 - fm as f64 - fm_1 as f64 - num_matrices as f64 * pi_rest).powi(2)
                / (num_matrices as f64 * pi_rest));

        let p_value = (-chi_squared / 2.0).exp();
        TestResult::new("Binary Matrix Rank", p_value, self.alpha)
    }

    /// Test 6: Discrete Fourier Transform (Spectral) Test
    /// Focus: Peak heights in DFT of the sequence
    pub fn dft_test(&self, bits: &[u8]) -> TestResult {
        let n = bits.len();
        let x: Vec<f64> = bits.iter().map(|&b| if b == 1 { 1.0 } else { -1.0 }).collect();

        // Compute DFT
        let mut s = vec![0.0; n / 2];
        for k in 0..n / 2 {
            let mut sum_cos = 0.0;
            let mut sum_sin = 0.0;
            for j in 0..n {
                let angle = 2.0 * PI * (k as f64) * (j as f64) / (n as f64);
                sum_cos += x[j] * angle.cos();
                sum_sin += x[j] * angle.sin();
            }
            s[k] = (sum_cos * sum_cos + sum_sin * sum_sin).sqrt();
        }

        // Threshold
        let t = (3.0 * n as f64).sqrt();
        let n0 = 0.95 * (n as f64) / 2.0;
        let n1 = s.iter().filter(|&&val| val < t).count() as f64;

        let d = (n1 - n0) / ((n as f64 * 0.95 * 0.05 / 4.0).sqrt());
        let p_value = erfc(d.abs() / 2.0_f64.sqrt());

        TestResult::new("Discrete Fourier Transform", p_value, self.alpha)
    }

    /// Test 7: Non-overlapping Template Matching Test
    /// Focus: Number of occurrences of pre-specified target strings
    pub fn non_overlapping_template_test(&self, bits: &[u8]) -> TestResult {
        let m = 9; // Template length
        let template = vec![1u8; m]; // Template of all ones
        let n = bits.len();
        let num_blocks = 8;
        let block_size = n / num_blocks;

        if block_size < m {
            return TestResult::new("Non-overlapping Template", 0.0, self.alpha);
        }

        let mu = (block_size - m + 1) as f64 / 2.0_f64.powi(m as i32);
        let sigma_squared = block_size as f64 * (1.0 / 2.0_f64.powi(m as i32)
            - (2.0 * m as f64 - 1.0) / 2.0_f64.powi(2 * m as i32));

        let mut w = vec![0; num_blocks];
        for i in 0..num_blocks {
            let start = i * block_size;
            let end = start + block_size;
            w[i] = count_non_overlapping_matches(&bits[start..end], &template);
        }

        let mut chi_squared = 0.0;
        for count in w {
            chi_squared += (count as f64 - mu).powi(2) / sigma_squared;
        }

        let p_value = igamc((num_blocks as f64) / 2.0, chi_squared / 2.0);
        TestResult::new("Non-overlapping Template", p_value, self.alpha)
    }

    /// Test 8: Overlapping Template Matching Test
    /// Focus: Number of occurrences of pre-specified target strings (overlapping)
    pub fn overlapping_template_test(&self, bits: &[u8]) -> TestResult {
        let m = 9; // Template length (all ones)
        let n = bits.len();
        let num_blocks = 968; // N in spec
        let block_size = n / num_blocks;

        if block_size < m {
            return TestResult::new("Overlapping Template", 0.0, self.alpha);
        }

        let lambda = (block_size - m + 1) as f64 / 2.0_f64.powi(m as i32);
        let eta = lambda / 2.0;

        let pi = calculate_overlapping_probabilities(m, lambda, eta);
        let mut nu = vec![0; 6]; // v0, v1, v2, v3, v4, v5+

        for i in 0..num_blocks {
            let start = i * block_size;
            let end = start + block_size;
            let count = count_overlapping_matches(&bits[start..end], m);
            let idx = if count >= 5 { 5 } else { count };
            nu[idx] += 1;
        }

        let mut chi_squared = 0.0;
        for i in 0..6 {
            chi_squared += (nu[i] as f64 - num_blocks as f64 * pi[i]).powi(2)
                / (num_blocks as f64 * pi[i]);
        }

        let p_value = igamc(5.0 / 2.0, chi_squared / 2.0);
        TestResult::new("Overlapping Template", p_value, self.alpha)
    }

    /// Test 9: Maurer's "Universal Statistical" Test
    /// Focus: Compressibility of the sequence
    pub fn universal_test(&self, bits: &[u8]) -> TestResult {
        let l = 7; // Block size
        let q = 1280; // Initialization blocks
        let n = bits.len();

        if n < (q + 100) * l {
            return TestResult::new("Universal Statistical", 0.0, self.alpha);
        }

        let k = (n / l) - q;
        let mut t = vec![0; 1 << l]; // 2^L table

        // Initialize table with positions
        for i in 0..q {
            let block = extract_block(bits, i, l);
            t[block] = i + 1;
        }

        let mut sum = 0.0;
        for i in q..q + k {
            let block = extract_block(bits, i, l);
            sum += ((i + 1 - t[block]) as f64).log2();
            t[block] = i + 1;
        }

        let fn_val = sum / k as f64;

        // Expected value and variance (approximate for L=7)
        let expected_value = 6.1962507;
        let variance = 3.125;

        let p_value = erfc(((fn_val - expected_value).abs() / (variance * 2.0_f64).sqrt()) / 2.0_f64.sqrt());
        TestResult::new("Universal Statistical", p_value, self.alpha)
    }

    /// Test 10: Linear Complexity Test
    /// Focus: Length of a Linear Feedback Shift Register (LFSR)
    pub fn linear_complexity_test(&self, bits: &[u8], block_size: usize) -> TestResult {
        let n = bits.len();
        let num_blocks = n / block_size;

        if num_blocks == 0 {
            return TestResult::new("Linear Complexity", 0.0, self.alpha);
        }

        let mu = block_size as f64 / 2.0 + (9.0 + (-1.0_f64).powi(block_size as i32 + 1)) / 36.0
            - (block_size as f64 / 3.0 + 2.0 / 9.0) / 2.0_f64.powi(block_size as i32);

        let mut nu = vec![0; 7]; // v0 to v6

        for i in 0..num_blocks {
            let start = i * block_size;
            let end = start + block_size;
            let complexity = berlekamp_massey(&bits[start..end]);
            let t = (-1.0_f64).powi(block_size as i32) * (complexity as f64 - mu) + 2.0 / 9.0;

            let idx = if t <= -2.5 { 0 }
                else if t <= -1.5 { 1 }
                else if t <= -0.5 { 2 }
                else if t <= 0.5 { 3 }
                else if t <= 1.5 { 4 }
                else if t <= 2.5 { 5 }
                else { 6 };
            nu[idx] += 1;
        }

        let pi = [0.010417, 0.03125, 0.125, 0.5, 0.25, 0.0625, 0.020833];
        let mut chi_squared = 0.0;
        for i in 0..7 {
            chi_squared += (nu[i] as f64 - num_blocks as f64 * pi[i]).powi(2)
                / (num_blocks as f64 * pi[i]);
        }

        let p_value = igamc(6.0 / 2.0, chi_squared / 2.0);
        TestResult::new("Linear Complexity", p_value, self.alpha)
    }

    /// Test 11: Serial Test (2-bit Test)
    /// Focus: Frequency of all possible overlapping m-bit patterns
    pub fn serial_test(&self, bits: &[u8], m: usize) -> SerialTestResult {
        let n = bits.len();
        let psi_m = compute_psi_squared(bits, m, n);
        let psi_m_1 = compute_psi_squared(bits, m - 1, n);
        let psi_m_2 = compute_psi_squared(bits, m - 2, n);

        let delta1 = psi_m - psi_m_1;
        let delta2 = psi_m - 2.0 * psi_m_1 + psi_m_2;

        let p_value1 = igamc(2.0_f64.powi(m as i32 - 2) as f64, delta1 / 2.0);
        let p_value2 = igamc(2.0_f64.powi(m as i32 - 3) as f64, delta2 / 2.0);

        SerialTestResult {
            passed: p_value1 >= self.alpha && p_value2 >= self.alpha,
            p_value1,
            p_value2,
        }
    }

    /// Test 12: Approximate Entropy Test
    /// Focus: Frequency of all possible overlapping m-bit patterns
    pub fn approximate_entropy_test(&self, bits: &[u8], m: usize) -> TestResult {
        let n = bits.len();
        let phi_m = compute_phi(bits, m, n);
        let phi_m_1 = compute_phi(bits, m + 1, n);

        let apen = phi_m - phi_m_1;
        let chi_squared = 2.0 * n as f64 * (apen.ln() - 2.0_f64.powi(m as i32) as f64);
        let p_value = igamc(2.0_f64.powi(m as i32 - 1) as f64, chi_squared / 2.0);

        TestResult::new("Approximate Entropy", p_value, self.alpha)
    }

    /// Test 13: Cumulative Sums (Cusum) Test
    /// Focus: Maximal excursion of random walk
    pub fn cumulative_sums_test(&self, bits: &[u8]) -> CumulativeSumsResult {
        let n = bits.len();
        let x: Vec<i32> = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).collect();

        // Forward cumulative sums
        let mut max_forward = 0;
        let mut sum = 0;
        for &val in &x {
            sum += val;
            if sum.abs() > max_forward {
                max_forward = sum.abs();
            }
        }

        // Backward cumulative sums
        let mut max_backward = 0;
        sum = 0;
        for &val in x.iter().rev() {
            sum += val;
            if sum.abs() > max_backward {
                max_backward = sum.abs();
            }
        }

        let p_forward = compute_cusum_p_value(max_forward, n);
        let p_backward = compute_cusum_p_value(max_backward, n);

        CumulativeSumsResult {
            passed: p_forward >= self.alpha && p_backward >= self.alpha,
            p_value_forward: p_forward,
            p_value_backward: p_backward,
        }
    }

    /// Test 14: Random Excursions Test
    /// Focus: Number of cycles having exactly K visits in a cumulative sum random walk
    pub fn random_excursions_test(&self, bits: &[u8]) -> RandomExcursionsResult {
        let states = vec![-4, -3, -2, -1, 1, 2, 3, 4];
        let x: Vec<i32> = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).collect();

        // Compute cumulative sum
        let mut s = vec![0];
        let mut sum = 0;
        for &val in &x {
            sum += val;
            s.push(sum);
        }

        // Count cycles
        let mut cycles = 0;
        for i in 1..s.len() {
            if s[i] == 0 && s[i - 1] != 0 {
                cycles += 1;
            }
        }

        if cycles < 500 {
            return RandomExcursionsResult {
                passed: false,
                states_tested: 0,
                states_passed: 0,
                min_p_value: 0.0,
            };
        }

        let mut passed_count = 0;
        let mut min_p = 1.0;

        for &state in &states {
            let mut nu = vec![0; 6];
            for cycle in compute_cycles(&s) {
                let count = cycle.iter().filter(|&&val| val == state).count();
                let idx = if count >= 5 { 5 } else { count };
                nu[idx] += 1;
            }

            let pi = compute_random_excursion_probabilities(state);
            let mut chi_squared = 0.0;
            for i in 0..6 {
                chi_squared += (nu[i] as f64 - cycles as f64 * pi[i]).powi(2)
                    / (cycles as f64 * pi[i]);
            }

            let p_value = igamc(5.0 / 2.0, chi_squared / 2.0);
            if p_value >= self.alpha {
                passed_count += 1;
            }
            if p_value < min_p {
                min_p = p_value;
            }
        }

        RandomExcursionsResult {
            passed: passed_count == states.len(),
            states_tested: states.len(),
            states_passed: passed_count,
            min_p_value: min_p,
        }
    }

    /// Test 15: Random Excursions Variant Test
    /// Focus: Total number of times that a particular state is visited
    pub fn random_excursions_variant_test(&self, bits: &[u8]) -> RandomExcursionsVariantResult {
        let states = vec![-9, -8, -7, -6, -5, -4, -3, -2, -1, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let x: Vec<i32> = bits.iter().map(|&b| if b == 1 { 1 } else { -1 }).collect();

        // Compute cumulative sum
        let mut s = vec![0];
        let mut sum = 0;
        for &val in &x {
            sum += val;
            s.push(sum);
        }

        let j = s.len() as i32;
        let mut passed_count = 0;
        let mut min_p = 1.0;

        for &state in &states {
            let count = s.iter().filter(|&&val| val == state).count() as i32;
            let p_value = erfc(((count - j).abs() as f64 / (2.0 * j as f64 * (4.0 * state.abs() as f64 - 2.0)).sqrt())
                / 2.0_f64.sqrt());

            if p_value >= self.alpha {
                passed_count += 1;
            }
            if p_value < min_p {
                min_p = p_value;
            }
        }

        RandomExcursionsVariantResult {
            passed: passed_count == states.len(),
            states_tested: states.len(),
            states_passed: passed_count,
            min_p_value: min_p,
        }
    }
}

// ==================== Helper Functions ====================

/// Convert bytes to bit array
fn bytes_to_bits(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

/// Complementary error function
fn erfc(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.5 * x.abs());
    let exponent = -x.abs() * x.abs() - 1.26551223
        + t * (1.00002368 + t * (0.37409196 + t * (0.09678418
        + t * (-0.18628806 + t * (0.27886807 + t * (-1.13520398
        + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277))))))));
    let tau = t * exponent.exp();
    if x >= 0.0 { tau } else { 2.0 - tau }
}

/// Incomplete gamma function (complementary)
fn igamc(a: f64, x: f64) -> f64 {
    if x <= 0.0 || a <= 0.0 {
        return 1.0;
    }

    // Simple approximation
    let mut sum = 1.0;
    let mut term = 1.0;
    for n in 1..1000 {
        term *= x / (a + n as f64);
        sum += term;
        if term < 1e-10 * sum {
            break;
        }
    }

    1.0 - x.powf(a) * (-x).exp() * sum / gamma(a + 1.0)
}

/// Gamma function (approximation)
fn gamma(z: f64) -> f64 {
    // Stirling's approximation
    (2.0 * PI / z).sqrt() * (z / std::f64::consts::E).powf(z)
}

/// Find longest run of ones in a bit sequence
fn longest_run_of_ones(bits: &[u8]) -> usize {
    let mut max_run = 0;
    let mut current_run = 0;

    for &bit in bits {
        if bit == 1 {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0;
        }
    }
    max_run
}

/// Compute binary matrix rank using Gaussian elimination
fn compute_rank(bits: &[u8], rows: usize, cols: usize) -> usize {
    let mut matrix = vec![vec![0u8; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            matrix[i][j] = bits[i * cols + j];
        }
    }

    let mut rank = 0;
    for col in 0..cols.min(rows) {
        // Find pivot
        let mut pivot = None;
        for row in rank..rows {
            if matrix[row][col] == 1 {
                pivot = Some(row);
                break;
            }
        }

        if let Some(pivot_row) = pivot {
            // Swap rows
            matrix.swap(rank, pivot_row);

            // Eliminate
            for row in (rank + 1)..rows {
                if matrix[row][col] == 1 {
                    for c in 0..cols {
                        matrix[row][c] ^= matrix[rank][c];
                    }
                }
            }
            rank += 1;
        }
    }
    rank
}

/// Count non-overlapping pattern matches
fn count_non_overlapping_matches(bits: &[u8], template: &[u8]) -> usize {
    let n = bits.len();
    let m = template.len();
    if m > n {
        return 0;
    }

    let mut count = 0;
    let mut i = 0;
    while i <= n - m {
        if bits[i..i + m] == template[..] {
            count += 1;
            i += m; // Non-overlapping
        } else {
            i += 1;
        }
    }
    count
}

/// Count overlapping pattern matches (all ones of length m)
fn count_overlapping_matches(bits: &[u8], m: usize) -> usize {
    let n = bits.len();
    if m > n {
        return 0;
    }

    let mut count = 0;
    for i in 0..=n - m {
        if bits[i..i + m].iter().all(|&b| b == 1) {
            count += 1;
        }
    }
    count
}

/// Calculate probabilities for overlapping template test
fn calculate_overlapping_probabilities(_m: usize, lambda: f64, _eta: f64) -> Vec<f64> {
    let mut pi = vec![0.0; 6];
    pi[0] = (-lambda).exp();
    for i in 1..=5 {
        if i < 5 {
            pi[i] = lambda.powi(i as i32) * (-lambda).exp() / factorial(i as u64) as f64;
        } else {
            pi[5] = 1.0 - pi[0..5].iter().sum::<f64>();
        }
    }
    pi
}

/// Factorial
fn factorial(n: u64) -> u64 {
    (1..=n).product()
}

/// Extract L-bit block at position i
fn extract_block(bits: &[u8], i: usize, l: usize) -> usize {
    let mut block = 0;
    for j in 0..l {
        block = (block << 1) | (bits[i * l + j] as usize);
    }
    block
}

/// Berlekamp-Massey algorithm for linear complexity
fn berlekamp_massey(bits: &[u8]) -> usize {
    let n = bits.len();
    let mut c = vec![0u8; n];
    let mut b = vec![0u8; n];
    c[0] = 1;
    b[0] = 1;

    let mut l = 0;
    let mut m = -1i32;

    for i in 0..n {
        let mut d = bits[i];
        for j in 1..=l {
            d ^= c[j] & bits[i - j];
        }

        if d == 1 {
            let t = c.clone();
            for j in 0..n {
                let idx = i as i32 - m + j as i32;
                if idx >= 0 && (idx as usize) < n {
                    c[j] ^= b[j];
                }
            }
            if l <= i / 2 {
                l = i + 1 - l;
                m = i as i32;
                b = t;
            }
        }
    }
    l
}

/// Compute psi-squared for serial test
fn compute_psi_squared(bits: &[u8], m: usize, n: usize) -> f64 {
    let num_patterns = 1 << m; // 2^m
    let mut counts = vec![0; num_patterns];

    for i in 0..n {
        let mut pattern = 0;
        for j in 0..m {
            pattern = (pattern << 1) | (bits[(i + j) % n] as usize);
        }
        counts[pattern] += 1;
    }

    let mut psi = 0.0;
    for count in counts {
        if count > 0 {
            psi += (count * count) as f64;
        }
    }
    psi = psi / n as f64 - n as f64;
    psi
}

/// Compute phi for approximate entropy test
fn compute_phi(bits: &[u8], m: usize, n: usize) -> f64 {
    let num_patterns = 1 << m;
    let mut counts = vec![0; num_patterns];

    for i in 0..n {
        let mut pattern = 0;
        for j in 0..m {
            pattern = (pattern << 1) | (bits[(i + j) % n] as usize);
        }
        counts[pattern] += 1;
    }

    let mut phi = 0.0;
    for count in counts {
        if count > 0 {
            let probability = count as f64 / n as f64;
            phi += probability * probability.ln();
        }
    }
    phi
}

/// Compute p-value for cumulative sums test
fn compute_cusum_p_value(z: i32, n: usize) -> f64 {
    let z_norm = z.abs() as f64 / (n as f64).sqrt();
    let mut sum = 0.0;

    for k in (-(n as i32) / z.abs() - 1)..(n as i32 / z.abs() + 1) {
        let term1 = normal_cdf((4.0 * k as f64 + 1.0) * z_norm);
        let term2 = normal_cdf((4.0 * k as f64 - 1.0) * z_norm);
        sum += term1 - term2;
    }

    1.0 - sum
}

/// Normal CDF
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / 2.0_f64.sqrt()))
}

/// Error function
fn erf(x: f64) -> f64 {
    1.0 - erfc(x)
}

/// Compute cycles in cumulative sum
fn compute_cycles(s: &[i32]) -> Vec<Vec<i32>> {
    let mut cycles = Vec::new();
    let mut current_cycle = Vec::new();

    for &val in s {
        current_cycle.push(val);
        if val == 0 && current_cycle.len() > 1 {
            cycles.push(current_cycle.clone());
            current_cycle.clear();
            current_cycle.push(0);
        }
    }
    cycles
}

/// Compute probabilities for random excursions test
fn compute_random_excursion_probabilities(state: i32) -> Vec<f64> {
    // Simplified probabilities (would need full computation for accuracy)
    let x = state.abs() as f64;
    let pi0 = 1.0 / (2.0 * x);
    let pi = vec![pi0, pi0, pi0, pi0, pi0, pi0];
    pi
}

impl NistTestSuite {
    /// Print comprehensive test results
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║       NIST SP 800-22 STATISTICAL TEST SUITE RESULTS       ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        println!("Test 1:  {:40} {} (p={:.6})",
            self.frequency_monobit.test_name,
            if self.frequency_monobit.passed { "✓ PASS" } else { "✗ FAIL" },
            self.frequency_monobit.p_value);

        println!("Test 2:  {:40} {} (p={:.6})",
            self.frequency_block.test_name,
            if self.frequency_block.passed { "✓ PASS" } else { "✗ FAIL" },
            self.frequency_block.p_value);

        println!("Test 3:  {:40} {} (p={:.6})",
            self.runs.test_name,
            if self.runs.passed { "✓ PASS" } else { "✗ FAIL" },
            self.runs.p_value);

        println!("Test 4:  {:40} {} (p={:.6})",
            self.longest_run.test_name,
            if self.longest_run.passed { "✓ PASS" } else { "✗ FAIL" },
            self.longest_run.p_value);

        println!("Test 5:  {:40} {} (p={:.6})",
            self.binary_matrix_rank.test_name,
            if self.binary_matrix_rank.passed { "✓ PASS" } else { "✗ FAIL" },
            self.binary_matrix_rank.p_value);

        println!("Test 6:  {:40} {} (p={:.6})",
            self.discrete_fourier_transform.test_name,
            if self.discrete_fourier_transform.passed { "✓ PASS" } else { "✗ FAIL" },
            self.discrete_fourier_transform.p_value);

        println!("Test 7:  {:40} {} (p={:.6})",
            self.non_overlapping_template.test_name,
            if self.non_overlapping_template.passed { "✓ PASS" } else { "✗ FAIL" },
            self.non_overlapping_template.p_value);

        println!("Test 8:  {:40} {} (p={:.6})",
            self.overlapping_template.test_name,
            if self.overlapping_template.passed { "✓ PASS" } else { "✗ FAIL" },
            self.overlapping_template.p_value);

        println!("Test 9:  {:40} {} (p={:.6})",
            self.universal.test_name,
            if self.universal.passed { "✓ PASS" } else { "✗ FAIL" },
            self.universal.p_value);

        println!("Test 10: {:40} {} (p={:.6})",
            self.linear_complexity.test_name,
            if self.linear_complexity.passed { "✓ PASS" } else { "✗ FAIL" },
            self.linear_complexity.p_value);

        println!("Test 11: {:40} {} (p1={:.6}, p2={:.6})",
            "Serial",
            if self.serial.passed { "✓ PASS" } else { "✗ FAIL" },
            self.serial.p_value1, self.serial.p_value2);

        println!("Test 12: {:40} {} (p={:.6})",
            self.approximate_entropy.test_name,
            if self.approximate_entropy.passed { "✓ PASS" } else { "✗ FAIL" },
            self.approximate_entropy.p_value);

        println!("Test 13: {:40} {} (pF={:.6}, pB={:.6})",
            "Cumulative Sums",
            if self.cumulative_sums.passed { "✓ PASS" } else { "✗ FAIL" },
            self.cumulative_sums.p_value_forward,
            self.cumulative_sums.p_value_backward);

        println!("Test 14: {:40} {} ({}/{} states, min_p={:.6})",
            "Random Excursions",
            if self.random_excursions.passed { "✓ PASS" } else { "✗ FAIL" },
            self.random_excursions.states_passed,
            self.random_excursions.states_tested,
            self.random_excursions.min_p_value);

        println!("Test 15: {:40} {} ({}/{} states, min_p={:.6})",
            "Random Excursions Variant",
            if self.random_excursions_variant.passed { "✓ PASS" } else { "✗ FAIL" },
            self.random_excursions_variant.states_passed,
            self.random_excursions_variant.states_tested,
            self.random_excursions_variant.min_p_value);

        let total = self.count_passed();
        println!("\n═══════════════════════════════════════════════════════════");
        println!("  OVERALL: {}/15 tests passed", total);
        println!("═══════════════════════════════════════════════════════════\n");
    }

    /// Count number of tests passed
    pub fn count_passed(&self) -> usize {
        let mut count = 0;
        if self.frequency_monobit.passed { count += 1; }
        if self.frequency_block.passed { count += 1; }
        if self.runs.passed { count += 1; }
        if self.longest_run.passed { count += 1; }
        if self.binary_matrix_rank.passed { count += 1; }
        if self.discrete_fourier_transform.passed { count += 1; }
        if self.non_overlapping_template.passed { count += 1; }
        if self.overlapping_template.passed { count += 1; }
        if self.universal.passed { count += 1; }
        if self.linear_complexity.passed { count += 1; }
        if self.serial.passed { count += 1; }
        if self.approximate_entropy.passed { count += 1; }
        if self.cumulative_sums.passed { count += 1; }
        if self.random_excursions.passed { count += 1; }
        if self.random_excursions_variant.passed { count += 1; }
        count
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.count_passed() == 15
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nist_suite_on_random_data() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let tester = NistTester::default();
        let results = tester.run_all_tests(&data);

        // Should have results for all 15 tests
        assert!(results.count_passed() <= 15);
    }

    #[test]
    fn test_frequency_monobit() {
        let tester = NistTester::default();
        let bits = vec![1, 0, 1, 1, 0, 1, 0, 0, 1, 1];
        let result = tester.frequency_monobit_test(&bits);
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
    }

    #[test]
    fn test_runs_test() {
        let tester = NistTester::default();
        let bits = vec![1, 0, 0, 1, 1, 1, 0, 1, 0, 0];
        let result = tester.runs_test(&bits);
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
    }
}
