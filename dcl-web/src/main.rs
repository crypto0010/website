mod api;
mod benchmark_data;
mod gpu_backend;

use axum::{Router, response::Html, routing::get};
use std::net::SocketAddr;
use std::sync::Arc;
use gpu_backend::DclBackend;

const INDEX_HTML: &str = include_str!("../static/index.html");

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[tokio::main]
async fn main() {
    let backend = Arc::new(DclBackend::new());

    let app = Router::new()
        .route("/", get(index))
        .merge(api::api_router(backend))
        .merge(benchmark_data::router());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🌐 DCL CUDA Explorer running at http://localhost:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
