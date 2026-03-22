# DCL CUDA Explorer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an interactive web application for testing and learning DCL CUDA operations with GPU acceleration and CPU fallback.

**Architecture:** Axum web server (`dcl-web/` crate) serving a single self-contained HTML file embedded via `include_str!`. REST API endpoints wrap dcl-gpu CUDA kernels with automatic CPU fallback to dcl-core. 6 interactive operations + pre-computed benchmark dashboard.

**Tech Stack:** Rust (Axum 0.7, Tokio, Serde), dcl-core, dcl-gpu, dcl-crypto, vanilla HTML/CSS/JS, SVG for graph visualization.

**Spec:** `docs/superpowers/specs/2026-03-21-dcl-web-explorer-design.md`

---

## File Structure

```
dcl-web/
├── Cargo.toml
├── src/
│   ├── main.rs              # Server startup, router, static file serving
│   ├── api/
│   │   ├── mod.rs           # Router aggregation, shared types
│   │   ├── health.rs        # GET /api/health
│   │   ├── gcd.rs           # POST /api/gcd
│   │   ├── power_map.rs     # POST /api/power-map
│   │   ├── coprime.rs       # POST /api/coprime-check
│   │   ├── evolve.rs        # POST /api/evolve
│   │   ├── repair.rs        # POST /api/repair
│   │   └── sieve.rs         # POST /api/prime-sieve
│   ├── gpu_backend.rs       # GPU/CPU dispatch layer
│   └── benchmark_data.rs    # Pre-computed paper results as JSON constants
└── static/
    └── index.html           # Single self-contained HTML/CSS/JS page
```

---

## Task 1: Scaffold `dcl-web` Crate

**Files:**
- Create: `dcl-web/Cargo.toml`
- Create: `dcl-web/src/main.rs`
- Modify: `Cargo.toml` (workspace root — add `dcl-web` to members)

- [ ] **Step 1: Create `dcl-web/Cargo.toml`**

```toml
[package]
name = "dcl-web"
version = "0.1.0"
edition = "2021"

[dependencies]
dcl-core = { path = "../dcl-core" }
dcl-gpu = { path = "../dcl-gpu" }
dcl-crypto = { path = "../dcl-crypto" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.5", features = ["cors"] }
```

- [ ] **Step 2: Create minimal `dcl-web/src/main.rs`**

```rust
use axum::{Router, routing::get, Json};
use serde_json::json;
use std::net::SocketAddr;

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/health", get(health));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🌐 DCL CUDA Explorer running at http://localhost:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 3: Add `dcl-web` to workspace `Cargo.toml`**

In `/home/csdf/C_Skills/dcl-rs/Cargo.toml`, add `"dcl-web"` to the `members` array.

- [ ] **Step 4: Verify it compiles**

Run: `cd /home/csdf/C_Skills/dcl-rs && cargo build -p dcl-web`
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add dcl-web/ Cargo.toml Cargo.lock
git commit -m "feat(dcl-web): scaffold Axum web server crate"
```

---

## Task 2: GPU Backend Abstraction Layer

**Files:**
- Create: `dcl-web/src/gpu_backend.rs`
- Modify: `dcl-web/src/main.rs` (add module declaration)

- [ ] **Step 1: Write test for GPU backend**

Create `dcl-web/src/gpu_backend.rs` with a test at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = DclBackend::new();
        // Should always succeed — falls back to CPU
        assert!(backend.backend_name() == "gpu" || backend.backend_name() == "cpu");
    }

    #[test]
    fn test_cpu_gcd() {
        let backend = DclBackend::new();
        let results = backend.batch_gcd(&[12, 35], &[8, 49]);
        assert_eq!(results[0].gcd, 4);
        assert!(!results[0].coprime);
        assert_eq!(results[1].gcd, 7);
        assert!(!results[1].coprime);
    }

    #[test]
    fn test_cpu_power_map() {
        let backend = DclBackend::new();
        let results = backend.power_map(&[2, 3, 5], 2, None);
        assert_eq!(results, vec![4, 9, 25]);
    }

    #[test]
    fn test_cpu_power_map_with_modulus() {
        let backend = DclBackend::new();
        let results = backend.power_map(&[2, 3, 5], 2, Some(65537));
        assert_eq!(results, vec![4, 9, 25]); // small values unaffected by large modulus
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p dcl-web -- gpu_backend`
Expected: FAIL — `DclBackend` not defined.

- [ ] **Step 3: Implement `DclBackend`**

```rust
use dcl_core::gcd;
use dcl_gpu::{CudaContext, GpuError};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize, Clone)]
pub struct GcdResult {
    pub a: u64,
    pub b: u64,
    pub gcd: u64,
    pub coprime: bool,
}

#[derive(Serialize, Clone)]
pub struct DeviceInfo {
    pub gpu_available: bool,
    pub device_name: String,
    pub backend: String,
}

pub struct DclBackend {
    gpu_ctx: Option<Arc<CudaContext>>,
    device_name: String,
}

impl DclBackend {
    pub fn new() -> Self {
        match CudaContext::new() {
            Ok(ctx) => {
                let name = ctx.device_info()
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|_| "Unknown GPU".to_string());
                println!("🔥 GPU: {}", name);
                DclBackend {
                    gpu_ctx: Some(Arc::new(ctx)),
                    device_name: name,
                }
            }
            Err(_) => {
                println!("⚠️  No GPU detected — running in CPU mode");
                DclBackend {
                    gpu_ctx: None,
                    device_name: "CPU".to_string(),
                }
            }
        }
    }

    pub fn backend_name(&self) -> &str {
        if self.gpu_ctx.is_some() { "gpu" } else { "cpu" }
    }

    pub fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            gpu_available: self.gpu_ctx.is_some(),
            device_name: self.device_name.clone(),
            backend: self.backend_name().to_string(),
        }
    }

    pub fn batch_gcd(&self, a: &[u64], b: &[u64]) -> Vec<GcdResult> {
        // Try GPU first
        if let Some(ctx) = &self.gpu_ctx {
            if let Ok(gpu_gcd) = dcl_gpu::kernels::gcd::GpuGcdBatch::new((*ctx.clone()).clone()) {
                if let Ok((gcds, coprimes)) = gpu_gcd.compute_batch(a, b) {
                    return a.iter().zip(b.iter()).zip(gcds.iter().zip(coprimes.iter()))
                        .map(|((&ai, &bi), (&g, &c))| GcdResult {
                            a: ai, b: bi, gcd: g, coprime: c != 0,
                        })
                        .collect();
                }
            }
        }
        // CPU fallback
        a.iter().zip(b.iter())
            .map(|(&ai, &bi)| {
                let g = gcd::gcd(ai, bi);
                GcdResult { a: ai, b: bi, gcd: g, coprime: g == 1 }
            })
            .collect()
    }

    pub fn power_map(&self, labels: &[u64], m: u32, modulus: Option<u64>) -> Vec<u64> {
        if let Some(ctx) = &self.gpu_ctx {
            if let Ok(gpu_pm) = dcl_gpu::kernels::power_map::GpuPowerMap::new((*ctx.clone()).clone()) {
                let mod_val = modulus.unwrap_or(0);
                if let Ok(result) = gpu_pm.apply_batch(labels, m, mod_val) {
                    return result;
                }
            }
        }
        // CPU fallback: binary exponentiation
        labels.iter().map(|&x| {
            cpu_power_map(x, m, modulus)
        }).collect()
    }

    pub fn coprime_check(&self, labels: &[u64], edges: &[(usize, usize)]) -> Vec<GcdResult> {
        edges.iter().map(|&(u, v)| {
            let a = labels[u];
            let b = labels[v];
            let g = gcd::gcd(a, b);
            GcdResult { a, b, gcd: g, coprime: g == 1 }
        }).collect()
    }

    pub fn evolve(
        &self,
        labels: &[u64],
        m: u32,
        modulus: Option<u64>,
        steps: usize,
    ) -> Vec<Vec<u64>> {
        let mut all_steps = Vec::with_capacity(steps + 1);
        let mut current = labels.to_vec();
        all_steps.push(current.clone());

        for _ in 0..steps {
            current = self.power_map(&current, m, modulus);
            all_steps.push(current.clone());
        }
        all_steps
    }

    pub fn coprime_repair(
        &self,
        labels: &[u64],
        edges: &[(usize, usize)],
    ) -> (Vec<u64>, Vec<(usize, usize, u64, u64)>) {
        let mut repaired = labels.to_vec();
        let mut changes = Vec::new();

        for &(u, v) in edges {
            if gcd::gcd(repaired[u], repaired[v]) != 1 {
                let old = repaired[v];
                // Increment larger label until coprime
                while gcd::gcd(repaired[u], repaired[v]) != 1 {
                    repaired[v] += 1;
                }
                changes.push((u, v, old, repaired[v]));
            }
        }
        (repaired, changes)
    }

    /// Find safe primes: p where both p and (p-1)/2 are prime.
    pub fn prime_sieve(&self, start: u64, count: usize) -> Vec<u64> {
        let mut safe_primes = Vec::new();
        let mut candidate = if start < 5 { 5 } else { start | 1 }; // start odd
        while safe_primes.len() < count && candidate < u64::MAX - 1000 {
            if dcl_crypto::miller_rabin::miller_rabin(candidate, 40) {
                let q = (candidate - 1) / 2;
                if dcl_crypto::miller_rabin::miller_rabin(q, 40) {
                    safe_primes.push(candidate);
                }
            }
            candidate += 2; // only check odd numbers
        }
        safe_primes
    }
}

fn cpu_power_map(x: u64, m: u32, modulus: Option<u64>) -> u64 {
    if x <= 1 || m == 0 { return x; }
    match modulus {
        Some(n) if n > 0 => {
            let mut result: u128 = 1;
            let mut base: u128 = (x % n) as u128;
            let mut exp = m;
            let n128 = n as u128;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = (result * base) % n128;
                }
                base = (base * base) % n128;
                exp >>= 1;
            }
            result as u64
        }
        _ => {
            // Unbounded with saturation
            let mut result: u128 = 1;
            let mut base: u128 = x as u128;
            let mut exp = m;
            let max_val = u64::MAX as u128;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = result.saturating_mul(base);
                    if result > max_val { return u64::MAX; }
                }
                base = base.saturating_mul(base);
                if base > max_val && exp > 1 { base = max_val; }
                exp >>= 1;
            }
            result as u64
        }
    }
}
```

- [ ] **Step 4: Add module declaration to `main.rs`**

Add `mod gpu_backend;` at the top of `dcl-web/src/main.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p dcl-web -- gpu_backend`
Expected: All 4 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add dcl-web/src/gpu_backend.rs dcl-web/src/main.rs
git commit -m "feat(dcl-web): add GPU/CPU backend abstraction layer"
```

---

## Task 3: API Endpoints — Health + GCD

**Files:**
- Create: `dcl-web/src/api/mod.rs`
- Create: `dcl-web/src/api/health.rs`
- Create: `dcl-web/src/api/gcd.rs`
- Modify: `dcl-web/src/main.rs` (add API router)

- [ ] **Step 1: Create `dcl-web/src/api/mod.rs`**

```rust
pub mod health;
pub mod gcd;

use axum::Router;
use std::sync::Arc;
use crate::gpu_backend::DclBackend;

pub type SharedBackend = Arc<DclBackend>;

pub fn api_router(backend: SharedBackend) -> Router {
    Router::new()
        .merge(health::router(backend.clone()))
        .merge(gcd::router(backend.clone()))
}
```

- [ ] **Step 2: Create `dcl-web/src/api/health.rs`**

```rust
use axum::{Router, routing::get, Json, extract::State};
use serde::Serialize;
use crate::api::SharedBackend;

#[derive(Serialize)]
struct HealthResponse {
    gpu_available: bool,
    device_name: String,
    backend: String,
}

async fn health(State(backend): State<SharedBackend>) -> Json<HealthResponse> {
    let info = backend.device_info();
    Json(HealthResponse {
        gpu_available: info.gpu_available,
        device_name: info.device_name,
        backend: info.backend,
    })
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .with_state(backend)
}
```

- [ ] **Step 3: Create `dcl-web/src/api/gcd.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;
use crate::gpu_backend::GcdResult;

#[derive(Deserialize)]
pub struct GcdRequest {
    pairs: Vec<[u64; 2]>,
}

#[derive(Serialize)]
pub struct GcdResponse {
    results: Vec<GcdResult>,
    backend: String,
    time_us: u64,
}

async fn batch_gcd(
    State(backend): State<SharedBackend>,
    Json(req): Json<GcdRequest>,
) -> Result<Json<GcdResponse>, Json<serde_json::Value>> {
    if req.pairs.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "At least one pair is required",
            "code": "INVALID_INPUT"
        })));
    }
    if req.pairs.len() > 10_000 {
        return Err(Json(serde_json::json!({
            "error": "Maximum 10,000 pairs per request",
            "code": "INVALID_INPUT"
        })));
    }

    let a: Vec<u64> = req.pairs.iter().map(|p| p[0]).collect();
    let b: Vec<u64> = req.pairs.iter().map(|p| p[1]).collect();

    let start = Instant::now();
    let results = backend.batch_gcd(&a, &b);
    let time_us = start.elapsed().as_micros() as u64;

    Ok(Json(GcdResponse {
        results,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/gcd", post(batch_gcd))
        .with_state(backend)
}
```

- [ ] **Step 4: Update `main.rs` to use API router**

```rust
mod api;
mod gpu_backend;

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use gpu_backend::DclBackend;

#[tokio::main]
async fn main() {
    let backend = Arc::new(DclBackend::new());

    let app = Router::new()
        .merge(api::api_router(backend));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🌐 DCL CUDA Explorer running at http://localhost:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build -p dcl-web`
Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add dcl-web/src/api/
git commit -m "feat(dcl-web): add health and batch GCD API endpoints"
```

---

## Task 4: API Endpoints — Power Map, Coprime Check, Evolve, Repair, Sieve

**Files:**
- Create: `dcl-web/src/api/power_map.rs`
- Create: `dcl-web/src/api/coprime.rs`
- Create: `dcl-web/src/api/evolve.rs`
- Create: `dcl-web/src/api/repair.rs`
- Create: `dcl-web/src/api/sieve.rs`
- Modify: `dcl-web/src/api/mod.rs` (register routes)

- [ ] **Step 1: Create `dcl-web/src/api/power_map.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;

#[derive(Deserialize)]
pub struct PowerMapRequest {
    labels: Vec<u64>,
    exponent: u32,
    modulus: Option<u64>,
}

#[derive(Serialize)]
pub struct PowerMapResponse {
    original: Vec<u64>,
    result: Vec<u64>,
    exponent: u32,
    modulus: Option<u64>,
    backend: String,
    time_us: u64,
}

async fn power_map(
    State(backend): State<SharedBackend>,
    Json(req): Json<PowerMapRequest>,
) -> Result<Json<PowerMapResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "At least one label is required",
            "code": "INVALID_INPUT"
        })));
    }
    if req.exponent < 2 {
        return Err(Json(serde_json::json!({
            "error": "Exponent must be >= 2",
            "code": "INVALID_INPUT"
        })));
    }
    if req.labels.iter().any(|&l| l == 0) {
        return Err(Json(serde_json::json!({
            "error": "Labels must be positive integers (> 0)",
            "code": "INVALID_INPUT"
        })));
    }

    let start = Instant::now();
    let result = backend.power_map(&req.labels, req.exponent, req.modulus);
    let time_us = start.elapsed().as_micros() as u64;

    Ok(Json(PowerMapResponse {
        original: req.labels,
        result,
        exponent: req.exponent,
        modulus: req.modulus,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/power-map", post(power_map))
        .with_state(backend)
}
```

- [ ] **Step 2: Create `dcl-web/src/api/coprime.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;
use crate::gpu_backend::GcdResult;

#[derive(Deserialize)]
pub struct CoprimeRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
}

#[derive(Serialize)]
pub struct CoprimeResponse {
    results: Vec<GcdResult>,
    all_coprime: bool,
    backend: String,
    time_us: u64,
}

async fn coprime_check(
    State(backend): State<SharedBackend>,
    Json(req): Json<CoprimeRequest>,
) -> Result<Json<CoprimeResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "At least one label is required",
            "code": "INVALID_INPUT"
        })));
    }
    if req.edges.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "At least one edge is required",
            "code": "INVALID_INPUT"
        })));
    }
    let n = req.labels.len();
    for edge in &req.edges {
        if edge[0] >= n || edge[1] >= n {
            return Err(Json(serde_json::json!({
                "error": format!("Edge ({}, {}) references vertex >= {} (num vertices)", edge[0], edge[1], n),
                "code": "INVALID_INPUT"
            })));
        }
    }

    let edges: Vec<(usize, usize)> = req.edges.iter().map(|e| (e[0], e[1])).collect();

    let start = Instant::now();
    let results = backend.coprime_check(&req.labels, &edges);
    let time_us = start.elapsed().as_micros() as u64;
    let all_coprime = results.iter().all(|r| r.coprime);

    Ok(Json(CoprimeResponse {
        results,
        all_coprime,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/coprime-check", post(coprime_check))
        .with_state(backend)
}
```

- [ ] **Step 3: Create `dcl-web/src/api/evolve.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;
use dcl_core::gcd;

#[derive(Deserialize)]
pub struct EvolveRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
    exponent: u32,
    modulus: Option<u64>,
    steps: usize,
}

#[derive(Serialize)]
pub struct EvolveStep {
    t: usize,
    labels: Vec<u64>,
    coprime: bool,
}

#[derive(Serialize)]
pub struct EvolveResponse {
    steps: Vec<EvolveStep>,
    backend: String,
    time_us: u64,
}

async fn evolve(
    State(backend): State<SharedBackend>,
    Json(req): Json<EvolveRequest>,
) -> Result<Json<EvolveResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "At least one label is required",
            "code": "INVALID_INPUT"
        })));
    }
    if req.labels.iter().any(|&l| l == 0) {
        return Err(Json(serde_json::json!({
            "error": "Labels must be positive integers (> 0)",
            "code": "INVALID_INPUT"
        })));
    }
    if req.exponent < 2 {
        return Err(Json(serde_json::json!({
            "error": "Exponent must be >= 2",
            "code": "INVALID_INPUT"
        })));
    }
    if req.steps > 50 {
        return Err(Json(serde_json::json!({
            "error": "Maximum 50 evolution steps",
            "code": "INVALID_INPUT"
        })));
    }
    if req.labels.len() > 100 {
        return Err(Json(serde_json::json!({
            "error": "Maximum 100 vertices",
            "code": "INVALID_INPUT"
        })));
    }

    let edges: Vec<(usize, usize)> = req.edges.iter().map(|e| (e[0], e[1])).collect();

    let start = Instant::now();
    let all_labels = backend.evolve(&req.labels, req.exponent, req.modulus, req.steps);
    let time_us = start.elapsed().as_micros() as u64;

    let steps: Vec<EvolveStep> = all_labels.iter().enumerate().map(|(t, labels)| {
        let coprime = edges.iter().all(|&(u, v)| gcd::are_coprime(labels[u], labels[v]));
        EvolveStep { t, labels: labels.clone(), coprime }
    }).collect();

    Ok(Json(EvolveResponse {
        steps,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/evolve", post(evolve))
        .with_state(backend)
}
```

- [ ] **Step 4: Create `dcl-web/src/api/repair.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;

#[derive(Deserialize)]
pub struct RepairRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
}

#[derive(Serialize)]
pub struct RepairChange {
    edge: [usize; 2],
    old_label: u64,
    new_label: u64,
    vertex: usize,
}

#[derive(Serialize)]
pub struct RepairResponse {
    original: Vec<u64>,
    repaired: Vec<u64>,
    changes: Vec<RepairChange>,
    backend: String,
    time_us: u64,
}

async fn repair(
    State(backend): State<SharedBackend>,
    Json(req): Json<RepairRequest>,
) -> Result<Json<RepairResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() || req.edges.is_empty() {
        return Err(Json(serde_json::json!({
            "error": "Labels and edges are required",
            "code": "INVALID_INPUT"
        })));
    }

    let edges: Vec<(usize, usize)> = req.edges.iter().map(|e| (e[0], e[1])).collect();

    let start = Instant::now();
    let (repaired, raw_changes) = backend.coprime_repair(&req.labels, &edges);
    let time_us = start.elapsed().as_micros() as u64;

    let changes = raw_changes.iter().map(|&(u, v, old, new)| RepairChange {
        edge: [u, v],
        old_label: old,
        new_label: new,
        vertex: v,
    }).collect();

    Ok(Json(RepairResponse {
        original: req.labels,
        repaired,
        changes,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/repair", post(repair))
        .with_state(backend)
}
```

- [ ] **Step 5: Create `dcl-web/src/api/sieve.rs`**

```rust
use axum::{Router, routing::post, Json, extract::State};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::api::SharedBackend;

#[derive(Deserialize)]
pub struct SieveRequest {
    start: Option<u64>,
    count: usize,
}

#[derive(Serialize)]
pub struct SieveResponse {
    safe_primes: Vec<u64>,
    count: usize,
    backend: String,
    time_us: u64,
}

async fn prime_sieve(
    State(backend): State<SharedBackend>,
    Json(req): Json<SieveRequest>,
) -> Result<Json<SieveResponse>, Json<serde_json::Value>> {
    if req.count == 0 || req.count > 1000 {
        return Err(Json(serde_json::json!({
            "error": "Count must be between 1 and 1000",
            "code": "INVALID_INPUT"
        })));
    }

    let start_val = req.start.unwrap_or(2);
    let start = Instant::now();
    let safe_primes = backend.prime_sieve(start_val, req.count);
    let time_us = start.elapsed().as_micros() as u64;

    Ok(Json(SieveResponse {
        count: safe_primes.len(),
        safe_primes,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/prime-sieve", post(prime_sieve))
        .with_state(backend)
}
```

- [ ] **Step 6: Update `dcl-web/src/api/mod.rs` with all routes**

```rust
pub mod health;
pub mod gcd;
pub mod power_map;
pub mod coprime;
pub mod evolve;
pub mod repair;
pub mod sieve;

use axum::Router;
use std::sync::Arc;
use crate::gpu_backend::DclBackend;

pub type SharedBackend = Arc<DclBackend>;

pub fn api_router(backend: SharedBackend) -> Router {
    Router::new()
        .merge(health::router(backend.clone()))
        .merge(gcd::router(backend.clone()))
        .merge(power_map::router(backend.clone()))
        .merge(coprime::router(backend.clone()))
        .merge(evolve::router(backend.clone()))
        .merge(repair::router(backend.clone()))
        .merge(sieve::router(backend.clone()))
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo build -p dcl-web`
Expected: Compiles successfully.

- [ ] **Step 8: Commit**

```bash
git add dcl-web/src/api/
git commit -m "feat(dcl-web): add all 6 interactive API endpoints"
```

---

## Task 5: Pre-computed Benchmark Data

**Files:**
- Create: `dcl-web/src/benchmark_data.rs`
- Modify: `dcl-web/src/main.rs` (add module + route)

- [ ] **Step 1: Create `dcl-web/src/benchmark_data.rs`**

Embed the key results from the GPU research paper as JSON constants:

```rust
use axum::{Router, routing::get, Json};

/// Pre-computed benchmark data from the GPU research paper.
/// GPU vs CPU scaling (Table IX), kernel throughput (Table X),
/// NIST SP 800-22 results (Table VII), brute-force search (Table XIII).
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
```

- [ ] **Step 2: Add module and route to `main.rs`**

Add `mod benchmark_data;` and merge `benchmark_data::router()` into the app router.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p dcl-web`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add dcl-web/src/benchmark_data.rs dcl-web/src/main.rs
git commit -m "feat(dcl-web): add pre-computed benchmark data from GPU paper"
```

---

## Task 6: Graph Preset Endpoint

**Files:**
- Create: `dcl-web/src/api/graph_presets.rs`
- Modify: `dcl-web/src/api/mod.rs` (register route)

- [ ] **Step 1: Create `dcl-web/src/api/graph_presets.rs`**

```rust
use axum::{Router, routing::get, Json, extract::Query};
use serde::{Deserialize, Serialize};
use dcl_core::graph::Graph;

#[derive(Deserialize)]
pub struct PresetQuery {
    family: String,
    n: usize,
}

#[derive(Serialize)]
pub struct GraphData {
    vertices: Vec<u64>,
    edges: Vec<[usize; 2]>,
    family: String,
    n: usize,
}

/// Generate initial coprime labels: first n primes
fn initial_labels(n: usize) -> Vec<u64> {
    let primes = [2,3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,
                  73,79,83,89,97,101,103,107,109,113,127,131,137,139,149,151,
                  157,163,167,173,179,181,191,193,197,199,211,223,227,229,233,
                  239,241,251,257,263,269,271,277,281,283,293,307,311,313,317,
                  331,337,347,349,353,359,367,373,379,383,389,397,401,409,419,
                  421,431,433,439,443,449,457,461,463,467,479,487,491,499,503];
    primes[..n.min(primes.len())].to_vec()
}

async fn presets(
    Query(params): Query<PresetQuery>,
) -> Result<Json<GraphData>, Json<serde_json::Value>> {
    if params.n < 2 || params.n > 32 {
        return Err(Json(serde_json::json!({
            "error": "n must be between 2 and 32",
            "code": "INVALID_INPUT"
        })));
    }

    let graph = match params.family.as_str() {
        "path" => Graph::path(params.n),
        "cycle" => Graph::cycle(params.n),
        "wheel" => Graph::wheel(params.n),
        "complete" => {
            if params.n > 20 {
                return Err(Json(serde_json::json!({
                    "error": "Complete graphs limited to n <= 20",
                    "code": "INVALID_INPUT"
                })));
            }
            Graph::complete(params.n)
        }
        "hypercube" => {
            if params.n > 5 {
                return Err(Json(serde_json::json!({
                    "error": "Hypercube dimension limited to d <= 5 (32 vertices)",
                    "code": "INVALID_INPUT"
                })));
            }
            Graph::hypercube(params.n)
        }
        _ => {
            return Err(Json(serde_json::json!({
                "error": format!("Unknown graph family: {}. Use: path, cycle, wheel, complete, hypercube", params.family),
                "code": "INVALID_INPUT"
            })));
        }
    };

    let edges: Vec<[usize; 2]> = graph.edges().iter().map(|&(u, v)| [u, v]).collect();
    let labels = initial_labels(params.n);

    Ok(Json(GraphData {
        vertices: labels,
        edges,
        family: params.family,
        n: params.n,
    }))
}

pub fn router() -> Router {
    Router::new()
        .route("/api/graph-presets", get(presets))
}
```

- [ ] **Step 2: Register in `api/mod.rs`**

Add `pub mod graph_presets;` and merge `graph_presets::router()` into `api_router`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p dcl-web`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add dcl-web/src/api/graph_presets.rs dcl-web/src/api/mod.rs
git commit -m "feat(dcl-web): add graph preset generation endpoint"
```

---

## Task 7: Frontend HTML — Page Shell + Navigation + Learn Section

**Files:**
- Create: `dcl-web/static/index.html`
- Modify: `dcl-web/src/main.rs` (serve static HTML)

This is the largest task. The single HTML file contains all CSS, JS, and markup. We build it incrementally: shell first, then each section.

- [ ] **Step 1: Create `dcl-web/static/index.html` with page shell**

Create the HTML file with:
- Dark theme CSS (all inline in `<style>`)
- Sticky navigation bar with section links
- Hero/header section with GPU status indicator
- Empty placeholder `<section>` elements for Learn, Playground, Graph Workspace, Benchmarks
- JS module structure: `fetchApi()` helper, `initApp()` on DOMContentLoaded that calls `/api/health`

The CSS should define:
- Dark background (`#0f172a`), light text (`#e2e8f0`)
- Card styling with `#1e293b` background, rounded corners
- Green (`#22c55e`) for coprime/pass, red (`#ef4444`) for fail
- Monospace font for numbers
- Responsive grid layout
- Navigation: sticky top bar with blur backdrop

The JS should define:
- `async function fetchApi(endpoint, body)` — POST JSON, return parsed response
- `async function fetchGet(endpoint)` — GET JSON
- `initApp()` — check health endpoint, update GPU status badge

- [ ] **Step 2: Add the Learn section**

Three collapsible theory cards inside `<section id="learn">`:

**Card 1: "What is Coprime Labeling?"**
- Definition: labels on vertices where adjacent vertices have gcd = 1
- Inline SVG example showing P₅ with labels [2,3,5,7,11]
- "Try it →" button linking to #coprime-check

**Card 2: "The Power Map g(x) = xᵐ"**
- Explanation: labels evolve as f_{t+1}(v) = f_t(v)^m
- Key theorem: if gcd(a,b)=1 then gcd(a^m, b^m)=1
- Table showing P₅ evolution for 3 steps
- "Try it →" button linking to #evolve

**Card 3: "GPU Acceleration"**
- Why DCL is embarrassingly parallel (each vertex/edge independent)
- 8 CUDA kernels, peak 263 M ops/s
- "See benchmarks →" button linking to #benchmarks

Each card uses a `<details>` element for collapse/expand.

- [ ] **Step 3: Update `main.rs` to serve the HTML**

Add a route that serves the embedded HTML:

```rust
use axum::response::Html;

const INDEX_HTML: &str = include_str!("../static/index.html");

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

// In main():
let app = Router::new()
    .route("/", get(index))
    .merge(api::api_router(backend))
    .merge(benchmark_data::router());
```

- [ ] **Step 4: Verify the server starts and HTML loads**

Run: `cargo run -p dcl-web &` then `curl -s http://localhost:3000/ | head -5`
Expected: Returns HTML starting with `<!DOCTYPE html>`.
Clean up: kill the background process.

- [ ] **Step 5: Commit**

```bash
git add dcl-web/static/index.html dcl-web/src/main.rs
git commit -m "feat(dcl-web): add HTML shell with navigation and learn section"
```

---

## Task 8: Frontend — Interactive Playground Panels (GCD, Power Map, Coprime Check)

**Files:**
- Modify: `dcl-web/static/index.html`

- [ ] **Step 1: Add Batch GCD panel**

Inside `<section id="playground">`, add a panel with:
- Textarea for entering number pairs (one pair per line, comma-separated)
- Pre-filled example: `12,8\n35,49\n97,13`
- "Run GCD ▶" button
- Results table: columns for a, b, gcd(a,b), Coprime? (green check / red X)
- Timing + backend indicator below results

JS handler: parse textarea, call `POST /api/gcd`, render table.

- [ ] **Step 2: Add Power Map panel**

Panel with:
- Input fields: labels (comma-separated), exponent m (number input, default 2), modulus N (optional number input)
- Pre-filled: labels = `2,3,5,7,11`, m = `2`
- "Apply Power Map ▶" button
- Results: two-row comparison table (original → result)
- If any result is `u64::MAX`, show overflow warning with "Retry with modulus 65537" button

JS handler: parse inputs, call `POST /api/power-map`, render comparison.

- [ ] **Step 3: Add Coprime Check panel**

Panel with:
- "Use current graph" button (pulls from graph workspace state)
- Or manual input: labels + edges textareas
- "Check Coprimality ▶" button
- Results: edge-by-edge table with color-coded pass/fail
- Summary: "All coprime ✓" or "X violations found"
- Educational message for failures explaining why gcd ≠ 1

JS handler: call `POST /api/coprime-check`, render results.

- [ ] **Step 4: Commit**

```bash
git add dcl-web/static/index.html
git commit -m "feat(dcl-web): add GCD, Power Map, and Coprime Check panels"
```

---

## Task 9: Frontend — Interactive Playground Panels (Evolve, Repair, Sieve)

**Files:**
- Modify: `dcl-web/static/index.html`

- [ ] **Step 1: Add Evolve panel (centerpiece)**

Panel with:
- Graph preset dropdown (P₅, C₆, W₅, K₄) — calls `/api/graph-presets` to load
- Initial labels display (editable)
- Exponent m input (default 2)
- Modulus input (optional, suggest 65537)
- Steps slider (1-50, default 5)
- "Evolve ▶" button
- Results: step-by-step table showing t, all vertex labels, coprime status
- Labels in scientific notation when > 10^9
- Play/Pause animation button that highlights one row at a time (1s interval)
- Timeline scrubber (range input) to jump to any step

JS: call `POST /api/evolve`, render animated table. The animation uses `setInterval` to advance a `currentStep` counter and highlight the corresponding row.

- [ ] **Step 2: Add Coprime Repair panel**

Panel with:
- "Load broken example" button that pre-fills labels [4, 6, 9, 10] on P₄ (intentionally non-coprime: gcd(4,6)=2)
- Manual label + edge input
- "Repair ▶" button
- Results: side-by-side original vs repaired labels
- Changes highlighted: "v₃: 6 → 7 (was gcd(4,6)=2, now gcd(4,7)=1)"
- Educational message explaining the repair algorithm

JS: call `POST /api/repair`, render diff.

- [ ] **Step 3: Add Prime Sieve panel**

Panel with:
- Count input (how many primes to find, default 20)
- Start value input (optional, default 2)
- "Find Primes ▶" button
- Results: grid of prime numbers, count found, time taken

JS: call `POST /api/prime-sieve`, render grid.

- [ ] **Step 4: Commit**

```bash
git add dcl-web/static/index.html
git commit -m "feat(dcl-web): add Evolve, Repair, and Sieve panels"
```

---

## Task 10: Frontend — Graph Workspace (SVG Editor + Presets)

**Files:**
- Modify: `dcl-web/static/index.html`

- [ ] **Step 1: Add Graph Workspace section**

`<section id="graph-workspace">` with:
- Preset buttons: P₅, C₆, W₅, K₄, Q₃ with n slider (range 3-20)
- SVG canvas (600×400) for graph display
- Instructions: "Click to add vertex, click two vertices to connect, double-click to edit label, right-click to delete"
- "Clear" button to reset graph
- "Use in Playground →" button

- [ ] **Step 2: Implement SVG graph rendering**

JS functions:
- `renderGraph(graph, svgElement)` — draws vertices as circles, edges as lines, labels as text inside circles
- `layoutGraph(graph)` — simple force-directed layout (spring simulation, ~50 lines):
  - Repulsive force between all vertex pairs (Coulomb's law)
  - Attractive force along edges (Hooke's law)
  - 100 iterations, damped
  - Vertices constrained to SVG bounds with 30px padding

Color logic:
- Default vertex fill: `#3b82f6`
- Edge stroke: compute gcd of endpoint labels; green if coprime, red dashed if not
- Selected vertex: amber `#f59e0b` stroke

- [ ] **Step 3: Implement editor interactions**

JS event handlers on the SVG:
- `click` on empty space → add vertex at click position, label = next prime
- `click` on vertex → select it (first click) or add/remove edge (second click on different vertex)
- `dblclick` on vertex → show input overlay to edit label
- `contextmenu` on vertex → remove vertex and its edges
- `mousedown` + `mousemove` on vertex → drag to reposition
- `mouseup` → stop dragging

Global state: `window.currentGraph = { vertices: [...], edges: [...] }`

- [ ] **Step 4: Wire presets to API**

When user clicks a preset button (e.g., "P₅"):
- Call `GET /api/graph-presets?family=path&n=5`
- Update `window.currentGraph` with response
- Call `layoutGraph()` + `renderGraph()`

- [ ] **Step 5: Commit**

```bash
git add dcl-web/static/index.html
git commit -m "feat(dcl-web): add SVG graph workspace with editor and presets"
```

---

## Task 11: Frontend — Benchmark Dashboard

**Files:**
- Modify: `dcl-web/static/index.html`

- [ ] **Step 1: Add Benchmark Dashboard section**

`<section id="benchmarks">` with four sub-panels:

**GPU vs CPU Scaling Chart:**
- Canvas element for bar chart
- JS: fetch `/api/benchmarks`, draw grouped bar chart (GPU blue, CPU gray) for each operation at each N
- X-axis: problem size, Y-axis: time (ms), grouped by operation
- Speedup annotations on GPU bars

**Kernel Throughput Chart:**
- Horizontal bar chart showing peak throughput per kernel
- Color-coded by kernel type

**NIST Results Table:**
- HTML table with test name + PASS/FAIL badge
- Summary: "14/15 PASS (93%)"

**Brute-Force Results Table:**
- HTML table with graph, vertices, threads, time, throughput, found count

All charts drawn on `<canvas>` elements using vanilla JS (no Chart.js):
- Simple `ctx.fillRect()` bar charts with axis labels
- ~80 lines per chart function

- [ ] **Step 2: Load benchmarks on page load**

In `initApp()`, fetch `/api/benchmarks` and call render functions for all 4 panels.

- [ ] **Step 3: Commit**

```bash
git add dcl-web/static/index.html
git commit -m "feat(dcl-web): add benchmark dashboard with charts and tables"
```

---

## Task 12: Final Integration + Polish

**Files:**
- Modify: `dcl-web/src/main.rs` (CORS, graceful startup message)
- Modify: `dcl-web/static/index.html` (footer, error states, responsiveness)

- [ ] **Step 1: Add CORS middleware**

In `main.rs`, add tower-http CORS layer for development convenience:

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

let app = Router::new()
    .route("/", get(index))
    .merge(api::api_router(backend))
    .merge(benchmark_data::router())
    .layer(cors);
```

- [ ] **Step 2: Add footer to HTML**

Footer with:
- "DCL CUDA Explorer — Built with Rust + CUDA + Axum"
- Link to DCL-RS repository
- Paper citation text

- [ ] **Step 3: Add offline/error states**

JS: if `/api/health` fetch fails, show a banner:
> "⚠️ Server offline — interactive features unavailable. Start with: `cargo run -p dcl-web`"

Disable all "Run" buttons but keep Learn section and Benchmark dashboard (pre-computed data embedded in HTML as fallback `<script>` block).

- [ ] **Step 4: Verify full application**

Run: `cargo run -p dcl-web`
Manual checks:
- Page loads at http://localhost:3000
- GPU status indicator shows correct state
- Each of the 6 interactive panels works with default inputs
- Graph editor: add/remove vertices and edges
- Presets load correctly
- Benchmark charts render
- Learn section cards expand/collapse

- [ ] **Step 5: Commit**

```bash
git add dcl-web/
git commit -m "feat(dcl-web): final integration, CORS, error states, footer"
```

---

## Summary

| Task | Description | Est. Files |
|------|-------------|-----------|
| 1 | Scaffold crate | 3 |
| 2 | GPU backend abstraction | 1 |
| 3 | Health + GCD endpoints | 3 |
| 4 | Remaining 4 API endpoints | 5 |
| 5 | Benchmark data | 1 |
| 6 | Graph presets endpoint | 1 |
| 7 | HTML shell + Learn section | 1 |
| 8 | Playground panels (GCD, PM, Coprime) | 1 |
| 9 | Playground panels (Evolve, Repair, Sieve) | 1 |
| 10 | Graph workspace (SVG editor) | 1 |
| 11 | Benchmark dashboard | 1 |
| 12 | Final integration + polish | 2 |
