use axum::{response::Html, routing::get, Router};
use memory_serve::{load_assets, MemoryServe};
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let memory_router = MemoryServe::new(load_assets!("../../static"))
        .index_file(Some("/index.html"))
        .into_router();

    let app = Router::new()
        .merge(memory_router)
        .route("/hello", get(handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
