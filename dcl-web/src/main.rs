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
