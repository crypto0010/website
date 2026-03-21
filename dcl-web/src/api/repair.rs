//! POST /api/repair — repair non-coprime graph edges by relabelling.

use axum::{Router, routing::post, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use super::SharedBackend;

#[derive(Deserialize)]
struct RepairRequest {
    labels: Vec<u64>,
    edges: Vec<[usize; 2]>,
}

#[derive(Serialize)]
struct RepairChange {
    edge: [usize; 2],
    old_label: u64,
    new_label: u64,
    vertex: usize,
}

#[derive(Serialize)]
struct RepairResponse {
    original: Vec<u64>,
    repaired: Vec<u64>,
    changes: Vec<RepairChange>,
    backend: String,
    time_us: u128,
}

async fn repair_handler(
    State(backend): State<SharedBackend>,
    Json(req): Json<RepairRequest>,
) -> Result<Json<RepairResponse>, Json<serde_json::Value>> {
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

    let start = Instant::now();
    let (repaired, repaired_edge_tuples) = backend.coprime_repair(&req.labels, &edge_tuples);
    let time_us = start.elapsed().as_micros();

    // Build change records: for each repaired edge, determine which vertex changed.
    let changes: Vec<RepairChange> = repaired_edge_tuples
        .iter()
        .map(|&(u, v)| {
            // The repair logic increments the smaller endpoint.
            let vertex = if req.labels[u] <= req.labels[v] { u } else { v };
            RepairChange {
                edge: [u, v],
                old_label: req.labels[vertex],
                new_label: repaired[vertex],
                vertex,
            }
        })
        .collect();

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
        .route("/api/repair", post(repair_handler))
        .with_state(backend)
}
