# Memory serve

**memory-serve** enables fast static file serving for axum web applications,
by keeping all assets in memory.

It loads static web assets like HTML, stylesheets, images and
scripts into the rust binary at compile time and exposes them as an
[axum](https://github.com/tokio-rs/axum) Router. It automatically adds cache
headers and handles file compression.

During development (debug builds) files are served dynamically,
they are read and compressed at request time.

In release mode text-based files like HTML or javascript
are compressed using [brotli](https://en.wikipedia.org/wiki/Brotli)
at compile time and decompressed at startup, to minimize the binary size.

All files are served with an
[etag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
header and
[If-None-Match](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match)
requests are handled accordingly.

Text-based files are served in plain or with gzip or brotli compression
based on the abilities and preferences of the client.

Assets can optionally be served on a
[cache-busted route](#cache-busting) containing a hash of their contents,
so that clients can cache them indefinitely.

Routing can be configured in a flexible manner, for instance to accommodate
an SPA.

## Compatibility

memory-serve is designed to work with [axum](https://github.com/tokio-rs/axum)

## Usage

Because memory-serve is used both from your `build.rs` (to load assets at
compile time) and from your application code (to serve them), it must be added
to **both** `[dependencies]` and `[build-dependencies]` in your `Cargo.toml`:

```toml
[dependencies]
memory-serve = "2"

[build-dependencies]
memory-serve = "2"
```

Omitting the `[build-dependencies]` entry results in a compile error from the
build script, like `error[E0433]: cannot find module or crate 'memory_serve'`.

Call `load_directory` with a relative path from your `build.rs`. See [the Cargo book](https://doc.rust-lang.org/cargo/reference/build-scripts.html) for more information about build scripts. See the example project in the memory-serve repository
for an example `build.rs`. Calling `load_directory` makes sure that your assets are loaded at compile time.

Call the `load!` macro from your application to create a `MemoryServe` instance.

When an instance of `MemoryServe` is created, we can bind it to your axum instance.
Calling [`MemoryServe::into_router()`] on the `MemoryServe` instance produces an axum
[`Router`](https://docs.rs/axum/latest/axum/routing/struct.Router.html) that
can either be merged in another `Router` or used directly in a server by
calling [`Router::into_make_service()`](https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.into_make_service).

### Named directories

Multiple directories can be included using `load_names_directories` from your `build.rs` script.
This takes a list of tuples, with the name and the path of your asset directories,
and a boolean indicating whether to embed the assets into the binary.

You can use the names as specified in the `load_names_directories` call to load the specific
`MemoryServe` instance by passing the name as a string to the `load!` macro.

### Features

Use the `force-embed` feature flag to always include assets in the binary - also in debug builds.

### Environment variables

These are read by the build script, so set them when running `cargo build`
(or `cargo test`, `cargo run`), for instance in your shell or in the `[env]`
section of `.cargo/config.toml`. They have no effect on the running program.

- `MEMORY_SERVE_QUIET=1` suppresses all messages at compile time.
- `MEMORY_SERVE_VERBOSE=1` also prints a message for every included file.

Any value other than empty, `0`, `false`, `no` or `off` enables a variable.
See [Logging](#logging) to filter the log messages of the running program.

## Example

`build.rs`:
```rust
fn main() {
    memory_serve::load_directory("./public");
}
```

`main.rs`:
```rust
use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let memory_router = memory_serve::load!()
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
| [`MemoryServe::fallback`]                | `None`                  | Which file to serve if no route matched the request        |
| [`MemoryServe::fallback_status`]         | `StatusCode::NOT_FOUND` | The HTTP status code to serve for routes that did not match |
| [`MemoryServe::enable_gzip`]             | `true` (release) [^1]   | Allow serving gzip encoded files                           |
| [`MemoryServe::enable_brotli`]           | `true` (release) [^1]   | Allow serving brotli encoded files                         |
| [`MemoryServe::html_cache_control`]      | `CacheControl::Short`   | Cache control header to serve on HTML files                |
| [`MemoryServe::cache_control`]           | `CacheControl::Medium`  | Cache control header to serve on other files               |
| [`MemoryServe::add_alias`]               | `[]`                    | Create a route / file alias                                |
| [`MemoryServe::enable_clean_url`]        | `false`                 | Enable clean URLs                                          |
| [`MemoryServe::enable_hashed_routes`]    | `false`                 | Also serve assets on a cache-busted, hashed route          |

See [`Cache control`](#cache-control) for the cache control options and
[`Cache busting`](#cache-busting) for hashed routes.

[^1]: Compression defaults to enabled in release builds and disabled in debug
builds (where assets are served dynamically).

## Logging

During compilation, the build script logs the asset directory, any skipped
files and a summary as warnings:

```txt
warning: memory-serve-test@0.0.0: Loading static assets from /home/user/project/my-project/static
warning: memory-serve-test@0.0.0: skipping file /empty.txt: file empty
warning: memory-serve-test@0.0.0: Embedding 7 assets into binary
```

With `MEMORY_SERVE_VERBOSE=1` every included file is logged as well, and
`MEMORY_SERVE_QUIET=1` suppresses all of these messages, see
[Environment variables](#environment-variables):

```txt
warning: memory-serve-test@0.0.0: including /blog/index.html 431 -> 175 bytes (compressed)
```

When running the resulting executable, a summary of the router is logged at
the `INFO` level using the [`tracing`](https://docs.rs/tracing/latest/tracing/)
crate. To print or log them, use [`tracing-subscriber`](https://docs.rs/tracing/latest/tracing_subscriber/).
Example output:

```txt
 INFO memory_serve: serving /index.html as index on /
 INFO memory_serve: serving 7 assets (4 on hashed routes)
```

Every registered route, asset size and hashed route is logged at the `DEBUG`
level:

```txt
DEBUG memory_serve: serving /assets/index.css 1552 -> 509 bytes (compressed)
DEBUG memory_serve: serving /assets/index.css on hashed route /assets/index.ec4edeea111c8549.css
```

All messages use the `memory_serve` target, so they can be filtered like any
other crate, for instance with `RUST_LOG=memory_serve=warn` when using
`tracing-subscriber`'s `EnvFilter`.

## Cache control

There are 5 different values to choose from for the cache-control settings:

| Option                    | Description                                                                         | Value                                          |
| ------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------- |
| [`CacheControl::Long`]    | clients can keep assets that have cache busting for a year                          | `max-age=31536000, immutable`                  |
| [`CacheControl::Medium`]  | assets without cache busting are revalidated after a day and can be kept for a week | `max-age=604800, stale-while-revalidate=86400` |
| [`CacheControl::Short`]   | cache kept for max 5 minutes, only at the client (not in a proxy)                   | `max-age=300, private`                         |
| [`CacheControl::NoCache`] | do not cache if freshness is really vital                                           | `no-cache`                                     |
| [`CacheControl::Custom`]  | Custom value                                                                        | _user defined_                                 |

## Cache busting

With [`MemoryServe::enable_hashed_routes`] every non-HTML asset is also served
on a route that contains a hash of its contents, for instance
`/assets/index.css` is also available as `/assets/index.3f9a1c2b7d84e6a0.css`.
The hashed route changes whenever the file changes, so embedded assets are
served there with [`CacheControl::Long`] (`immutable`). The plain route keeps
the configured cache control.

Use [`MemoryServe::manifest`] to look up the hashed routes, for instance to
reference them from your templates. HTML files always map to their plain
route, and when hashed routes are disabled all assets do, so templates work
regardless of the setting:

```rust
let memory_serve = memory_serve::load!().enable_hashed_routes(true);

// keep this around, for instance in your axum state
let manifest = memory_serve.manifest();
// Some("/assets/index.3f9a1c2b7d84e6a0.css")
let css = manifest.get("/assets/index.css");

let router = memory_serve.into_router();
```

In debug builds (dynamic serving) the hash is computed when the manifest or
router is created, and the hashed route is served with the regular cache
control so that edited files are not cached as immutable.
