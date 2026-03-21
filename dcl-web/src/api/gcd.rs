//! POST /api/gcd — batch GCD computation.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use crate::gpu_backend::GcdResult;
use super::SharedBackend;

#[derive(Deserialize)]
struct GcdRequest {
    pairs: Vec<[u64; 2]>,
}

#[derive(Serialize)]
struct GcdResponse {
    results: Vec<GcdResult>,
    backend: String,
    time_us: u128,
}

async fn gcd_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<GcdRequest>,
) -> Result<Json<GcdResponse>, Json<serde_json::Value>> {
    if req.pairs.is_empty() {
        return Err(Json(json!({
            "error": "pairs must not be empty",
            "code": "INVALID_INPUT"
        })));
    }
    if req.pairs.len() > 10_000 {
        return Err(Json(json!({
            "error": "pairs exceeds maximum of 10,000",
            "code": "INVALID_INPUT"
        })));
    }

    let a: Vec<u64> = req.pairs.iter().map(|p| p[0]).collect();
    let b: Vec<u64> = req.pairs.iter().map(|p| p[1]).collect();

    let start = Instant::now();
    let results = backend.batch_gcd(&a, &b);
    let time_us = start.elapsed().as_micros();

    Ok(Json(GcdResponse {
        results,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/gcd", post(gcd_handler))
        .with_state(backend)
}
