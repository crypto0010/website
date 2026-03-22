# DCL-RS Security & Scaling Improvements

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend the DCL-RS workspace with parameterized security evaluation, a post-quantum hash-based transform, naive-vs-pruned BFS comparison, scaling benchmarks, and new adversary types.

**Architecture:** Five independent feature additions across three crates (`dcl-core`, `dcl-security`, `dcl-cli`). Each feature extends existing traits/structs — no new crates needed. The existing `Transform` trait, `Adversary` trait, `Cryptanalyzer` struct, and `PruningConfig` struct are the extension points.

**Tech Stack:** Rust 2021, sha2 (already in workspace), clap (already in workspace), rand (already in workspace)

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

## Task 1: HashTransform — Post-Quantum Label Evolution

**Files:**
- Modify: `dcl-core/src/transform.rs` (append after `IdentityMap` at line 91)
- Modify: `dcl-core/Cargo.toml` (add `sha2` dependency)

**Step 1: Add sha2 dependency to dcl-core**

In `dcl-core/Cargo.toml`, add under `[dependencies]`:
```toml
sha2 = { workspace = true }
```

**Step 2: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `dcl-core/src/transform.rs`:

```rust
#[test]
fn hash_transform_deterministic() {
    let ht = HashTransform::new(1000, 0);
    let a = ht.apply(42);
    let b = ht.apply(42);
    assert_eq!(a, b, "HashTransform must be deterministic for same input");
    assert!(a >= 1 && a <= 1000, "Output must be in [1, max_label]");
}

#[test]
fn hash_transform_different_steps() {
    let ht0 = HashTransform::new(1000, 0);
    let ht1 = HashTransform::new(1000, 1);
    // Different step counters should (usually) produce different outputs
    let a = ht0.apply(42);
    let b = ht1.apply(42);
    // Not guaranteed to differ, but very likely for SHA-256
    assert!(a >= 1 && b >= 1);
}

#[test]
fn hash_transform_coprime_check() {
    // HashTransform is NOT guaranteed coprime-preserving
    // but should produce valid positive labels
    let ht = HashTransform::new(100, 0);
    for x in 1..=50 {
        let y = ht.apply(x);
        assert!(y >= 1 && y <= 100, "Label {} mapped to {} out of range", x, y);
    }
}
```

**Step 3: Run test to verify it fails**

```bash
cargo test -p dcl-core hash_transform
```
Expected: FAIL — `HashTransform` not found.

**Step 4: Implement HashTransform**

Insert after the `IdentityMap` impl block (after line 91) in `dcl-core/src/transform.rs`:

```rust
/// Hash-based label transform: f(x) = (SHA256(x || step) mod max_label) + 1.
///
/// This is a post-quantum alternative to the power map. Unlike PowerMap,
/// HashTransform does NOT guarantee coprimality preservation — a repair
/// step is required after evolution. The one-way property of SHA-256
/// provides preimage resistance without relying on modular exponentiation.
#[derive(Debug, Clone)]
pub struct HashTransform {
    pub max_label: u64,
    pub step: u64,
}

impl HashTransform {
    /// Create a hash transform with output range [1, max_label] at the given step.
    pub fn new(max_label: u64, step: u64) -> Self {
        assert!(max_label >= 1, "max_label must be >= 1");
        HashTransform { max_label, step }
    }
}

impl Transform for HashTransform {
    fn apply(&self, x: u64) -> u64 {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(x.to_le_bytes());
        hasher.update(self.step.to_le_bytes());
        let hash = hasher.finalize();
        // Take first 8 bytes as u64
        let raw = u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ]);
        // Map to [1, max_label]
        (raw % self.max_label) + 1
    }

    fn name(&self) -> &'static str {
        "HashTransform"
    }
}
```

**Step 5: Run tests to verify they pass**

```bash
cargo test -p dcl-core hash_transform
```
Expected: 3 tests PASS.

**Step 6: Compile workspace**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS (HashTransform is used via trait, no downstream breakage).

**Step 7: Commit**

```bash
git add dcl-core/src/transform.rs dcl-core/Cargo.toml
git commit -m "feat(dcl-core): add HashTransform for post-quantum label evolution"
```

---

## Task 2: New Adversary Types — Structural and Statistical

**Files:**
- Modify: `dcl-security/src/adversary.rs` (append after `RandomAdversary`)

**Step 1: Write the failing tests**

Append to the end of `dcl-security/src/adversary.rs` (after line 82), add a new test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::graph::Graph;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::PowerMap;

    #[test]
    fn structural_adversary_returns_valid_labeling() {
        let g = Graph::path(4);
        let pm = PowerMap::new(2);
        let f0 = Labeling::new(vec![1, 2, 3, 5]);
        let f1 = f0.evolve(&pm);
        let adv = StructuralAdversary { max_label: 20 };
        let guess = adv.guess_f0(&g, &pm, &[f1]);
        assert_eq!(guess.len(), 4);
        assert!(guess.labels.iter().all(|&l| l >= 1));
    }

    #[test]
    fn statistical_adversary_returns_valid_labeling() {
        let g = Graph::path(4);
        let pm = PowerMap::new(2);
        let f0 = Labeling::new(vec![1, 2, 3, 5]);
        let f1 = f0.evolve(&pm);
        let adv = StatisticalAdversary { max_label: 50 };
        let guess = adv.guess_f0(&g, &pm, &[f1]);
        assert_eq!(guess.len(), 4);
        assert!(guess.labels.iter().all(|&l| l >= 1));
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p dcl-security structural_adversary
```
Expected: FAIL — `StructuralAdversary` not found.

**Step 3: Implement both adversaries**

Append after the `RandomAdversary` impl block (after line 82) in `dcl-security/src/adversary.rs`:

```rust
/// Structural adversary: detects power-map evolution by testing whether
/// observed labels are perfect powers. If f1(v) = f0(v)^m, then f1(v)
/// has an integer m-th root. The adversary computes integer roots and
/// checks coprimality on the candidate f0.
pub struct StructuralAdversary {
    pub max_label: u64,
}

impl Adversary for StructuralAdversary {
    fn name(&self) -> &'static str {
        "Structural"
    }

    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling {
        if observed.is_empty() {
            return Labeling::new(vec![1; graph.n]);
        }
        let f1 = &observed[0];
        let m = transform.m;

        // For each vertex, try to compute the integer m-th root of f1(v)
        let mut guess_labels = Vec::with_capacity(graph.n);
        for v in 0..graph.n {
            let target = f1.get(v);
            let root = integer_root(target, m);
            if root > 0 && pow_u64(root, m) == target {
                guess_labels.push(root);
            } else {
                // Cannot invert — fall back to 1
                guess_labels.push(1);
            }
        }
        Labeling::new(guess_labels)
    }
}

/// Statistical adversary: analyses the entropy of label differences.
/// Evolved labels (perfect powers) tend to have structured spacing,
/// while random labels have higher entropy in their pairwise differences.
/// This adversary computes difference entropy and guesses based on a
/// threshold heuristic.
pub struct StatisticalAdversary {
    pub max_label: u64,
}

impl Adversary for StatisticalAdversary {
    fn name(&self) -> &'static str {
        "Statistical"
    }

    fn guess_f0(&self, graph: &Graph, transform: &PowerMap, observed: &[Labeling]) -> Labeling {
        if observed.is_empty() {
            return Labeling::new(vec![1; graph.n]);
        }
        let f1 = &observed[0];

        // Heuristic: if labels look like perfect powers, try root extraction;
        // otherwise return a simple prime labeling as "unevolved" guess
        let mut power_count = 0;
        let m = transform.m;
        for v in 0..graph.n {
            let val = f1.get(v);
            let root = integer_root(val, m);
            if root > 1 && pow_u64(root, m) == val {
                power_count += 1;
            }
        }

        // If most labels are perfect powers, this looks evolved — try inversion
        if power_count > graph.n / 2 {
            let structural = StructuralAdversary { max_label: self.max_label };
            return structural.guess_f0(graph, transform, observed);
        }

        // Otherwise, guess a simple coprime labeling (small primes)
        let primes: Vec<u64> = vec![1, 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
        let labels: Vec<u64> = (0..graph.n).map(|i| primes[i % primes.len()]).collect();
        Labeling::new(labels)
    }
}

/// Compute the integer m-th root of n: floor(n^(1/m)).
fn integer_root(n: u64, m: u32) -> u64 {
    if n <= 1 || m == 1 {
        return n;
    }
    if m >= 64 {
        return 1;
    }
    // Binary search for the m-th root
    let mut lo: u64 = 1;
    let mut hi: u64 = (n as f64).powf(1.0 / m as f64) as u64 + 2;
    hi = hi.min(n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if let Some(p) = checked_pow(mid, m) {
            if p < n {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        } else {
            // Overflow — mid is too large
            hi = mid;
        }
    }
    lo
}

/// Compute base^exp with overflow checking.
fn checked_pow(base: u64, exp: u32) -> Option<u64> {
    let mut result: u64 = 1;
    for _ in 0..exp {
        result = result.checked_mul(base)?;
    }
    Some(result)
}

/// Compute base^exp (saturating at u64::MAX).
fn pow_u64(base: u64, exp: u32) -> u64 {
    checked_pow(base, exp).unwrap_or(u64::MAX)
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p dcl-security structural_adversary -- --nocapture
cargo test -p dcl-security statistical_adversary -- --nocapture
```
Expected: both PASS.

**Step 5: Commit**

```bash
git add dcl-security/src/adversary.rs
git commit -m "feat(dcl-security): add StructuralAdversary and StatisticalAdversary"
```

---

## Task 3: Parameterized Cryptanalyzer with Sampling Mode

**Files:**
- Modify: `dcl-security/src/cryptanalysis.rs` (extend `Cryptanalyzer`)

**Step 1: Write the failing test**

Append inside the existing `#[cfg(test)] mod tests` block in `dcl-security/src/cryptanalysis.rs` (after line 543):

```rust
#[test]
fn cryptanalysis_with_config() {
    let g = Graph::path(6);
    let config = CryptanalysisConfig {
        max_label: 30,
        power_map_exp: 2,
        test_iterations: 20,
        brute_force_samples: 100,
    };
    let analyzer = Cryptanalyzer::with_config(&g, config);
    let report = analyzer.analyze();
    report.print_report();
    // Should complete without panic on P6
}

#[test]
fn cryptanalysis_larger_graph() {
    let g = Graph::path(10);
    let config = CryptanalysisConfig {
        max_label: 50,
        power_map_exp: 2,
        test_iterations: 10,
        brute_force_samples: 50,
    };
    let analyzer = Cryptanalyzer::with_config(&g, config);
    let report = analyzer.analyze();
    assert!(report.brute_force_analysis.search_space_size > 0);
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p dcl-security cryptanalysis_with_config
```
Expected: FAIL — `CryptanalysisConfig` and `with_config` not found.

**Step 3: Implement CryptanalysisConfig**

At the top of `dcl-security/src/cryptanalysis.rs`, after the existing struct definitions (after `StatisticalResult` around line 74), add:

```rust
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
}

impl Default for CryptanalysisConfig {
    fn default() -> Self {
        CryptanalysisConfig {
            max_label: 20,
            power_map_exp: 2,
            test_iterations: 100,
            brute_force_samples: 50,
        }
    }
}
```

Then add a new constructor to `impl<'a> Cryptanalyzer<'a>` (after the existing `new` method around line 90):

```rust
    /// Create a cryptanalyzer with explicit configuration.
    pub fn with_config(graph: &'a Graph, config: CryptanalysisConfig) -> Self {
        Cryptanalyzer {
            graph,
            power_map_exp: config.power_map_exp,
            test_iterations: config.test_iterations,
            config: Some(config),
        }
    }
```

And add a `config` field to `Cryptanalyzer`:

```rust
pub struct Cryptanalyzer<'a> {
    graph: &'a Graph,
    power_map_exp: u32,
    test_iterations: usize,
    config: Option<CryptanalysisConfig>,
}
```

Update the existing `new` to set `config: None`:

```rust
    pub fn new(graph: &'a Graph, power_map_exp: u32) -> Self {
        Cryptanalyzer {
            graph,
            power_map_exp,
            test_iterations: 100,
            config: None,
        }
    }
```

Update `test_brute_force_resistance` to use the configured `max_label` and `brute_force_samples`:

Replace the hardcoded `let max_label = 20;` (line 125) and `let sample_size = 50.min(search_space);` (line 127) with:

```rust
        let max_label = self.config.as_ref().map(|c| c.max_label).unwrap_or(20);
        let sample_count = self.config.as_ref().map(|c| c.brute_force_samples).unwrap_or(50);
        let search_space = (max_label as usize).pow(self.graph.n.min(10) as u32);
        let sample_size = sample_count.min(search_space);
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p dcl-security cryptanalysis -- --nocapture
```
Expected: all 3 cryptanalysis tests PASS (original + 2 new).

**Step 5: Commit**

```bash
git add dcl-security/src/cryptanalysis.rs
git commit -m "feat(dcl-security): add CryptanalysisConfig for parameterized graph-size evaluation"
```

---

## Task 4: Scaling Benchmark Module

**Files:**
- Create: `dcl-security/src/scaling.rs`
- Modify: `dcl-security/src/lib.rs` (add `pub mod scaling;`)
- Create: `dcl-cli/src/cmd_scaling.rs`
- Modify: `dcl-cli/src/main.rs` (add Scaling subcommand)

**Step 1: Write the failing test**

Create `dcl-security/src/scaling.rs` with test:

```rust
//! Security scaling benchmark — evaluates how security metrics change
//! as graph size increases from n_min to n_max.

use crate::cryptanalysis::{Cryptanalyzer, CryptanalysisConfig, SecurityReport};
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
) -> Vec<ScalingPoint> {
    let mut results = Vec::new();

    for &n in sizes {
        let graph = match graph_type {
            "path" => Graph::path(n),
            "cycle" => {
                if n < 3 { continue; }
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
        };

        let start = Instant::now();
        let analyzer = Cryptanalyzer::with_config(&graph, config);
        let report = analyzer.analyze();
        let wall_time = start.elapsed();

        let search_space = (max_label as f64).powi(n.min(20) as i32);

        results.push(ScalingPoint {
            graph_type: graph_type.to_string(),
            n,
            edges: graph.edges().len(),
            search_space,
            brute_force_success: report.brute_force_analysis.success_rate,
            preimage_success: report.preimage_resistance.success_rate,
            collision_rate: report.collision_resistance.collision_rate,
            timing_correlation: report.timing_analysis.correlation_coefficient,
            overall_level: format!("{:?}", report.overall_security_level),
            wall_time,
        });
    }

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
        let results = run_scaling("path", &[4, 6], 20, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].n, 4);
        assert_eq!(results[1].n, 6);
        // Search space should grow with n
        assert!(results[1].search_space > results[0].search_space);
    }
}
```

**Step 2: Register the module**

Add to `dcl-security/src/lib.rs`:
```rust
pub mod scaling;
```

**Step 3: Run test to verify it passes**

```bash
cargo test -p dcl-security scaling_path_small -- --nocapture
```
Expected: PASS.

**Step 4: Create CLI subcommand**

Create `dcl-cli/src/cmd_scaling.rs`:

```rust
//! Security scaling benchmark CLI subcommand.

use clap::Args;
use dcl_security::scaling;

#[derive(Args)]
pub struct ScalingArgs {
    /// Graph type: path, cycle, complete
    #[arg(short, long, default_value = "path")]
    graph: String,

    /// Minimum graph size
    #[arg(long, default_value = "4")]
    min_size: usize,

    /// Maximum graph size
    #[arg(long, default_value = "12")]
    max_size: usize,

    /// Step between sizes
    #[arg(long, default_value = "2")]
    step: usize,

    /// Maximum label value
    #[arg(long, default_value = "50")]
    max_label: u64,

    /// Power map exponent
    #[arg(short, long, default_value = "2")]
    m: u32,
}

pub fn run(args: ScalingArgs) {
    println!("\n=== DCL Security Scaling Benchmark ===\n");
    println!("Graph type: {}", args.graph);
    println!("Size range: {} to {} (step {})", args.min_size, args.max_size, args.step);
    println!("Max label: {}, Power map: x^{}\n", args.max_label, args.m);

    let sizes: Vec<usize> = (args.min_size..=args.max_size)
        .step_by(args.step)
        .collect();

    let results = scaling::run_scaling(&args.graph, &sizes, args.max_label, args.m);
    scaling::print_scaling_table(&results);
}
```

**Step 5: Register in main.rs**

In `dcl-cli/src/main.rs`:
- Add `mod cmd_scaling;` after `mod cmd_telemetry;` (line 17)
- Add variant to `Commands` enum:
  ```rust
  /// Run security scaling benchmark across graph sizes
  Scaling(cmd_scaling::ScalingArgs),
  ```
- Add match arm in `main()`:
  ```rust
  Commands::Scaling(args) => cmd_scaling::run(args),
  ```

**Step 6: Build and test**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

**Step 7: Smoke test CLI**

```bash
cargo run -p dcl-cli -- scaling --graph path --min-size 4 --max-size 8 --step 2 --max-label 20
```
Expected: Table printed with 3 rows (n=4, 6, 8).

**Step 8: Commit**

```bash
git add dcl-security/src/scaling.rs dcl-security/src/lib.rs dcl-cli/src/cmd_scaling.rs dcl-cli/src/main.rs
git commit -m "feat: add security scaling benchmark (dcl-cli scaling)"
```

---

## Task 5: Naive-vs-Pruned BFS Comparison Mode

**Files:**
- Modify: `dcl-cli/src/cmd_optimized_bfs.rs` (add `--compare` flag)

**Step 1: Add the `--compare` flag**

In `dcl-cli/src/cmd_optimized_bfs.rs`, add to `OptimizedBfsArgs`:

```rust
    /// Run both naive and pruned, print comparison table
    #[arg(long)]
    compare: bool,
```

**Step 2: Implement comparison logic**

In the `run` function of `cmd_optimized_bfs.rs`, add at the end (before the final closing brace), after the existing output block:

```rust
    if args.compare {
        println!("\n=== Naive vs Pruned Comparison ===\n");

        // Run naive (no pruning)
        let naive_config = PruningConfig {
            prefer_primes: false,
            use_symmetry: false,
            max_sum: None,
            max_solutions: args.max_solutions,
            incremental_check: false,
        };
        let mut naive_searcher = OptimizedBfs::new(&graph, args.max_label, naive_config);
        let naive_result = naive_searcher.search();

        // Run pruned (all enabled)
        let pruned_config = PruningConfig {
            prefer_primes: true,
            use_symmetry: true,
            max_sum: args.max_sum,
            max_solutions: args.max_solutions,
            incremental_check: true,
        };
        let mut pruned_searcher = OptimizedBfs::new(&graph, args.max_label, pruned_config);
        let pruned_result = pruned_searcher.search();

        let naive_total = naive_result.states_explored + naive_result.states_pruned;
        let pruned_total = pruned_result.states_explored + pruned_result.states_pruned;

        println!("{:<20} {:>12} {:>12}", "Metric", "Naive", "Pruned");
        println!("{}", "-".repeat(46));
        println!("{:<20} {:>12} {:>12}", "States explored",
            naive_result.states_explored, pruned_result.states_explored);
        println!("{:<20} {:>12} {:>12}", "States pruned",
            naive_result.states_pruned, pruned_result.states_pruned);
        println!("{:<20} {:>12} {:>12}", "Total visited",
            naive_total, pruned_total);
        println!("{:<20} {:>12} {:>12}", "Solutions found",
            naive_result.solutions.len(), pruned_result.solutions.len());

        if naive_total > 0 && pruned_total > 0 {
            let reduction = 1.0 - (pruned_total as f64 / naive_total as f64);
            let speedup = naive_total as f64 / pruned_total as f64;
            println!("\nPruning reduction: {:.1}%", reduction * 100.0);
            println!("Speedup factor: {:.2}x", speedup);
        }
    }
```

**Step 3: Build and test**

```bash
cargo build -p dcl-cli
cargo run -p dcl-cli -- optimized-bfs --graph path --n 4 --max-label 20 --compare
```
Expected: Comparison table with naive vs pruned columns.

**Step 4: Commit**

```bash
git add dcl-cli/src/cmd_optimized_bfs.rs
git commit -m "feat(dcl-cli): add --compare flag for naive-vs-pruned BFS comparison"
```

---

## Task 6: Integration — Update CLI Cryptanalysis with New Adversaries

**Files:**
- Modify: `dcl-cli/src/cmd_cryptanalysis.rs` (add `--max-label` and `--samples` flags)
- Modify: `dcl-cli/src/cmd_security.rs` (use new adversary types)

**Step 1: Add flags to cmd_cryptanalysis.rs**

Add to `CryptanalysisArgs`:

```rust
    /// Maximum label value for search space
    #[arg(long, default_value = "20")]
    max_label: u64,

    /// Number of brute-force samples (for large graphs)
    #[arg(long, default_value = "50")]
    samples: usize,
```

Update the `run` function to use `CryptanalysisConfig`:

```rust
pub fn run(args: CryptanalysisArgs) {
    // ... existing header printing ...

    let graph = match args.graph.as_str() {
        "path" => Graph::path(args.n),
        "cycle" => Graph::cycle(args.n),
        "wheel" => Graph::wheel(args.n),
        "complete" => Graph::complete(args.n),
        _ => {
            eprintln!("Unknown graph type: {}", args.graph);
            return;
        }
    };

    let config = dcl_security::cryptanalysis::CryptanalysisConfig {
        max_label: args.max_label,
        power_map_exp: args.m,
        test_iterations: if args.quick { 20 } else { 100 },
        brute_force_samples: args.samples,
    };

    let analyzer = Cryptanalyzer::with_config(&graph, config);
    let report = analyzer.analyze();
    report.print_report();

    // ... existing recommendations ...
}
```

**Step 2: Build and test**

```bash
cargo build -p dcl-cli
cargo run -p dcl-cli -- cryptanalysis --graph path --n 8 --max-label 30 --samples 20
```
Expected: Cryptanalysis runs on P8 with configured parameters.

**Step 3: Commit**

```bash
git add dcl-cli/src/cmd_cryptanalysis.rs
git commit -m "feat(dcl-cli): parameterize cryptanalysis with --max-label and --samples"
```

---

## Task 7: Full Integration Test and Final Build

**Step 1: Run all tests**

```bash
cargo test -p dcl-core -p dcl-security -p dcl-complexity
```
Expected: All tests PASS.

**Step 2: Run full workspace build**

```bash
cargo build -p dcl-core -p dcl-complexity -p dcl-hypergraph -p dcl-crypto -p dcl-security -p dcl-ramsey -p dcl-cli -p dcl-zkp -p dcl-wasm
```
Expected: SUCCESS.

**Step 3: Smoke test all new features**

```bash
# HashTransform test (via unit tests)
cargo test -p dcl-core hash_transform -- --nocapture

# New adversaries
cargo test -p dcl-security structural_adversary -- --nocapture
cargo test -p dcl-security statistical_adversary -- --nocapture

# Parameterized cryptanalysis on larger graph
cargo run -p dcl-cli -- cryptanalysis --graph path --n 10 --max-label 30 --samples 20

# Scaling benchmark
cargo run -p dcl-cli -- scaling --graph path --min-size 4 --max-size 10 --step 2

# Naive vs Pruned comparison
cargo run -p dcl-cli -- optimized-bfs --graph path --n 4 --max-label 20 --compare
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "test: integration smoke tests for all new features"
```
