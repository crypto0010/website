//! POST /api/power-map — apply x^m (mod n) to a list of labels.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use super::SharedBackend;

#[derive(Deserialize)]
struct PowerMapRequest {
    labels: Vec<u64>,
    exponent: u32,
    modulus: Option<u64>,
}

#[derive(Serialize)]
struct PowerMapResponse {
    original: Vec<u64>,
    result: Vec<u64>,
    exponent: u32,
    modulus: Option<u64>,
    backend: String,
    time_us: u128,
}

async fn power_map_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<PowerMapRequest>,
) -> Result<Json<PowerMapResponse>, Json<serde_json::Value>> {
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

    let start = Instant::now();
    let result = backend.power_map(&req.labels, req.exponent, req.modulus);
    let time_us = start.elapsed().as_micros();

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
        .route("/api/power-map", post(power_map_handler))
        .with_state(backend)
}
