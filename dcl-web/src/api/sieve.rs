//! POST /api/prime-sieve — find safe primes starting from an optional offset.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use super::SharedBackend;

#[derive(Deserialize)]
struct SieveRequest {
    start: Option<u64>,
    count: usize,
}

#[derive(Serialize)]
struct SieveResponse {
    safe_primes: Vec<u64>,
    count: usize,
    backend: String,
    time_us: u128,
}

async fn sieve_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<SieveRequest>,
) -> Result<Json<SieveResponse>, Json<serde_json::Value>> {
    if req.count == 0 || req.count > 1000 {
        return Err(Json(json!({
            "error": "count must be between 1 and 1000",
            "code": "INVALID_INPUT"
        })));
    }

    let start_val = req.start.unwrap_or(5);

    let t0 = Instant::now();
    let safe_primes = backend.prime_sieve(start_val, req.count);
    let time_us = t0.elapsed().as_micros();

    let count = safe_primes.len();

    Ok(Json(SieveResponse {
        safe_primes,
        count,
        backend: backend.backend_name().to_string(),
        time_us,
    }))
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/prime-sieve", post(sieve_handler))
        .with_state(backend)
}
