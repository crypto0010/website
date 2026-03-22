# DCL-RS Cost/Performance/Security Improvements — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve DCL-RS across three axes — security (fix frequency test, NIST-on-DCL pipeline, coprimality repair, modular PowerMap testing), performance (binary exponentiation, zero-clone BFS, rayon parallelism), and cost (in-place evolve, --modulus flags).

**Architecture:** Eight targeted changes across four crates (`dcl-core`, `dcl-complexity`, `dcl-security`, `dcl-cli`). Each change extends existing structs/traits — no new crates. Reuses `Transform`, `Labeling`, `CryptanalysisConfig`, `PruningConfig` as extension points.

**Tech Stack:** Rust 2021, rayon (already in workspace), sha2 (already in workspace), clap (already in workspace)

**Build command (excludes fuzz):**
```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```

**Test commands per crate:**
```bash
cargo test -p dcl-core
cargo test -p dcl-security
cargo test -p dcl-complexity
cargo test -p dcl-cli
```

---

## Task 1: Binary Exponentiation in Unbounded PowerMap

**Files:**
- Modify: `dcl-core/src/transform.rs:64-72` (replace unbounded apply body)

**Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `dcl-core/src/transform.rs` (after line 205):

```rust
    #[test]
    fn power_map_large_exponent() {
        // With O(m) loop, m=60 takes 60 iterations.
        // With binary exp, takes ~6 iterations.
        // Both should produce u64::MAX (saturation).
        let pm = PowerMap::new(60);
        assert_eq!(pm.apply(3), u64::MAX); // 3^60 overflows u64
        assert_eq!(pm.apply(1), 1);        // 1^anything = 1
        assert_eq!(pm.apply(0), 0);        // 0^anything = 0 (but labels are >0)
    }

    #[test]
    fn power_map_binary_exp_correctness() {
        // Verify exact results for cases that don't overflow
        let pm = PowerMap::new(10);
        assert_eq!(pm.apply(2), 1024);     // 2^10 = 1024
        assert_eq!(pm.apply(3), 59049);    // 3^10 = 59049

        let pm3 = PowerMap::new(3);
        assert_eq!(pm3.apply(5), 125);     // 5^3 = 125
        assert_eq!(pm3.apply(10), 1000);   // 10^3 = 1000
    }
```

**Step 2: Run test to verify it passes with current code (baseline)**

```bash
cargo test -p dcl-core power_map_large_exponent -- --nocapture
cargo test -p dcl-core power_map_binary_exp_correctness -- --nocapture
```
Expected: PASS (current O(m) code produces same results, just slower).

**Step 3: Replace unbounded PowerMap with binary exponentiation**

Replace lines 64-72 in `dcl-core/src/transform.rs` (the `else` branch of `apply`):

Old code:
```rust
        } else {
            // Unbounded: use u128 saturation
            let mut result: u128 = 1;
            let base = x as u128;
            for _ in 0..self.m {
                result = result.saturating_mul(base);
            }
            result.min(u64::MAX as u128) as u64
        }
```

New code:
```rust
        } else {
            // Unbounded: binary exponentiation with early saturation exit
            if x <= 1 {
                return x;
            }
            let limit = u64::MAX as u128;
            let mut result: u128 = 1;
            let mut base: u128 = x as u128;
            let mut exp = self.m;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = result.saturating_mul(base);
                    if result >= limit {
                        return u64::MAX;
                    }
                }
                exp >>= 1;
                if exp > 0 {
                    base = base.saturating_mul(base);
                    if base >= limit {
                        // All further multiplications will saturate
                        // If remaining exp has any bits set, result will overflow
                        if exp > 0 {
                            return u64::MAX;
                        }
                    }
                }
            }
            result.min(limit) as u64
        }
```

**Step 4: Run all transform tests to verify correctness**

```bash
cargo test -p dcl-core transform -- --nocapture
```
Expected: All 8 transform tests PASS (including existing `power_map_apply`, `power_map_coprime_preserving`, and the 2 new tests).

**Step 5: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

---

## Task 2: In-Place BFS Backtracking (Zero-Clone Search)

**Files:**
- Modify: `dcl-complexity/src/bfs_optimized.rs:139-203` (replace `get_candidates` and `search` methods)

**Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `dcl-complexity/src/bfs_optimized.rs` (after line 278):

```rust
    #[test]
    fn zero_clone_finds_same_solutions() {
        let g = Graph::path(4);
        let config = PruningConfig {
            prefer_primes: false,
            use_symmetry: true,
            max_sum: None,
            max_solutions: 5,
            incremental_check: true,
        };
        let mut searcher = OptimizedBfs::new(&g, 15, config);
        let result = searcher.search();

        // Must find solutions
        assert!(!result.solutions.is_empty(), "Should find solutions for Path-4 with max_label=15");
        // Every solution must be a valid coprime labeling
        for sol in &result.solutions {
            assert!(g.is_coprime_labeling(sol), "Solution {:?} is not coprime", sol.labels);
        }
        // States explored should be reasonable (not blowup from cloning bugs)
        assert!(result.states_explored < 10000, "Too many states explored: {}", result.states_explored);
    }
```

**Step 2: Run test to verify it passes with current code (baseline)**

```bash
cargo test -p dcl-complexity zero_clone -- --nocapture
```
Expected: PASS (current code finds same solutions, just with cloning).

**Step 3: Rewrite get_candidates to return slice**

Replace `get_candidates` method (lines 139-146) with:

```rust
    /// Get candidate labels for position (primes if enabled).
    fn get_candidates(&self) -> &[u64] {
        if self.config.prefer_primes && !self.prime_cache.is_empty() {
            &self.prime_cache
        } else {
            &self.all_labels
        }
    }
```

Add `all_labels: Vec<u64>` field to `OptimizedBfs` struct (line 49, after `prime_cache`):

```rust
    all_labels: Vec<u64>,
```

Initialize it in `new()` — insert after line 64 (before closing brace):

```rust
        let all_labels: Vec<u64> = (1..=max_label).collect();
```

And add the field to the struct construction:

```rust
        OptimizedBfs {
            graph,
            config,
            max_label,
            prime_cache,
            all_labels,
        }
```

**Step 4: Rewrite search method with in-place backtracking**

Replace the `search` method (lines 148-203) with:

```rust
    /// Backtracking search with pruning — zero-clone in-place algorithm.
    pub fn search(&mut self) -> BfsResult {
        let n = self.graph.n;
        let mut solutions = Vec::new();
        let mut states_explored: usize = 0;
        let mut states_pruned: usize = 0;
        let mut max_label_found: u64 = 0;

        let candidates = self.get_candidates().to_vec();
        let mut partial = vec![0u64; n];
        // Stack stores (position, index_into_candidates)
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];

        while let Some(frame) = stack.last_mut() {
            let (pos, cand_idx) = *frame;

            if *cand_idx >= candidates.len() {
                // Exhausted candidates at this position — backtrack
                stack.pop();
                continue;
            }

            let label = candidates[*cand_idx];
            *&mut frame.1 += 1; // advance to next candidate for when we return

            partial[pos] = label;
            states_explored += 1;

            // Symmetry check
            if self.config.use_symmetry && pos > 0 && label < partial[pos - 1] {
                states_pruned += 1;
                continue;
            }

            // Incremental coprimality check
            if !self.is_promising(&partial, pos) {
                states_pruned += 1;
                continue;
            }

            if pos + 1 == n {
                // Complete labeling — validate and record
                let labeling = Labeling::new(partial.clone());
                if self.graph.is_coprime_labeling(&labeling) {
                    let max_in_solution = *partial.iter().max().unwrap_or(&0);
                    max_label_found = max_label_found.max(max_in_solution);
                    solutions.push(labeling);

                    if solutions.len() >= self.config.max_solutions {
                        break;
                    }
                }
            } else {
                // Descend to next position
                stack.push((pos + 1, 0));
            }
        }

        BfsResult {
            solutions,
            states_explored,
            states_pruned,
            max_label_found,
        }
    }
```

**Step 5: Run all BFS tests to verify correctness**

```bash
cargo test -p dcl-complexity -- --nocapture
```
Expected: All 4 tests PASS (`optimized_bfs_path3`, `prune_efficiency`, `sieve_primes_correctness`, `zero_clone_finds_same_solutions`).

**Step 6: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

---

## Task 3: Rayon Parallelism in Scaling Benchmark

**Files:**
- Modify: `dcl-security/Cargo.toml` (add rayon dependency)
- Modify: `dcl-security/src/scaling.rs:1-72` (parallelize with rayon)

**Step 1: Add rayon dependency**

In `dcl-security/Cargo.toml`, add under `[dependencies]`:
```toml
rayon = { workspace = true }
```

**Step 2: Rewrite run_scaling with par_iter**

Replace the entire `run_scaling` function (lines 23-72) in `dcl-security/src/scaling.rs`:

```rust
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
```

**Step 3: Update the test to match new signature**

Replace the test in `dcl-security/src/scaling.rs` (lines 95-103):

```rust
    #[test]
    fn scaling_path_small() {
        let results = run_scaling("path", &[4, 6], 20, 2, None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].n, 4);
        assert_eq!(results[1].n, 6);
        // Search space should grow with n
        assert!(results[1].search_space > results[0].search_space);
    }
```

**Step 4: Update CLI caller**

In `dcl-cli/src/cmd_scaling.rs`, update line 43:

Old:
```rust
    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m);
```

New:
```rust
    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m, None);
```

(The `None` is a placeholder — Task 8 will add the `--modulus` flag.)

**Step 5: Run tests**

```bash
cargo test -p dcl-security scaling -- --nocapture
```
Expected: PASS.

**Step 6: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

---

## Task 4: Fix Frequency Test — Adaptive Bins and Correct Threshold

**Files:**
- Modify: `dcl-security/src/cryptanalysis.rs:390-408` (rewrite `frequency_test`)

**Step 1: Write the failing test**

Append inside the existing `#[cfg(test)] mod tests` block in `dcl-security/src/cryptanalysis.rs`:

```rust
    #[test]
    fn frequency_test_validates_randomness() {
        let g = Graph::path(4);
        let analyzer = Cryptanalyzer::new(&g, 2);

        // Uniform-ish data should pass
        let uniform: Vec<u64> = (0..200).map(|i| i % 32).collect();
        assert!(analyzer.frequency_test(&uniform), "Uniform data should pass frequency test");

        // Heavily biased data should fail
        let mut biased = vec![1u64; 180];
        biased.extend(vec![2u64; 20]);
        assert!(!analyzer.frequency_test(&biased), "Biased data should fail frequency test");
    }
```

**Step 2: Run test to see current behavior**

```bash
cargo test -p dcl-security frequency_test_validates -- --nocapture
```
Expected: May FAIL on the biased assertion (current threshold 500.0 is too lenient).

**Step 3: Rewrite frequency_test with adaptive bins**

Replace lines 390-408 in `dcl-security/src/cryptanalysis.rs`:

```rust
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
```

Add a helper function before the `impl Cryptanalyzer` block (e.g., after line 99):

```rust
/// Chi-squared critical values at alpha=0.05 for common degrees of freedom.
fn chi_squared_critical(df: usize) -> f64 {
    // Lookup table for alpha=0.05
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
        // Approximation for other df: Wilson-Hilferty
        _ => {
            let z = 1.645; // z for alpha=0.05
            let term = 1.0 - 2.0 / (9.0 * df as f64) + z * (2.0 / (9.0 * df as f64)).sqrt();
            df as f64 * term.powi(3)
        }
    }
}
```

**Step 4: Run the new test and all cryptanalysis tests**

```bash
cargo test -p dcl-security frequency_test_validates -- --nocapture
cargo test -p dcl-security cryptanalysis -- --nocapture
```
Expected: All PASS. Frequency test now correctly rejects biased data.

**Step 5: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

---

## Task 5: NIST Tests on DCL Label Evolution Stream

**Files:**
- Modify: `dcl-security/src/cryptanalysis.rs` (add `dcl_labels_to_test_data` public function)
- Modify: `dcl-cli/src/cmd_nist_tests.rs` (add `--dcl-mode` flag and DCL pipeline)

**Step 1: Add dcl_labels_to_test_data function**

Add after the `chi_squared_critical` function (added in Task 4), before `impl<'a> Cryptanalyzer`:

```rust
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
```

Note: This depends on `evolve_in_place` from Task 8. If implementing in order, Task 8's `evolve_in_place` must be done first, or use `current = current.evolve(transform)` as a temporary fallback:

```rust
        current = current.evolve(transform);
```

**Step 2: Write the test**

Append inside the `#[cfg(test)] mod tests` block in `dcl-security/src/cryptanalysis.rs`:

```rust
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
```

**Step 3: Run the test**

```bash
cargo test -p dcl-security dcl_labels_to_test -- --nocapture
```
Expected: PASS.

**Step 4: Add --dcl-mode flag to NIST CLI**

In `dcl-cli/src/cmd_nist_tests.rs`, add new fields to `NistTestsArgs` (after line 29):

```rust
    /// Run NIST tests on DCL label evolution (instead of primes)
    #[arg(long)]
    dcl_mode: bool,

    /// Graph type for DCL mode
    #[arg(long, default_value = "path")]
    graph: String,

    /// Number of vertices for DCL mode
    #[arg(short, long, default_value = "10")]
    n: usize,

    /// Number of evolution steps for DCL mode
    #[arg(long, default_value = "200")]
    steps: usize,

    /// Power map exponent for DCL mode
    #[arg(short, long, default_value = "2")]
    m: u32,
```

**Step 5: Add DCL mode branch to run function**

In `dcl-cli/src/cmd_nist_tests.rs`, add this at the start of the `run` function, after the header println (after line 43), replacing the prime generation block (lines 45-76):

```rust
    let test_data = if args.dcl_mode {
        println!("Mode: DCL Label Evolution");
        println!("  Graph: {}-{}", args.graph, args.n);
        println!("  Steps: {}", args.steps);
        println!("  Power Map: x^{}", args.m);
        println!();

        use dcl_core::graph::Graph;
        use dcl_core::labeling::Labeling;
        use dcl_core::transform::PowerMap;

        let graph = match args.graph.as_str() {
            "path" => Graph::path(args.n),
            "cycle" => Graph::cycle(args.n),
            "complete" => Graph::complete(args.n),
            _ => Graph::path(args.n),
        };

        // Generate initial coprime labeling using small primes
        let primes = vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
        let labels: Vec<u64> = (0..args.n).map(|i| primes[i % primes.len()]).collect();
        let initial = Labeling::new(labels);

        let pm = PowerMap::new(args.m);
        println!("Step 1/3: Generating DCL evolution data ({} steps)...", args.steps);
        let data = dcl_security::cryptanalysis::dcl_labels_to_test_data(&pm, &initial, args.steps);
        println!("  Generated {} bytes ({} bits) of DCL evolution data\n", data.len(), data.len() * 8);
        data
    } else {
        // Existing prime-based pipeline
        println!("Step 1/3: Generating {} safe primes...", args.count);
        let sieve = ImprovedSieve::default();

        let start = std::time::Instant::now();
        let primes = if args.parallel {
            sieve.generate_batch_parallel(args.count, args.mr_rounds)
        } else {
            sieve.generate_batch(args.count, args.mr_rounds)
        };
        let generation_time = start.elapsed();

        if primes.primes.is_empty() {
            eprintln!("Failed to generate primes!");
            return;
        }

        println!("  Generated {} primes in {:.2?}", primes.primes.len(), generation_time);
        println!("  Success Rate: {:.1}%\n", primes.success_rate() * 100.0);

        let mut test_data = Vec::new();
        for prime in &primes.primes {
            let mut hasher = Sha256::new();
            hasher.update(prime.to_le_bytes());
            let hash = hasher.finalize();
            test_data.extend_from_slice(&hash);
        }
        test_data
    };

    println!("Step 2/3: {} bytes of test data ready\n", test_data.len());
```

Then replace lines 80-86 (the existing Step 3) to use `test_data`:

```rust
    println!("Step 3/3: Running NIST SP 800-22 test suite...\n");
    let tester = NistTester::new(args.alpha);
    let start = std::time::Instant::now();
    let results = tester.run_all_tests(&test_data);
    let test_time = start.elapsed();
```

**Step 6: Build and smoke test**

```bash
cargo build -p dcl-cli
cargo run -p dcl-cli -- nist-tests --dcl-mode --graph path --n 10 --steps 200 --m 2
```
Expected: NIST suite runs on DCL evolution data, prints results.

---

## Task 6: HashTransform Coprimality Repair

**Files:**
- Modify: `dcl-core/src/labeling.rs` (add `repair_coprimality` method, needs `use crate::graph::Graph;`)

**Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `dcl-core/src/labeling.rs` (after line 140):

```rust
    #[test]
    fn repair_coprimality_fixes_violations() {
        use crate::graph::Graph;

        let g = Graph::path(4);
        // Labels [6, 9, 5, 7]: gcd(6,9)=3, edge (0,1) violates coprimality
        let mut labeling = Labeling::new(vec![6, 9, 5, 7]);
        assert!(!g.is_coprime_labeling(&labeling));

        let repairs = labeling.repair_coprimality(&g);
        assert!(repairs > 0, "Should have made repairs");
        assert!(g.is_coprime_labeling(&labeling), "Should be coprime after repair");
        // All labels must remain positive
        assert!(labeling.labels.iter().all(|&l| l >= 1));
    }

    #[test]
    fn repair_coprimality_no_op_on_valid() {
        use crate::graph::Graph;

        let g = Graph::path(3);
        let mut labeling = Labeling::new(vec![2, 3, 5]);
        assert!(g.is_coprime_labeling(&labeling));

        let repairs = labeling.repair_coprimality(&g);
        assert_eq!(repairs, 0, "No repairs needed for valid labeling");
        assert_eq!(labeling.labels, vec![2, 3, 5], "Labels should be unchanged");
    }
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p dcl-core repair_coprimality -- --nocapture
```
Expected: FAIL with "no method named `repair_coprimality`".

**Step 3: Implement repair_coprimality**

Add `use crate::graph::Graph;` at the top of `dcl-core/src/labeling.rs` (after line 6):

```rust
use crate::graph::Graph;
```

Add the method inside `impl Labeling` (after the `max_label` method, before the closing brace at line 96):

```rust
    /// Repair coprimality violations by incrementing labels on edges where gcd != 1.
    /// Returns the number of repairs performed.
    /// After repair, the labeling satisfies coprimality for all edges in `graph`.
    pub fn repair_coprimality(&mut self, graph: &Graph) -> usize {
        let mut repairs = 0;
        loop {
            let mut found_violation = false;
            for (u, v) in graph.edges() {
                while !are_coprime(self.labels[u], self.labels[v]) {
                    // Increment the larger label
                    if self.labels[u] >= self.labels[v] {
                        self.labels[u] += 1;
                    } else {
                        self.labels[v] += 1;
                    }
                    repairs += 1;
                    found_violation = true;
                }
            }
            if !found_violation {
                break;
            }
        }
        repairs
    }
```

**Step 4: Run tests**

```bash
cargo test -p dcl-core repair_coprimality -- --nocapture
cargo test -p dcl-core -- --nocapture
```
Expected: All tests PASS including the 2 new repair tests.

**Step 5: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

---

## Task 7: Modular PowerMap in Cryptanalysis

**Files:**
- Modify: `dcl-security/src/cryptanalysis.rs` (add `modulus` to `CryptanalysisConfig`, update PowerMap construction)
- Modify: `dcl-cli/src/cmd_cryptanalysis.rs` (add `--modulus` flag)

**Step 1: Add modulus field to CryptanalysisConfig**

In `dcl-security/src/cryptanalysis.rs`, add to `CryptanalysisConfig` struct (after line 87):

```rust
    /// Optional prime modulus for bounded label evolution
    pub modulus: Option<u64>,
```

Update the `Default` impl (line 96, before closing brace):

```rust
            brute_force_samples: 50,
            modulus: None,
```

**Step 2: Add a helper method to Cryptanalyzer for PowerMap construction**

Add inside `impl<'a> Cryptanalyzer` (after `with_config` method, line 127):

```rust
    /// Build the PowerMap based on configuration (unbounded or modular).
    fn make_power_map(&self) -> PowerMap {
        match self.config.as_ref().and_then(|c| c.modulus) {
            Some(n) => PowerMap::with_modulus(self.power_map_exp, n),
            None => PowerMap::new(self.power_map_exp),
        }
    }
```

**Step 3: Replace all PowerMap::new calls with make_power_map**

In `dcl-security/src/cryptanalysis.rs`, replace these 4 lines:

Line 218: `let power_map = PowerMap::new(self.power_map_exp);`
Replace with: `let power_map = self.make_power_map();`

Line 260: `let power_map = PowerMap::new(self.power_map_exp);`
Replace with: `let power_map = self.make_power_map();`

Line 302: `let power_map = PowerMap::new(self.power_map_exp);`
Replace with: `let power_map = self.make_power_map();`

Line 363: `let power_map = PowerMap::new(self.power_map_exp);`
Replace with: `let power_map = self.make_power_map();`

**Step 4: Fix all CryptanalysisConfig construction sites**

Every place that constructs `CryptanalysisConfig` needs the new `modulus` field.

In `dcl-security/src/scaling.rs`, the config construction (line 43-48) — add `modulus` field:

```rust
        let config = CryptanalysisConfig {
            max_label,
            power_map_exp: power_exp,
            test_iterations: 20,
            brute_force_samples: 30,
            modulus,
        };
```

(This already has the `modulus` parameter from Task 3.)

In `dcl-cli/src/cmd_cryptanalysis.rs`, the config construction (lines 58-63) — add `modulus` field:

```rust
    let config = dcl_security::cryptanalysis::CryptanalysisConfig {
        max_label: args.max_label,
        power_map_exp: args.m,
        test_iterations: if args.quick { 20 } else { 100 },
        brute_force_samples: args.samples,
        modulus: args.modulus,
    };
```

**Step 5: Add --modulus flag to CLI**

In `dcl-cli/src/cmd_cryptanalysis.rs`, add to `CryptanalysisArgs` (after line 32):

```rust
    /// Optional prime modulus for bounded label evolution
    #[arg(long)]
    modulus: Option<u64>,
```

**Step 6: Write the test**

Append inside `#[cfg(test)] mod tests` in `dcl-security/src/cryptanalysis.rs`:

```rust
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
        // Should complete without panic and produce a valid report
        assert!(report.brute_force_analysis.total_attempts <= 10);
    }
```

**Step 7: Run tests and build**

```bash
cargo test -p dcl-security cryptanalysis -- --nocapture
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: All PASS, build SUCCESS.

**Step 8: Smoke test CLI**

```bash
cargo run -p dcl-cli -- cryptanalysis --graph path --n 6 --modulus 65537
```
Expected: Cryptanalysis runs with modular PowerMap, prints report.

---

## Task 8: In-Place Evolve and --modulus on Scaling CLI

**Files:**
- Modify: `dcl-core/src/labeling.rs` (add `evolve_in_place` method)
- Modify: `dcl-cli/src/cmd_scaling.rs` (add `--modulus` flag)

**Step 1: Write the failing test for evolve_in_place**

Append to `#[cfg(test)] mod tests` in `dcl-core/src/labeling.rs`:

```rust
    #[test]
    fn evolve_in_place_matches_evolve() {
        let original = Labeling::new(vec![2, 3, 5, 7]);
        let pm = PowerMap::new(2);

        // evolve() produces new labeling
        let evolved = original.evolve(&pm);

        // evolve_in_place() modifies in-place
        let mut in_place = original.clone();
        in_place.evolve_in_place(&pm);

        assert_eq!(evolved.labels, in_place.labels, "evolve and evolve_in_place must produce identical results");
        assert_eq!(in_place.labels, vec![4, 9, 25, 49]);
    }
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p dcl-core evolve_in_place -- --nocapture
```
Expected: FAIL with "no method named `evolve_in_place`".

**Step 3: Implement evolve_in_place**

Add inside `impl Labeling` in `dcl-core/src/labeling.rs` (after the `evolve` method, line 90):

```rust
    /// Apply a transform in-place, modifying labels without allocation.
    /// f_{t+1}(v) = g(f_t(v)) for all v.
    pub fn evolve_in_place(&mut self, transform: &dyn Transform) {
        for label in &mut self.labels {
            *label = transform.apply(*label);
        }
    }
```

**Step 4: Run tests**

```bash
cargo test -p dcl-core evolve_in_place -- --nocapture
cargo test -p dcl-core -- --nocapture
```
Expected: All PASS.

**Step 5: Add --modulus flag to scaling CLI**

In `dcl-cli/src/cmd_scaling.rs`, add to `ScalingArgs` (after line 30):

```rust
    /// Optional prime modulus for bounded label evolution
    #[arg(long)]
    modulus: Option<u64>,
```

Update line 43 in the `run` function:

Old:
```rust
    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m, None);
```

New:
```rust
    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m, args.modulus);
```

**Step 6: Build and smoke test**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
cargo run -p dcl-cli -- scaling --graph path --min-size 4 --max-size 8 --step 2 --modulus 65537
```
Expected: Build SUCCESS, scaling runs with modular PowerMap.

---

## Task 9: Full Integration Test

**Step 1: Run all tests across all crates**

```bash
cargo test -p dcl-core -p dcl-security -p dcl-complexity
```
Expected: All tests PASS (33+ core, 33+ security, 9+ complexity).

**Step 2: Run full workspace build**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

**Step 3: Smoke test all new features**

```bash
# Binary exponentiation (Task 1) — verified via unit tests

# BFS zero-clone (Task 2)
cargo run -p dcl-cli -- optimized-bfs --graph path --n 4 --max-label 20

# Rayon scaling (Task 3)
cargo run -p dcl-cli -- scaling --graph path --min-size 4 --max-size 10 --step 2 --max-label 20

# Fixed frequency test (Task 4) — verified via unit tests

# NIST on DCL labels (Task 5)
cargo run -p dcl-cli -- nist-tests --dcl-mode --graph path --n 10 --steps 200

# Modular PowerMap cryptanalysis (Task 7)
cargo run -p dcl-cli -- cryptanalysis --graph path --n 6 --modulus 65537

# Scaling with modulus (Task 8)
cargo run -p dcl-cli -- scaling --graph path --min-size 4 --max-size 8 --step 2 --modulus 65537
```
Expected: All commands complete successfully.
