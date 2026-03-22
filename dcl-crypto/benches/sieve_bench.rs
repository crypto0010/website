// dcl-crypto/benches/sieve_bench.rs
//! Criterion benchmarks for HDCL-SP sieve vs standard safe-prime generation.

use criterion::{criterion_group, criterion_main, Criterion};
use dcl_core::labeling::Labeling;
use dcl_crypto::baseline::standard_safe_prime;
use dcl_crypto::sieve::{hdcl_sp_generate, SieveConfig};
use dcl_hypergraph::hypergraph::Hypergraph;

fn make_test_hypergraph() -> (Hypergraph, Labeling) {
    let mut h = Hypergraph::new(4);
    h.add_edge(vec![0, 1, 2]);
    h.add_edge(vec![1, 2, 3]);
    // Prime labels — pairwise and setwise coprime
    let labels = Labeling::new(vec![2, 3, 5, 7]);
    (h, labels)
}

fn bench_hdcl_sp(c: &mut Criterion) {
    let config = SieveConfig {
        mr_rounds: 10, // Fewer rounds for benchmarking speed
        delta: 1,
        max_attempts: 10_000,
    };
    c.bench_function("HDCL-SP safe-prime", |b| {
        b.iter(|| {
            let (h, mut labels) = make_test_hypergraph();
            hdcl_sp_generate(&h, &mut labels, &config)
        })
    });
}

fn bench_standard_safe_prime(c: &mut Criterion) {
    c.bench_function("Standard safe-prime (baseline)", |b| {
        let mut rng = rand::thread_rng();
        b.iter(|| standard_safe_prime(32, &mut rng))
    });
}

fn bench_miller_rabin(c: &mut Criterion) {
    use dcl_crypto::miller_rabin::miller_rabin;
    // Known safe prime for benchmarking
    let known_safe_prime: u64 = 1_073_741_789; // 2^30-35 — actually prime
    c.bench_function("Miller-Rabin 40 rounds", |b| {
        b.iter(|| miller_rabin(known_safe_prime, 40))
    });
}

criterion_group!(
    benches,
    bench_hdcl_sp,
    bench_standard_safe_prime,
    bench_miller_rabin
);
criterion_main!(benches);
