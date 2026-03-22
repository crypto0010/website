# DCL-RS Cost/Performance/Security Improvements — Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement the corresponding implementation plan.

**Goal:** Improve the DCL-RS workspace across three axes — security (resistance against cryptanalytic attacks including NIST tests), performance (high throughput), and cost (low resource footprint) — via 8 targeted changes across 4 crates.

**Approach:** Targeted improvements (Approach B). New functions where needed, no new crates, no architectural redesign. Reuse existing traits and patterns.

---

## Section 1: Performance — PowerMap Binary Exponentiation

**File**: `dcl-core/src/transform.rs` (lines 64-73)

**Problem**: Unbounded PowerMap uses O(m) naive loop with `saturating_mul`. No early exit when saturation occurs.

**Solution**: Replace with O(log m) binary exponentiation (same pattern as the modular case at lines 46-57). Add early termination when result reaches `u64::MAX`.

**Algorithm**:
- If x <= 1 or m == 0, return fast path
- result = 1u128, base = x as u128, exp = m
- While exp > 0: square-and-multiply with `saturating_mul`
- If result >= u64::MAX at any point, return u64::MAX immediately
- Clamp final result to u64

**Impact**: 10-100x faster for large exponents. Zero API change.

---

## Section 2: Performance — BFS In-Place Backtracking

**File**: `dcl-complexity/src/bfs_optimized.rs` (lines 148-203)

**Problem**: `stack.push((partial.clone(), pos + 1))` clones full `Vec<u64>` at every state. For Path-4 with max_label=20, ~160K clones.

**Solution**: Single mutable `Vec<u64>` with a stack of `(pos, candidate_index)` pairs. Restore `partial[pos]` on backtrack instead of cloning entire vectors.

Also fix `get_candidates()` (line 142) which clones `prime_cache` on every call — return a slice reference instead.

**Impact**: O(1) per state instead of O(n). Memory drops from O(states * n) to O(n + stack_depth). Same `BfsResult` output, same `PruningConfig` API.

---

## Section 3: Performance — Rayon Parallelism in Scaling

**Files**: `dcl-security/src/scaling.rs`, `dcl-security/Cargo.toml`

**Problem**: Sequential loop over graph sizes. Each size is independent but runs single-threaded.

**Solution**: Add `rayon = { workspace = true }` to dcl-security. Replace `for &n in sizes` with `sizes.par_iter().filter_map(...).collect()`. Each iteration creates its own Graph/Config/Cryptanalyzer with no shared mutable state.

Note: `println!` inside `Cryptanalyzer::analyze()` will interleave across threads. Acceptable for benchmark tool — final table prints after collection.

**Impact**: 4-8x speedup on multi-core.

---

## Section 4: Security — Fix Frequency Test Threshold

**File**: `dcl-security/src/cryptanalysis.rs` (line 407)

**Problem**: Threshold hardcoded to 500.0 instead of correct chi-squared critical value. With 100 samples across 256 bins (expected frequency 0.39/bin), chi-squared is meaningless.

**Solution**:
1. Adaptive bin count: `num_bins = data.len().min(64)` instead of fixed 256
2. Correct critical values via small lookup table for common df values (7, 15, 31, 63, 127, 255) at alpha=0.05
3. No external statistics crate needed

**Impact**: Frequency test now actually validates randomness. False pass rate drops from ~100% to correct 5%.

---

## Section 5: Security — NIST Tests on DCL Label Evolution

**Files**: `dcl-security/src/cryptanalysis.rs` (new function), `dcl-cli/src/cmd_nist_tests.rs` (new flag)

**Problem**: NIST tests run on SHA256(primes), never on actual DCL evolution sequences.

**Solution**:
1. New function `dcl_labels_to_test_data(graph, transform, initial, steps) -> Vec<u8>` — evolves labeling for T steps, appends each step's label bytes to output. Uses `evolve_in_place()` from Section 8.
2. New CLI flag `--dcl-mode` on `nist-tests`. When set, generates test data from DCL evolution with `--graph`, `--n`, `--steps`, `--m` parameters. Falls back to prime pipeline when unset.

For graph size 10 and 200 steps: 10 * 8 * 200 = 16000 bytes = 128000 bits. Sufficient for all 15 NIST tests.

**Impact**: NIST suite directly validates DCL security properties.

---

## Section 6: Security — HashTransform Coprimality Repair

**File**: `dcl-core/src/labeling.rs`

**Problem**: HashTransform docs say "repair step required" but no repair function exists.

**Solution**: Add `repair_coprimality(&mut self, graph: &Graph) -> usize` method on `Labeling`. For each edge (u,v) where gcd != 1, increment the larger label until coprime. Returns repair count.

Correctness: for any integer a, among a, a+1, a+2, ... there is always a value coprime to any fixed b within at most b steps (pigeonhole on prime factors). In practice 1-3 increments suffice.

No `evolve_with_repair()` convenience — caller does two explicit calls (YAGNI).

**Impact**: HashTransform becomes usable as real post-quantum alternative.

---

## Section 7: Security — Modular PowerMap in Cryptanalysis

**Files**: `dcl-security/src/cryptanalysis.rs`, `dcl-cli/src/cmd_cryptanalysis.rs`

**Problem**: All security tests use only `PowerMap::new()` (unbounded). Modular variant `PowerMap::with_modulus()` exists but is never tested.

**Solution**:
1. Add `modulus: Option<u64>` field to `CryptanalysisConfig` (default None)
2. Update all `PowerMap::new()` calls in Cryptanalyzer to check config modulus
3. Add `--modulus` CLI flag to cmd_cryptanalysis

**Impact**: Security analysis covers both bounded and unbounded modes. No breaking changes.

---

## Section 8: Cost — In-Place Evolve and --modulus on Scaling

**Files**: `dcl-core/src/labeling.rs`, `dcl-cli/src/cmd_scaling.rs`, `dcl-security/src/scaling.rs`

**Problem**: `evolve()` allocates new Vec every call. For multi-step chains (NIST data gen, BigInt evolution), creates unnecessary allocations.

**Solution**:
1. Add `evolve_in_place(&mut self, transform: &dyn Transform)` to Labeling. Zero allocation, modifies labels in-place. Existing `evolve()` unchanged.
2. Add `--modulus` flag to scaling CLI. Pass through to `CryptanalysisConfig::modulus`. Also add `modulus: Option<u64>` parameter to `run_scaling()`.

**Impact**: Eliminates N allocations per evolution chain. Scaling can evaluate modular mode.

---

## Files Changed Summary

| Crate | File | Sections |
|-------|------|----------|
| dcl-core | src/transform.rs | 1 |
| dcl-core | src/labeling.rs | 6, 8 |
| dcl-complexity | src/bfs_optimized.rs | 2 |
| dcl-security | Cargo.toml | 3 |
| dcl-security | src/scaling.rs | 3, 8 |
| dcl-security | src/cryptanalysis.rs | 4, 5, 7 |
| dcl-cli | src/cmd_nist_tests.rs | 5 |
| dcl-cli | src/cmd_cryptanalysis.rs | 7 |
| dcl-cli | src/cmd_scaling.rs | 8 |
