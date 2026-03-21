use axum::{Router, routing::get, Json};

pub fn benchmark_json() -> serde_json::Value {
    serde_json::json!({
        "gpu_vs_cpu": {
            "title": "GPU vs. CPU Benchmark Results (RTX 3050)",
            "operations": [
                {
                    "name": "Batch GCD",
                    "data": [
                        {"n": 50000, "gpu_ms": 7.03, "cpu_ms": 4.34, "speedup": 0.6},
                        {"n": 100000, "gpu_ms": 6.56, "cpu_ms": 8.67, "speedup": 1.3},
                        {"n": 500000, "gpu_ms": 9.62, "cpu_ms": 51.09, "speedup": 5.3},
                        {"n": 1000000, "gpu_ms": 12.47, "cpu_ms": 98.47, "speedup": 7.9}
                    ]
                },
                {
                    "name": "Power Map x³",
                    "data": [
                        {"n": 50000, "gpu_ms": 0.34, "cpu_ms": 1.36, "speedup": 4.0},
                        {"n": 100000, "gpu_ms": 0.67, "cpu_ms": 2.65, "speedup": 4.0},
                        {"n": 500000, "gpu_ms": 2.00, "cpu_ms": 13.70, "speedup": 6.9},
                        {"n": 1000000, "gpu_ms": 3.80, "cpu_ms": 26.53, "speedup": 7.0}
                    ]
                },
                {
                    "name": "Prime Sieve",
                    "data": [
                        {"n": 50000, "gpu_ms": 0.80, "cpu_ms": 0.90, "speedup": 1.1},
                        {"n": 100000, "gpu_ms": 1.24, "cpu_ms": 2.19, "speedup": 1.8},
                        {"n": 500000, "gpu_ms": 6.64, "cpu_ms": 18.76, "speedup": 2.8},
                        {"n": 1000000, "gpu_ms": 12.74, "cpu_ms": 43.96, "speedup": 3.4}
                    ]
                }
            ]
        },
        "kernel_throughput": {
            "title": "Peak GPU Throughput by Kernel Type",
            "kernels": [
                {"name": "Batch GCD", "throughput_mops": 80.2, "at_n": "1M"},
                {"name": "Power Map x³", "throughput_mops": 263.2, "at_n": "1M"},
                {"name": "Prime Sieve", "throughput_mops": 78.5, "at_n": "1M"},
                {"name": "Batch Evolve", "throughput_mops": 400.9, "at_n": "100K×10"},
                {"name": "Brute-Force Search", "throughput_mcands": 70.5, "at_n": "500K"},
                {"name": "NIST Data Gen", "throughput_mbps": 1.0, "at_n": "10v×200"}
            ]
        },
        "nist_results": {
            "title": "NIST SP 800-22 Statistical Test Results",
            "tests": [
                {"name": "Frequency (Monobit)", "result": "PASS"},
                {"name": "Block Frequency", "result": "PASS"},
                {"name": "Runs Test", "result": "PASS"},
                {"name": "Longest Run of Ones", "result": "PASS"},
                {"name": "Binary Matrix Rank", "result": "PASS"},
                {"name": "Discrete Fourier Transform", "result": "PASS"},
                {"name": "Non-overlapping Template", "result": "PASS"},
                {"name": "Overlapping Template", "result": "PASS"},
                {"name": "Linear Complexity", "result": "PASS"},
                {"name": "Serial Test", "result": "PASS"},
                {"name": "Approximate Entropy", "result": "PASS"},
                {"name": "Cumulative Sums (Forward)", "result": "PASS"},
                {"name": "Cumulative Sums (Reverse)", "result": "PASS"},
                {"name": "Maurer's Universal", "result": "FAIL"},
                {"name": "Random Excursions", "result": "PASS"}
            ],
            "summary": "14/15 PASS (93%)"
        },
        "brute_force": {
            "title": "GPU Brute-Force Coprime Labeling Search",
            "results": [
                {"graph": "P₄", "n": 4, "threads": 100000, "time_ms": 6.26, "mcands": 16.0, "found": 20},
                {"graph": "P₆", "n": 6, "threads": 500000, "time_ms": 7.09, "mcands": 70.5, "found": 20},
                {"graph": "C₆", "n": 6, "threads": 500000, "time_ms": 12.14, "mcands": 41.2, "found": 20},
                {"graph": "K₄", "n": 4, "threads": 500000, "time_ms": 8.66, "mcands": 57.7, "found": 20}
            ]
        }
    })
}

async fn benchmarks() -> Json<serde_json::Value> {
    Json(benchmark_json())
}

pub fn router() -> Router {
    Router::new()
        .route("/api/benchmarks", get(benchmarks))
}
