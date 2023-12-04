//! [memory-serve] enables fast static file serving for axum web applications,
//! by keeping all assets in memory.
//!
//! It loads static web assets like HTML, stylesheets, images and
//! scripts into the rust binary at compile time and exposes them as an
//! [axum](https://github.com/tokio-rs/axum) Router. It automatically adds cache
//! headers and handles file compression.
//!
//! During development (debug builds) files are served dynamically,
//! they are read and compressed at request time.
//!
//! Text-based files like HTML or javascript
//! are compressed using [brotli](https://en.wikipedia.org/wiki/Brotli)
//! at compile time and decompressed at startup, to minimize the binary size.
//!
//! All files are served with an
//! [etag](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)
//! header and
//! [If-None-Match](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match)
//! requests are handled accordingly.
//!
//! Text based files are served as-is or with gzip or brotli compression
//! based on the abilities and preferences of the client.
//!
//! Routing can be configured in a flexible manner, for instance to accommodate
//! an SPA.
//!
//! # Compatibility
//!
//! memory-serve is designed to work with [axum](https://github.com/tokio-rs/axum)
//!
//! # Usage
//!
//! Provide a relative path to the directory containing your static assets
//! to the `load_assets!` macro. This macro creates a data structure intended to
//! be consumed by [MemoryServe::new]. Calling [MemoryServe::into_router()] on
//! the resulting instance produces a axum
//! [Router](https://docs.rs/axum/latest/axum/routing/struct.Router.html) that
//! can either be merged in another `Router` or used directly in a server by
//! calling `Router::into_make_service()`.
//!
//! # Example
//!
//! ```no_run
//! use axum::{response::Html, routing::get, Router};
//! use memory_serve::{load_assets, MemoryServe};
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() {
//!     let memory_router = MemoryServe::new(load_assets!("../static"))
//!         .index_file(Some("/index.html"))
//!         .into_router();
//!
//!     // possible other routes an be added at this point, like API routes
//!     let app = Router::new()
//!         .merge(memory_router);
//!
//!     let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//!     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! # Configuration options
//!
//! An instance of the `MemoryServe` struct can be configured by calling
//! the following configuration methods:
//!
//! | method                            | Default value           | Description                                           |
//! |-----------------------------------|-------------------------|-------------------------------------------------------|
//! | [MemoryServe::index_file]         | `Some("/index.html")`   | Which file to serve on the route "/"                  |
//! | [MemoryServe::fallback]           | `None`                  | Which file to serve if no routed matched the request  |
//! | [MemoryServe::fallback_status]    | `StatusCode::NOT_FOUND` | The HTTP status code to routes that did not match     |
//! | [MemoryServe::enable_gzip]        | `true`                  | Allow to serve gzip encoded files                     |
//! | [MemoryServe::enable_brotli]      | `true`                  | Allow to serve brotli encoded files                   |
//! | [MemoryServe::html_cache_control] | `CacheConrol::Short`    | Cache control header to serve on HTML files           |
//! | [MemoryServe::cache_control]      | `CacheConrol::Medium`   | Cache control header to serve on other files          |
//! | [MemoryServe::add_alias]          | `[]`                    | Create a route / file alias                           |
//!
//! See [Cache control](index.html#cache-control) for the cache control options.
//!
//! # Logging
//!
//! During compilation, problems that occur with the inclusion or compression
//! of assets are logged to stdout, for instance:
//!
//! ```txt,no_run
//! WARN skipping file "static/empty.txt": file empty
//! ```
//!
//! When running the resulting executable, all registered routes and asset
//! sizes are logged using the [tracing](https://github.com/tokio-rs/tracing)
//! crate. To print or log them, use `tracing-subscriber`.
//! Example output:
//!
//! ```txt,no_run
//!  INFO memory_serve: serving /assets/icon.jpg 1366 bytes
//!  INFO memory_serve: serving /assets/index.css 1552 bytes
//!  INFO memory_serve: serving /assets/index.css (brotli compressed) 509 bytes
//!  INFO memory_serve: serving /assets/index.css (gzip compressed) 624 bytes
//!  INFO memory_serve: serving /assets/index.js 20 bytes
//!  INFO memory_serve: serving /assets/stars.svg 2255 bytes
//!  INFO memory_serve: serving /assets/stars.svg (brotli compressed) 907 bytes
//!  INFO memory_serve: serving /assets/stars.svg (gzip compressed) 1048 bytes
//!  INFO memory_serve: serving /index.html 437 bytes
//!  INFO memory_serve: serving /index.html (brotli compressed) 178 bytes
//!  INFO memory_serve: serving /index.html (gzip compressed) 274 bytes
//!  INFO memory_serve: serving /index.html as index on /
//! ```
//!
//! # Cache control
//!
//! There are 5 different values to choose from for the cache-control settings:
//!
//! | Option                  | Description                                                                                | Value                                          |
//! |-------------------------|--------------------------------------------------------------------------------------------|------------------------------------------------|
//! | [CacheControl::Long]    | clients can keep assets that have cache busting for a year                                 | `max-age=31536000, immutable`                  |
//! | [CacheControl::Medium]  | assets without cache busting are revalidated after a day and can be kept for a week        | `max-age=604800, stale-while-revalidate=86400` |
//! | [CacheControl::Short]   | cache kept for max 5 minutes, only at the client (not in a proxy)                          | `max-age:300, private`                         |
//! | [CacheControl::NoCache] | do not cache if freshness is really vital                                                  | `no-cache`                                     |
//! | [CacheControl::Custom]  | Custom value                                                                               | *user defined*                                 |

use axum::{
    http::{HeaderMap, StatusCode},
    routing::get,
};
use std::future::ready;
use tracing::info;

mod asset;
mod cache_control;
mod util;

use crate::util::{compress_gzip, decompress_brotli};

pub use crate::asset::Asset;
pub use crate::cache_control::CacheControl;

/// Macro to load a directory of static files into the resulting binary
/// (possibly compressed) and create a data structure of (meta)data
/// as an input for [MemoryServe::new]
pub use memory_serve_macros::load_assets;

#[derive(Debug, Clone, Copy)]
struct ServeOptions {
    index_file: Option<&'static str>,
    fallback: Option<&'static str>,
    fallback_status: StatusCode,
    html_cache_control: CacheControl,
    cache_control: CacheControl,
    enable_brotli: bool,
    enable_gzip: bool,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            index_file: Some("/index.html"),
            fallback: None,
            fallback_status: StatusCode::NOT_FOUND,
            html_cache_control: CacheControl::Short,
            cache_control: CacheControl::Medium,
            enable_brotli: true,
            enable_gzip: true,
        }
    }
}

/// Helper struct to create and configure an axum to serve static files from
/// memory.
/// Initiate an instance with the `MemoryServe::new` method and pass a call
/// to  the `load_assets!` macro as the single argument.
#[derive(Debug, Default)]
pub struct MemoryServe {
    options: ServeOptions,
    assets: &'static [Asset],
    aliases: Vec<(&'static str, &'static str)>,
}

impl MemoryServe {
    /// Initiate a `MemoryServe` instance, takes the output of `load_assets!`
    /// as an argument. `load_assets!` takes a directory name relative from
    /// the project root.
    pub fn new(assets: &'static [Asset]) -> Self {
        Self {
            assets,
            ..Default::default()
        }
    }

    /// Which static file to serve on the route "/" (the index)
    /// The path (or route) should be relative to the directory passed to
    /// the `load_assets!` macro, but prepended with a slash.
    /// By default this is `Some("/index.html")`
    pub fn index_file(mut self, index_file: Option<&'static str>) -> Self {
        self.options.index_file = index_file;

        self
    }

    /// Which static file to serve when no other routes are matched, also see
    /// [fallback](https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.fallback)
    /// The path (or route) should be relative to the directory passed to
    /// the `load_assets!` macro, but prepended with a slash.
    /// By default this is `None`, which means axum will return an empty
    /// response with a HTTP 404 status code when no route matches.
    pub fn fallback(mut self, fallback: Option<&'static str>) -> Self {
        self.options.fallback = fallback;

        self
    }

    /// What HTTP status code to return when a static file is returned by the
    /// fallback handler.
    pub fn fallback_status(mut self, fallback_status: StatusCode) -> Self {
        self.options.fallback_status = fallback_status;

        self
    }

    /// Whether to enable gzip compression. When set to `true`, clients that
    /// accept gzip compressed files, but not brotli compressed files,
    /// are served gzip compressed files.
    pub fn enable_gzip(mut self, enable_gzip: bool) -> Self {
        self.options.enable_gzip = enable_gzip;

        self
    }

    /// Whether to enable brotli compression. When set to `true`, clients that
    /// accept brotli compressed files are served brotli compressed files.
    pub fn enable_brotli(mut self, enable_brotli: bool) -> Self {
        self.options.enable_brotli = enable_brotli;

        self
    }

    /// The Cache-Control header to set for HTML files.
    /// See [Cache control](index.html#cache-control) for options.
    pub fn html_cache_control(mut self, html_cache_control: CacheControl) -> Self {
        self.options.html_cache_control = html_cache_control;

        self
    }

    /// Cache header to non-HTML files.
    /// See [Cache control](index.html#cache-control) for options.
    pub fn cache_control(mut self, cache_control: CacheControl) -> Self {
        self.options.cache_control = cache_control;

        self
    }

    /// Create an alias for a route / file
    pub fn add_alias(mut self, from: &'static str, to: &'static str) -> Self {
        self.aliases.push((from, to));

        self
    }

    /// Create an axum `Router` instance that will serve the included static assets
    /// Caution! This method leaks memory. It should only be called once (at startup).
    pub fn into_router<S>(self) -> axum::Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        let mut router = axum::Router::new();
        let options = Box::leak(Box::new(self.options));

        for asset in self.assets {
            let bytes = if asset.bytes.is_empty() && !asset.brotli_bytes.is_empty() {
                Box::new(decompress_brotli(asset.brotli_bytes).unwrap_or_default()).leak()
            } else {
                asset.bytes
            };

            let gzip_bytes = if !asset.brotli_bytes.is_empty() && options.enable_gzip {
                Box::new(compress_gzip(bytes).unwrap_or_default()).leak()
            } else {
                Default::default()
            };

            if !bytes.is_empty() {
                if !asset.brotli_bytes.is_empty() {
                    info!(
                        "serving {} {} -> {} bytes (compressed)",
                        asset.route,
                        bytes.len(),
                        asset.brotli_bytes.is_empty()
                    );
                } else {
                    info!("serving {} {} bytes", asset.route, bytes.len());
                }
            }

            let handler = |headers: HeaderMap| {
                ready(asset.handler(&headers, StatusCode::OK, bytes, gzip_bytes, options))
            };

            if Some(asset.route) == options.fallback {
                info!("serving {} as fallback", asset.route);

                router = router.fallback(|headers: HeaderMap| {
                    ready(asset.handler(
                        &headers,
                        options.fallback_status,
                        bytes,
                        gzip_bytes,
                        options,
                    ))
                });
            }

            if Some(asset.route) == options.index_file {
                info!("serving {} as index on /", asset.route);

                router = router.route("/", get(handler));
            }

            router = router.route(asset.route, get(handler));

            // add all aliases that point to the asset route
            for (from, to) in self.aliases.iter() {
                if *to == asset.route {
                    info!("serving {} as index on {}", asset.route, from);

                    router = router.route(from, get(handler));
                }
            }
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as memory_serve, load_assets, Asset, CacheControl, MemoryServe};
    use axum::{
        body::Body,
        http::{
            self,
            header::{self, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH},
            HeaderMap, HeaderName, HeaderValue, Request, StatusCode,
        },
        Router,
    };
    use tower::ServiceExt;

    async fn get(
        router: Router,
        path: &str,
        key: &str,
        value: &str,
    ) -> (StatusCode, HeaderMap<HeaderValue>) {
        let response = router
            .oneshot(
                Request::builder()
                    .method(http::Method::GET)
                    .header(key, value)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        (response.status(), response.headers().to_owned())
    }

    fn get_header<'s>(headers: &'s HeaderMap, name: &HeaderName) -> &'s str {
        headers.get(name).unwrap().to_str().unwrap()
    }

    #[derive(Clone)]
    struct AppState {}

    #[test]
    fn test_load_assets() {
        let assets: &'static [Asset] = load_assets!("../static");
        let routes: Vec<&str> = assets.iter().map(|a| a.route).collect();
        let content_types: Vec<&str> = assets.iter().map(|a| a.content_type).collect();
        let etags: Vec<&str> = assets.iter().map(|a| a.etag).collect();

        assert_eq!(
            routes,
            [
                "/assets/icon.jpg",
                "/assets/index.css",
                "/assets/index.js",
                "/assets/stars.svg",
                "/index.html"
            ]
        );
        assert_eq!(
            content_types,
            [
                "image/jpeg",
                "text/css",
                "application/javascript",
                "image/svg+xml",
                "text/html"
            ]
        );
        if cfg!(debug_assertions) {
            assert_eq!(etags, ["", "", "", "", ""]);
        } else {
            assert_eq!(
                etags,
                [
                    "e64f4683bf82d854df40b7246666f6f0816666ad8cd886a8e159535896eb03d6",
                    "ec4edeea111c854901385011f403e1259e3f1ba016dcceabb6d566316be3677b",
                    "86a7fdfd19700843e5f7344a63d27e0b729c2554c8572903ceee71f5658d2ecf",
                    "bd9dccc152de48cb7bedc35b9748ceeade492f6f904710f9c5d480bd6299cc7d",
                    "0639dc8aac157b58c74f65bbb026b2fd42bc81d9a0a64141df456fa23c214537"
                ]
            );
        }
    }

    #[tokio::test]
    async fn if_none_match_handling() {
        let memory_router = MemoryServe::new(load_assets!("../static")).into_router();
        let (code, headers) =
            get(memory_router.clone(), "/index.html", "accept", "text/html").await;
        let etag: &str = headers.get(header::ETAG).unwrap().to_str().unwrap();

        assert_eq!(code, 200);
        assert_eq!(
            etag,
            "0639dc8aac157b58c74f65bbb026b2fd42bc81d9a0a64141df456fa23c214537"
        );

        let (code, headers) = get(memory_router, "/index.html", "If-None-Match", etag).await;
        let length = get_header(&headers, &CONTENT_LENGTH);

        assert_eq!(code, 304);
        assert_eq!(length.parse::<i32>().unwrap(), 0);
    }

    #[tokio::test]
    async fn brotli_compression() {
        let memory_router = MemoryServe::new(load_assets!("../static")).into_router();
        let (code, headers) = get(
            memory_router.clone(),
            "/index.html",
            "accept-encoding",
            "br",
        )
        .await;
        let encoding = get_header(&headers, &CONTENT_ENCODING);
        let length = get_header(&headers, &CONTENT_LENGTH);

        assert_eq!(code, 200);
        assert_eq!(encoding, "br");
        assert_eq!(length.parse::<i32>().unwrap(), 178);

        // check disable compression
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .enable_brotli(false)
            .into_router();
        let (code, headers) = get(
            memory_router.clone(),
            "/index.html",
            "accept-encoding",
            "br",
        )
        .await;
        let length: &str = get_header(&headers, &CONTENT_LENGTH);

        assert_eq!(code, 200);
        assert_eq!(length.parse::<i32>().unwrap(), 437);
    }

    #[tokio::test]
    async fn gzip_compression() {
        let memory_router = MemoryServe::new(load_assets!("../static")).into_router();
        let (code, headers) = get(
            memory_router.clone(),
            "/index.html",
            "accept-encoding",
            "gzip",
        )
        .await;
        let encoding = get_header(&headers, &CONTENT_ENCODING);
        let length = get_header(&headers, &CONTENT_LENGTH);

        assert_eq!(code, 200);
        assert_eq!(encoding, "gzip");
        assert_eq!(length.parse::<i32>().unwrap(), 274);

        // check disable compression
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .enable_gzip(false)
            .into_router();
        let (code, headers) = get(
            memory_router.clone(),
            "/index.html",
            "accept-encoding",
            "gzip",
        )
        .await;
        let length: &str = get_header(&headers, &CONTENT_LENGTH);

        assert_eq!(code, 200);
        assert_eq!(length.parse::<i32>().unwrap(), 437);
    }

    #[tokio::test]
    async fn index_file() {
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .index_file(None)
            .into_router();

        let (code, _) = get(memory_router.clone(), "/", "accept", "*").await;
        assert_eq!(code, 404);

        let memory_router = MemoryServe::new(load_assets!("../static"))
            .index_file(Some("/index.html"))
            .into_router();

        let (code, _) = get(memory_router.clone(), "/", "accept", "*").await;
        assert_eq!(code, 200);
    }

    #[tokio::test]
    async fn fallback() {
        let memory_router = MemoryServe::new(load_assets!("../static")).into_router();
        let (code, _) = get(memory_router.clone(), "/foobar", "accept", "*").await;
        assert_eq!(code, 404);

        let memory_router = MemoryServe::new(load_assets!("../static"))
            .fallback(Some("/index.html"))
            .into_router();
        let (code, headers) = get(memory_router.clone(), "/foobar", "accept", "*").await;
        let length = get_header(&headers, &CONTENT_LENGTH);
        assert_eq!(code, 404);
        assert_eq!(length.parse::<i32>().unwrap(), 437);

        let memory_router = MemoryServe::new(load_assets!("../static"))
            .fallback(Some("/index.html"))
            .fallback_status(StatusCode::OK)
            .into_router();
        let (code, headers) = get(memory_router.clone(), "/foobar", "accept", "*").await;
        let length = get_header(&headers, &CONTENT_LENGTH);
        assert_eq!(code, 200);
        assert_eq!(length.parse::<i32>().unwrap(), 437);
    }

    #[tokio::test]
    async fn cache_control() {
        async fn check_cache_control(cache_control: CacheControl, expected: &str) {
            let memory_router = MemoryServe::new(load_assets!("../static"))
                .cache_control(cache_control)
                .into_router();

            let (code, headers) =
                get(memory_router.clone(), "/assets/icon.jpg", "accept", "*").await;

            let cache_control = get_header(&headers, &CACHE_CONTROL);
            assert_eq!(code, 200);
            assert_eq!(cache_control, expected);
        }

        check_cache_control(
            CacheControl::NoCache,
            CacheControl::NoCache.as_header().1.to_str().unwrap(),
        )
        .await;
        check_cache_control(
            CacheControl::Short,
            CacheControl::Short.as_header().1.to_str().unwrap(),
        )
        .await;
        check_cache_control(
            CacheControl::Medium,
            CacheControl::Medium.as_header().1.to_str().unwrap(),
        )
        .await;
        check_cache_control(
            CacheControl::Long,
            CacheControl::Long.as_header().1.to_str().unwrap(),
        )
        .await;

        async fn check_html_cache_control(cache_control: CacheControl, expected: &str) {
            let memory_router = MemoryServe::new(load_assets!("../static"))
                .html_cache_control(cache_control)
                .into_router();

            let (code, headers) = get(memory_router.clone(), "/index.html", "accept", "*").await;
            let cache_control = get_header(&headers, &CACHE_CONTROL);
            assert_eq!(code, 200);
            assert_eq!(cache_control, expected);
        }

        check_html_cache_control(
            CacheControl::NoCache,
            CacheControl::NoCache.as_header().1.to_str().unwrap(),
        )
        .await;
        check_html_cache_control(
            CacheControl::Short,
            CacheControl::Short.as_header().1.to_str().unwrap(),
        )
        .await;
        check_html_cache_control(
            CacheControl::Medium,
            CacheControl::Medium.as_header().1.to_str().unwrap(),
        )
        .await;
        check_html_cache_control(
            CacheControl::Long,
            CacheControl::Long.as_header().1.to_str().unwrap(),
        )
        .await;
    }

    #[tokio::test]
    async fn aliases() {
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .add_alias("/foobar", "/index.html")
            .add_alias("/baz", "/index.html")
            .into_router();
        let (code, _) = get(memory_router.clone(), "/foobar", "accept", "*").await;
        assert_eq!(code, 200);

        let (code, _) = get(memory_router.clone(), "/baz", "accept", "*").await;
        assert_eq!(code, 200);

        let (code, _) = get(memory_router.clone(), "/index.html", "accept", "*").await;
        assert_eq!(code, 200);

        let (code, _) = get(memory_router.clone(), "/barfoo", "accept", "*").await;
        assert_eq!(code, 404);
    }
}
