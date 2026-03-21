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

fn initial_labels(n: usize) -> Vec<u64> {
    let primes = [2u64,3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,
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
        "cycle" => {
            if params.n < 3 {
                return Err(Json(serde_json::json!({
                    "error": "Cycle graphs require n >= 3",
                    "code": "INVALID_INPUT"
                })));
            }
            Graph::cycle(params.n)
        }
        "wheel" => {
            if params.n < 4 {
                return Err(Json(serde_json::json!({
                    "error": "Wheel graphs require n >= 4 (hub + 3-cycle)",
                    "code": "INVALID_INPUT"
                })));
            }
            Graph::wheel(params.n)
        }
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

    let n_vertices = graph.n;
    let edges: Vec<[usize; 2]> = graph.edges().iter().map(|&(u, v)| [u, v]).collect();
    let labels = initial_labels(n_vertices);

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
