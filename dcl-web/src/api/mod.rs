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
