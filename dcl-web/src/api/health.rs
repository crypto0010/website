//! GET /api/health — backend status endpoint.

use axum::{Router, routing::get, extract::State, Json};
use serde::Serialize;
use super::SharedBackend;

#[derive(Serialize)]
struct HealthResponse {
    gpu_available: bool,
    device_name: String,
    backend: String,
}

async fn health_handler(State(backend): State<SharedBackend>) -> Json<HealthResponse> {
    let info = backend.device_info();
    Json(HealthResponse {
        gpu_available: info.gpu_available,
        device_name: info.device_name,
        backend: info.backend,
    })
}

pub fn router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
        .with_state(backend)
}
