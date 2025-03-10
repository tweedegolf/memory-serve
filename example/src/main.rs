use axum::{Router, response::Html, routing::get};
use memory_serve::{MemoryServe, load_assets};
use std::net::SocketAddr;
use tracing::{Level, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let memory_router = MemoryServe::new(load_assets!("../static"))
        .index_file(Some("/index.html"))
        .into_router();

    let app = Router::new()
        .merge(memory_router)
        .route("/hello", get(handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
