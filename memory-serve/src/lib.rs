#![doc = include_str!("../README.md")]
use axum::{
    http::{HeaderMap, StatusCode},
    routing::get,
};
use std::future::ready;
use tracing::info;
mod asset;
mod cache_control;
mod util;

#[allow(unused)]
use crate as memory_serve;

use crate::util::{compress_gzip, decompress_brotli};

pub use crate::{asset::Asset, cache_control::CacheControl};
pub use memory_serve_macros::load_assets;

#[derive(Debug, Clone, Copy)]
struct ServeOptions {
    index_file: Option<&'static str>,
    index_on_subdirectories: bool,
    fallback: Option<&'static str>,
    fallback_status: StatusCode,
    html_cache_control: CacheControl,
    cache_control: CacheControl,
    enable_brotli: bool,
    enable_gzip: bool,
    enable_clean_url: bool,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            index_file: Some("/index.html"),
            index_on_subdirectories: false,
            fallback: None,
            fallback_status: StatusCode::NOT_FOUND,
            html_cache_control: CacheControl::Short,
            cache_control: CacheControl::Medium,
            enable_brotli: !cfg!(debug_assertions),
            enable_gzip: !cfg!(debug_assertions),
            enable_clean_url: false,
        }
    }
}

/// Helper struct to create and configure an axum to serve static files from
/// memory.
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

    /// Initiate a `MemoryServe` instance, takes the contents of `memory_serve_assets.bin`
    /// created at build time.
    /// Specify which asset directory to include using the environment variable `ASSET_DIR`.
    pub fn from_env() -> Self {
        let assets: &[(&str, &[Asset])] =
            include!(concat!(env!("OUT_DIR"), "/memory_serve_assets.rs"));

        if assets.is_empty() {
            panic!("No assets found, did you forget to set the ASSET_DIR environment variable?");
        }

        Self::new(assets[0].1)
    }

    /// Include a directory using a named environment variable, prefixed by ASSRT_DIR_.
    /// Specify which asset directory to include using the environment variable `ASSET_DIR_<SOME NAME>`.
    /// The name should be in uppercase.
    /// For example to include assets from the public directory using the name PUBLIC, set the enirobment variable
    /// `ASSET_DIR_PUBLIC=./public` and call `MemoryServe::from_name("PUBLIC")`.
    pub fn from_env_name(name: &str) -> Self {
        let assets: &[(&str, &[Asset])] =
            include!(concat!(env!("OUT_DIR"), "/memory_serve_assets.rs"));

        let assets = assets
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, a)| *a)
            .unwrap_or_default();

        if assets.is_empty() {
            panic!(
                "No assets found, did you forget to set the ASSET_DIR_{name} environment variable?"
            );
        }

        Self::new(assets)
    }

    /// Which static file to serve on the route "/" (the index)
    /// The path (or route) should be relative to the directory set with
    /// the `ASSET_DIR` variable, but prepended with a slash.
    /// By default this is `Some("/index.html")`
    pub fn index_file(mut self, index_file: Option<&'static str>) -> Self {
        self.options.index_file = index_file;

        self
    }

    /// Whether to serve the corresponding index.html file when a route
    /// matches a subdirectory
    pub fn index_on_subdirectories(mut self, enable: bool) -> Self {
        self.options.index_on_subdirectories = enable;

        self
    }

    /// Which static file to serve when no other routes are matched, also see
    /// [fallback](https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.fallback)
    /// The path (or route) should be relative to the directory set with
    /// the `ASSET_DIR` variable, but prepended with a slash.
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

    /// Whether to enable clean URLs. When set to `true`, the routing path for
    /// HTML files will not include the extension so that a file located at
    /// "/about.html" maps to "/about" instead of "/about.html".
    pub fn enable_clean_url(mut self, enable_clean_url: bool) -> Self {
        self.options.enable_clean_url = enable_clean_url;

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
            let mut bytes = asset.bytes.unwrap_or_default();

            if asset.is_compressed {
                bytes = Box::new(decompress_brotli(bytes).unwrap_or_default()).leak()
            }

            let gzip_bytes = if asset.is_compressed && options.enable_gzip {
                Box::new(compress_gzip(bytes).unwrap_or_default()).leak()
            } else {
                Default::default()
            };

            let brotli_bytes = if asset.is_compressed {
                asset.bytes.unwrap_or_default()
            } else {
                Default::default()
            };

            if !bytes.is_empty() {
                if asset.is_compressed {
                    info!(
                        "serving {} {} -> {} bytes (compressed)",
                        asset.route,
                        bytes.len(),
                        brotli_bytes.len()
                    );
                } else {
                    info!("serving {} {} bytes", asset.route, bytes.len());
                }
            } else {
                info!("serving {} (dynamically)", asset.route);
            }

            let handler = |headers: HeaderMap| {
                ready(asset.handler(
                    &headers,
                    StatusCode::OK,
                    bytes,
                    brotli_bytes,
                    gzip_bytes,
                    options,
                ))
            };

            if Some(asset.route) == options.fallback {
                info!("serving {} as fallback", asset.route);

                router = router.fallback(|headers: HeaderMap| {
                    ready(asset.handler(
                        &headers,
                        options.fallback_status,
                        bytes,
                        brotli_bytes,
                        gzip_bytes,
                        options,
                    ))
                });
            }

            if let Some(index) = options.index_file {
                if asset.route == index {
                    info!("serving {} as index on /", asset.route);

                    router = router.route("/", get(handler));
                } else if options.index_on_subdirectories && asset.route.ends_with(index) {
                    let path = &asset.route[..asset.route.len() - index.len()];
                    info!("serving {} as index on {}", asset.route, path);

                    router = router.route(path, get(handler));
                }
            }

            let path = if options.enable_clean_url && asset.route.ends_with(".html") {
                &asset.route[..asset.route.len() - 5]
            } else {
                asset.route
            };
            router = router.route(path, get(handler));

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
    use crate::{self as memory_serve, Asset, CacheControl, MemoryServe};
    use axum::{
        body::Body,
        http::{
            self,
            header::{self, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH},
            HeaderMap, HeaderName, HeaderValue, Request, StatusCode,
        },
        Router,
    };
    use memory_serve_macros::load_assets;
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

    #[tokio::test]
    async fn test_load_assets() {
        let assets: &[Asset] = load_assets!("../static");
        let routes: Vec<&str> = assets.iter().map(|a| a.route).collect();
        let content_types: Vec<&str> = assets.iter().map(|a| a.content_type).collect();
        let etags: Vec<&str> = assets.iter().map(|a| a.etag).collect();

        assert_eq!(
            routes,
            [
                "/about.html",
                "/assets/icon.jpg",
                "/assets/index.css",
                "/assets/index.js",
                "/assets/stars.svg",
                "/blog/index.html",
                "/index.html"
            ]
        );
        assert_eq!(
            content_types,
            [
                "text/html",
                "image/jpeg",
                "text/css",
                "text/javascript",
                "image/svg+xml",
                "text/html",
                "text/html"
            ]
        );
        if cfg!(debug_assertions) || cfg!(feature = "force-embed") {
            assert_eq!(etags, ["", "", "", "", "", "", ""]);
        } else {
            assert_eq!(
                etags,
                [
                    "56a0dcb83ec56b6c967966a1c06c7b1392e261069d0844aa4e910ca5c1e8cf58",
                    "e64f4683bf82d854df40b7246666f6f0816666ad8cd886a8e159535896eb03d6",
                    "ec4edeea111c854901385011f403e1259e3f1ba016dcceabb6d566316be3677b",
                    "86a7fdfd19700843e5f7344a63d27e0b729c2554c8572903ceee71f5658d2ecf",
                    "bd9dccc152de48cb7bedc35b9748ceeade492f6f904710f9c5d480bd6299cc7d",
                    "89e9873a8e49f962fe83ad2bfe6ac9b21ef7c1b4040b99c34eb783dccbadebc5",
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
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .enable_brotli(true)
            .into_router();
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
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .enable_gzip(true)
            .into_router();
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
    async fn index_file_on_subdirs() {
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .index_file(Some("/index.html"))
            .index_on_subdirectories(false)
            .into_router();

        let (code, _) = get(memory_router.clone(), "/blog", "accept", "*").await;
        assert_eq!(code, 404);

        let memory_router = MemoryServe::new(load_assets!("../static"))
            .index_file(Some("/index.html"))
            .index_on_subdirectories(true)
            .into_router();

        let (code, _) = get(memory_router.clone(), "/blog", "accept", "*").await;
        assert_eq!(code, 200);
    }

    #[tokio::test]
    async fn clean_url() {
        let memory_router = MemoryServe::new(load_assets!("../static"))
            .enable_clean_url(true)
            .into_router();

        let (code, _) = get(memory_router.clone(), "/about.html", "accept", "*").await;
        assert_eq!(code, 404);

        let (code, _) = get(memory_router.clone(), "/about", "accept", "*").await;
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
