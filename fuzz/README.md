# DCL Framework Fuzzing Tests

This directory contains fuzz tests for the DCL (Dynamic Coprime Labeling) Framework using `cargo-fuzz` and `libfuzzer`.

## Overview

Fuzzing helps discover edge cases, crashes, and unexpected behavior by feeding randomized inputs to the code. We test critical components:

- **fuzz_labeling**: Tests DCL labeling operations, graph creation, and sequence evolution
- **fuzz_gcd**: Tests GCD, LCM, and coprimality algorithms for mathematical correctness
- **fuzz_miller_rabin**: Tests Miller-Rabin primality testing for determinism and correctness
- **fuzz_side_channel**: Tests constant-time operations and side-channel resistance

## Prerequisites

Install cargo-fuzz:

```bash
cargo install cargo-fuzz
```

Note: Fuzzing requires a nightly Rust compiler.

## Running Fuzz Tests

### Run a specific fuzz target

```bash
# Fuzz labeling operations
cargo +nightly fuzz run fuzz_labeling

# Fuzz GCD algorithms
cargo +nightly fuzz run fuzz_gcd

# Fuzz Miller-Rabin primality testing
cargo +nightly fuzz run fuzz_miller_rabin

# Fuzz side-channel resistant operations
cargo +nightly fuzz run fuzz_side_channel
```

### Run with custom options

```bash
# Run for a specific duration (e.g., 60 seconds)
cargo +nightly fuzz run fuzz_gcd -- -max_total_time=60

# Run with a specific number of iterations
cargo +nightly fuzz run fuzz_labeling -- -runs=100000

# Run with custom memory limit (in MB)
cargo +nightly fuzz run fuzz_side_channel -- -rss_limit_mb=2048

# Use multiple jobs for parallel fuzzing
cargo +nightly fuzz run fuzz_miller_rabin -- -jobs=4
```

## Understanding Results

### Successful Run

If fuzzing runs without finding issues, you'll see output like:

```
#1000000: cov: 234 ft: 567 corp: 89 exec/s: 12345
```

- `cov`: Code coverage (edges covered)
- `ft`: Features (interesting code paths)
- `corp`: Corpus size (number of interesting inputs saved)
- `exec/s`: Executions per second

### Crash Found

If a crash is found, cargo-fuzz will:

1. Stop execution
2. Save the failing input to `fuzz/artifacts/`
3. Display the crash details

To reproduce a crash:

```bash
cargo +nightly fuzz run fuzz_labeling fuzz/artifacts/fuzz_labeling/crash-<hash>
```

## Corpus Management

The fuzzing corpus (interesting inputs) is saved in:

```
fuzz/corpus/fuzz_labeling/
fuzz/corpus/fuzz_gcd/
fuzz/corpus/fuzz_miller_rabin/
fuzz/corpus/fuzz_side_channel/
```

### Minimize corpus

To reduce corpus size while maintaining coverage:

```bash
cargo +nightly fuzz cmin fuzz_labeling
```

### Merge corpora

To merge multiple corpus directories:

```bash
cargo +nightly fuzz cmin --merge fuzz_labeling corpus1/ corpus2/
```

## Coverage Reports

Generate coverage information:

```bash
cargo +nightly fuzz coverage fuzz_labeling
```

View coverage in HTML:

```bash
cargo cov -- show target/*/release/fuzz_labeling \
    --format=html \
    -instr-profile=fuzz/coverage/fuzz_labeling/coverage.profdata \
    > coverage.html
```

## Continuous Fuzzing

For long-running fuzzing campaigns:

```bash
# Run indefinitely until crash
cargo +nightly fuzz run fuzz_gcd -- -max_total_time=0

# Run with periodic stats
cargo +nightly fuzz run fuzz_miller_rabin -- -print_final_stats=1
```

## Target Descriptions

### fuzz_labeling

Tests DCL labeling creation, validation, and evolution:
- Label creation with various sizes
- Coprimality verification
- Graph construction
- Sequence evolution steps

**Properties verified:**
- No zero labels
- Valid index bounds
- Coprimality constraints
- Deterministic evolution

### fuzz_gcd

Tests mathematical correctness of GCD/LCM algorithms:
- GCD computation
- Coprimality checking
- LCM calculation
- Fundamental relation: `a * b = gcd(a,b) * lcm(a,b)`

**Properties verified:**
- GCD divides both inputs
- GCD is commutative
- GCD is associative
- LCM is divisible by both inputs

### fuzz_miller_rabin

Tests primality testing for correctness and determinism:
- Known primes always return true
- Known composites always return false
- Even numbers > 2 are composite
- Powers of 2 (except 2) are composite
- Carmichael numbers detection

**Properties verified:**
- Deterministic behavior
- Correct classification of small primes/composites
- Consistency across different round counts

### fuzz_side_channel

Tests constant-time operations for correctness:
- Constant-time GCD
- Constant-time byte comparison
- Constant-time selection
- Cache-resistant hashing
- Modular exponentiation

**Properties verified:**
- Correctness vs reference implementations
- Deterministic behavior
- Self-equality properties
- Batch operation consistency

## Best Practices

1. **Start with short runs** to catch obvious bugs quickly
2. **Use parallel jobs** (`-jobs=N`) to speed up fuzzing
3. **Minimize corpus regularly** to keep it manageable
4. **Run overnight** for thorough testing
5. **Share corpus** between developers for better coverage

## Troubleshooting

### Out of Memory

Reduce memory limit:
```bash
cargo +nightly fuzz run fuzz_labeling -- -rss_limit_mb=1024
```

### Slow Execution

Some targets may be slower. Use timeout:
```bash
cargo +nightly fuzz run fuzz_miller_rabin -- -timeout=1
```

### No Coverage Increase

The fuzzer may have exhausted easy paths. This is normal. Consider:
- Running longer
- Checking if all code paths are reachable
- Adding seed inputs to `corpus/` directory

## Integration with CI

Example GitHub Actions workflow:

```yaml
- name: Fuzz test
  run: |
    cargo install cargo-fuzz
    cargo +nightly fuzz run fuzz_gcd -- -max_total_time=60 -runs=1000000
```

## References

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [Rust Fuzzing Authority](https://github.com/rust-fuzz)
