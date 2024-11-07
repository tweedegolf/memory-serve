# Memory serve

**memory-serve** enables fast static file serving for axum web applications,
by keeping all assets in memory.

It loads static web assets like HTML, stylesheets, images and
scripts into the rust binary at compile time and exposes them as an
[axum](https://github.com/tokio-rs/axum) Router. It automatically adds cache
headers and handles file compression.

During development (debug builds) files are served dynamically,
they are read and compressed at request time.

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

There are two mechanisms to include assets at compile time.

1. Specify the path using a enviroment variable `ASSET_PATH` and call: `MemoryServe::from_env()` (best-practice)
2. Call the `load_assets!` macro, and pass this to the constructor: `MemoryServe::new(load_assets!("/foo/bar"))`

The environment variable is handled by a build script and instructs cargo to re-evaluate when an asset in the directory changes.
The output of the macro might be cached between build.

Both options try to be smart in resolving absolute and relative paths.

When an instance of `MemoryServe` is created, we can bind these to your axum instance.
Calling [`MemoryServe::into_router()`] on the `MemoryServe` instance produces an axum
[`Router`](https://docs.rs/axum/latest/axum/routing/struct.Router.html) that
can either be merged in another `Router` or used directly in a server by
calling [`Router::into_make_service()`](https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.into_make_service).

### Named directories

Multiple directories can be included using different environment variables, all prefixed by `ASSET_PATH_`.
For example: if you specify `ASSET_PATH_FOO` and `ASSET_PATH_BAR` the memory serve instances can be loaded
using `MemoryServe::from_env_name("FOO")` and `MemoryServe::from_env_name("BAR")` respectively.

### Features

Use the `force-embed` feature flag to always include assets in the binary - also in debug builds.

### Environment variables

Use `MEMORY_SERVE_ROOT` to specify a root directory for relative paths provided to the `load_assets!` macro (or th `ASSET_PATH` variable).

Uee `MEMORY_SERVE_QUIET=1` to not print log messages at compile time.

## Example

```rust,no_run
use axum::{response::Html, routing::get, Router};
use memory_serve::{MemoryServe, load_assets};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let memory_router = MemoryServe::new(load_assets!("../static"))
        .index_file(Some("/index.html"))
        .into_router();

    // possible other routes can be added at this point, like API routes
    let app = Router::new()
        .merge(memory_router);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Configuration options

An instance of the `MemoryServe` struct can be configured by calling
the following configuration methods:

| method                                   | Default value           | Description                                                |
| ---------------------------------------- | ----------------------- | ---------------------------------------------------------- |
| [`MemoryServe::index_file`]              | `Some("/index.html")`   | Which file to serve on the route "/"                       |
| [`MemoryServe::index_on_subdirectories`] | `false`                 | Whether to serve the corresponding index in subdirectories |
| [`MemoryServe::fallback`]                | `None`                  | Which file to serve if no routed matched the request       |
| [`MemoryServe::fallback_status`]         | `StatusCode::NOT_FOUND` | The HTTP status code to routes that did not match          |
| [`MemoryServe::enable_gzip`]             | `true`                  | Allow to serve gzip encoded files                          |
| [`MemoryServe::enable_brotli`]           | `true`                  | Allow to serve brotli encoded files                        |
| [`MemoryServe::html_cache_control`]      | `CacheControl::Short`   | Cache control header to serve on HTML files                |
| [`MemoryServe::cache_control`]           | `CacheControl::Medium`  | Cache control header to serve on other files               |
| [`MemoryServe::add_alias`]               | `[]`                    | Create a route / file alias                                |
| [`MemoryServe::enable_clean_url`]        | `false`                 | Enable clean URLs                                          |

See [`Cache control`](#cache-control) for the cache control options.

## Logging

During compilation, problems that occur with the inclusion or compression
of assets are logged to stdout, for instance:

```txt
WARN skipping file "static/empty.txt": file empty
```

When running the resulting executable, all registered routes and asset
sizes are logged using the [`tracing`](https://docs.rs/tracing/latest/tracing/)
crate. To print or log them, use [`tracing-subscriber`](https://docs.rs/tracing/latest/tracing_subscriber/).
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

| Option                    | Description                                                                         | Value                                          |
| ------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------- |
| [`CacheControl::Long`]    | clients can keep assets that have cache busting for a year                          | `max-age=31536000, immutable`                  |
| [`CacheControl::Medium`]  | assets without cache busting are revalidated after a day and can be kept for a week | `max-age=604800, stale-while-revalidate=86400` |
| [`CacheControl::Short`]   | cache kept for max 5 minutes, only at the client (not in a proxy)                   | `max-age:300, private`                         |
| [`CacheControl::NoCache`] | do not cache if freshness is really vital                                           | `no-cache`                                     |
| [`CacheControl::Custom`]  | Custom value                                                                        | _user defined_                                 |
