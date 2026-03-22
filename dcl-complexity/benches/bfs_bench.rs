// dcl-complexity/benches/bfs_bench.rs
//! Criterion benchmarks for DCL-BFS on hypercubes and Erdős-Rényi graphs.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dcl_complexity::bfs::find_dcl_bfs;
use dcl_core::graph::Graph;

fn bench_bfs_hypercube(c: &mut Criterion) {
    let mut group = c.benchmark_group("DCL-BFS/Hypercube");
    for n in [3usize, 4, 5] {
        let g = Graph::hypercube(n);
        let max_label = g.n as u64 * 3;
        group.bench_with_input(BenchmarkId::new("Q_n", n), &(g, max_label), |b, (g, k)| {
            b.iter(|| find_dcl_bfs(g, *k))
        });
    }
    group.finish();
}

fn bench_bfs_erdos_renyi(c: &mut Criterion) {
    let mut group = c.benchmark_group("DCL-BFS/ErdosRenyi");
    for n in [10usize, 15, 20] {
        let mut rng = rand::thread_rng();
        let g = Graph::erdos_renyi(n, 0.3, &mut rng);
        let max_label = n as u64 * 4;
        group.bench_with_input(
            BenchmarkId::new("G(n,0.3)", n),
            &(g, max_label),
            |b, (g, k)| b.iter(|| find_dcl_bfs(g, *k)),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_bfs_hypercube, bench_bfs_erdos_renyi);
criterion_main!(benches);
