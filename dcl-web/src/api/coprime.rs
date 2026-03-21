//! POST /api/coprime-check — check coprimality of graph edges.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use crate::gpu_backend::GcdResult;
use super::SharedBackend;

#[derive(Deserialize)]
struct CoprimeRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
}

#[derive(Serialize)]
struct CoprimeResponse {
    results: Vec<GcdResult>,
    all_coprime: bool,
    backend: String,
    time_us: u128,
}

async fn coprime_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<CoprimeRequest>,
) -> Result<Json<CoprimeResponse>, Json<serde_json::Value>> {
    if req.labels.is_empty() {
        return Err(Json(json!({
            "error": "labels must not be empty",
            "code": "INVALID_INPUT"
        })));
    }
    if req.edges.is_empty() {
        return Err(Json(json!({
            "error": "edges must not be empty",
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

    let edge_tuples: Vec<(usize, usize)> = req.edges.iter().map(|&[u, v]| (u, v)).collect();
    let a: Vec<u64> = edge_tuples.iter().map(|&(u, _)| req.labels[u]).collect();
    let b: Vec<u64> = edge_tuples.iter().map(|&(_, v)| req.labels[v]).collect();

    let start = Instant::now();
    let results = backend.batch_gcd(&a, &b);
    let time_us = start.elapsed().as_micros();

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
        .route("/api/coprime-check", post(coprime_handler))
        .with_state(backend)
}
