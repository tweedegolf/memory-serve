# Memory serve

**memory-serve** enables fast static file serving for axum web applications,
by keeping all assets in memory.

It loads static web assets like HTML, stylesheets, images and
scripts into the rust binary at compile time and exposes them as an
[axum](https://github.com/tokio-rs/axum) Router. It automatically adds cache
headers and handles file compression.

Text-based files like HTML or javascript
are compressed using [brotli](https://en.wikipedia.org/wiki/Brotli)
at compile time and decompressed at startup, to minimize the binary size.

All files are served with an
[etag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
header and
[If-None-Match](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match)
requests are handled accordingly.

Text-based files are served in plain or with gzip or brotli compression
based on the abilities and preferences of the client.

Routing can be configured in a flexible manner, for instance to accommodate
an SPA.

## Compatibility

memory-serve is designed to work with [axum](https://github.com/tokio-rs/axum)

## Usage

Provide a relative path to the directory containing your static assets
to the `load_assets!` macro. This macro creates a data structure intended to
be consumed by `MemoryServe::new`. Calling `MemoryServe::into_router()` on
the resulting instance produces a axum
[Router](https://docs.rs/axum/latest/axum/routing/struct.Router.html) that
can either be merged in another `Router` or used directly in a server by
calling `Router::into_make_service()`.

## Example

```rust
use axum::{response::Html, routing::get, Router};
use memory_serve::{load_assets, MemoryServe};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let memory_router = MemoryServe::new(load_assets!("static"))
        .index_file(Some("/index.html"))
        .into_router();

    // possible other routes an be added at this point, like API routes
    let app = Router::new()
        .merge(memory_router);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Configuration options

An instance of the `MemoryServe` struct can be configured by calling
the following configuration methods:

| method               | Default value           | Description                                           |
|----------------------|-------------------------|-------------------------------------------------------|
| `index_file`         | `Some("index.html")`    | Which file to serve on the route "/"                  |
| `fallback`           | `None`                  | Which file to serve if no routed matched the request  |
| `fallback_status`    | `StatusCode::NOT_FOUND` | The HTTP status code to routes that did not match     |
| `enable_gzip`        | `true`                  | Allow to serve gzip encoded files                     |
| `enable_brotli`      | `true`                  | Allow to serve brotli encoded files                   |
| `html_cache_control` | `CacheConrol::Short`    | Cache control header to serve on HTML files           |
| `cache_control`      | `CacheConrol::Medium`   | Cache control header to serve on other files          |

See `Cache control` for the cache control options.

## Logging

During compilation, problems that occur with the inclusion or compression
of assets are logged to stdout, for instance:

```txt
WARN skipping file "static/empty.txt": file empty
```

When running the resulting executable, all registered routes and asset
sizes are logged using the [tracing](https://github.com/tokio-rs/tracing)
crate. To print or log them, use `tracing-subscriber`.
Example output:

```txt
 INFO memory_serve: serving /assets/icon.jpg 1366 bytes
 INFO memory_serve: serving /assets/index.css 1552 bytes
 INFO memory_serve: serving /assets/index.css (brotli compressed) 509 bytes
 INFO memory_serve: serving /assets/index.css (gzip compressed) 624 bytes
 INFO memory_serve: serving /assets/index.js 20 bytes
 INFO memory_serve: serving /assets/stars.svg 2255 bytes
 INFO memory_serve: serving /assets/stars.svg (brotli compressed) 907 bytes
 INFO memory_serve: serving /assets/stars.svg (gzip compressed) 1048 bytes
 INFO memory_serve: serving /index.html 437 bytes
 INFO memory_serve: serving /index.html (brotli compressed) 178 bytes
 INFO memory_serve: serving /index.html (gzip compressed) 274 bytes
 INFO memory_serve: serving /index.html as index on /
```

## Cache control

There are 5 different values to choose from for the cache-control settings:

| Option                | Description                                                                                | Value                                          |
|-----------------------|--------------------------------------------------------------------------------------------|------------------------------------------------|
| CacheControl::Long    | clients can keep assets that have cache busting for a year                                 | `max-age=31536000, immutable`                  |
| CacheControl::Medium  | assets without cache busting are revalidated after a day and can be kept for a week        | `max-age=604800, stale-while-revalidate=86400` |
| CacheControl::Short   | cache kept for max 5 minutes, only at the client (not in a proxy)                          | `max-age:300, private`                         |
| CacheControl::NoCache | do not cache if freshness is really vital                                                  | `no-cache`                                     |
| CacheControl::Custom  | Custom value                                                                               | *user defined*                                 |
