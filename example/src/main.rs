use axum::{Router, extract::State, response::Html, routing::get};
use memory_serve::Manifest;
use std::net::SocketAddr;
use tracing::{Level, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .init();

    let memory_serve = memory_serve::load!()
        // "/" is rendered by `handler` below instead of the static index.html
        .index_file(None)
        // also serve assets on a cache-busted route, e.g.
        // /assets/index.css -> /assets/index.ec4edeea111c8549.css
        .enable_hashed_routes(true);

    // maps plain routes to hashed routes, keep it in the axum state so
    // handlers can reference the cache-busted assets
    let manifest = memory_serve.manifest();

    let app = Router::new()
        .merge(memory_serve.into_router())
        .route("/", get(handler))
        .with_state(manifest);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

/// Render a page that links to the hashed routes of the static assets.
/// Static HTML files like /index.html are served as-is, the library does not
/// rewrite the asset references inside them.
async fn handler(State(manifest): State<Manifest>) -> Html<String> {
    let asset = |route: &str| manifest.get(route).unwrap_or(route).to_string();

    Html(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>Hello, World!</title>
    <script type="module" src="{js}"></script>
    <link rel="stylesheet" href="{css}">
    <link rel="icon" type="image/jpeg" href="{icon}">
  </head>
  <body>
    <h1>Hello, World!</h1>
  </body>
</html>"#,
        js = asset("/assets/index.js"),
        css = asset("/assets/index.css"),
        icon = asset("/assets/icon.jpg"),
    ))
}
