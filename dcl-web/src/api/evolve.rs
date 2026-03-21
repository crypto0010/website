//! POST /api/evolve — multi-step power-map evolution of a labelled graph.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use dcl_core::gcd::are_coprime;
use super::SharedBackend;

#[derive(Deserialize)]
struct EvolveRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
    exponent: u32,
    modulus: Option<u64>,
    steps: usize,
}

#[derive(Serialize)]
struct StepSnapshot {
    t: usize,
    labels: Vec<u64>,
    coprime: Vec<bool>,
}

#[derive(Serialize)]
struct EvolveResponse {
    steps: Vec<StepSnapshot>,
    backend: String,
    time_us: u128,
}

async fn evolve_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<EvolveRequest>,
) -> Result<Json<EvolveResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() {
        return Err(Json(json!({
            "error": "labels must not be empty",
            "code": "INVALID_INPUT"
        })));
    }
    if req.labels.iter().any(|&l| l == 0) {
        return Err(Json(json!({
            "error": "all labels must be greater than 0",
            "code": "INVALID_INPUT"
        })));
    }
    if req.exponent < 2 {
        return Err(Json(json!({
            "error": "exponent must be at least 2",
            "code": "INVALID_INPUT"
        })));
    }
    if req.steps == 0 || req.steps > 50 {
        return Err(Json(json!({
            "error": "steps must be between 1 and 50",
            "code": "INVALID_INPUT"
        })));
    }
    if req.labels.len() > 100 {
        return Err(Json(json!({
            "error": "number of vertices must not exceed 100",
            "code": "INVALID_INPUT"
        })));
    }

    let n = req.labels.len();
    for (i, &[u, v]) in req.edges.iter().enumerate() {
        if u >= n || v >= n {
            return Err(Json(json!({
                "error": format!("edge[{}] vertex out of bounds (labels has {} vertices)", i, n),
                "code": "INVALID_INPUT"
            })));
        }
    }

    let start = Instant::now();

    // Compute coprime flags for a given label set.
    let coprime_flags = |labels: &[u64]| -> Vec<bool> {
        req.edges
            .iter()
            .map(|&[u, v]| are_coprime(labels[u], labels[v]))
            .collect()
    };

    // Step 0: initial state.
    let mut snapshots = Vec::with_capacity(req.steps + 1);
    snapshots.push(StepSnapshot {
        t: 0,
        labels: req.labels.clone(),
        coprime: coprime_flags(&req.labels),
    });

    // Steps 1..=N: apply power map one step at a time.
    let mut current = req.labels.clone();
    for t in 1..=req.steps {
        current = backend.power_map(&current, req.exponent, req.modulus);
        let coprime = coprime_flags(&current);
        snapshots.push(StepSnapshot {
            t,
            labels: current.clone(),
            coprime,
        });
    }

    let time_us = start.elapsed().as_micros();

    Ok(Json(EvolveResponse {
        steps: snapshots,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/evolve", post(evolve_handler))
        .with_state(backend)
}
