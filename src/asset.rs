use axum::{
    http::{
        HeaderMap, HeaderName, HeaderValue, StatusCode,
        header::{CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    response::{IntoResponse, Response},
};
use tracing::debug;

use crate::{
    options::ServeOptions,
    util::{
        compression::{compress_brotli, compress_gzip, decompress_brotli},
        headers::{content_length, supports_encoding},
    },
};

const BROTLI_ENCODING: &str = "br";

const BROTLI_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(BROTLI_ENCODING));

const GZIP_ENCODING: &str = "gzip";

const GZIP_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(GZIP_ENCODING));

/// Preferred compression for a dynamically served asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnDemandEncoding {
    /// Send the original bytes without further compression.
    Identity,
    /// Compress the response using brotli.
    Brotli,
    /// Compress the response using gzip.
    Gzip,
}

/// Represents a static asset that can be served
#[derive(Debug)]
pub struct Asset {
    /// The HTTP route used to serve the asset, e.g. `/index.html`.
    pub route: &'static str,
    /// Absolute filesystem path pointing to the source asset on disk.
    pub path: &'static str,
    /// Strong validator (SHA-256) used for HTTP caching semantics.
    pub etag: &'static str,
    /// MIME type advertised for the asset.
    pub content_type: &'static str,
    /// Optional embedded bytes for the asset; `None` when dynamic loading is used.
    pub bytes: Option<&'static [u8]>,
    /// Indicates if the embedded bytes are already brotli compressed.
    pub is_compressed: bool,
    /// Whether the asset should be compressed before sending to clients.
    pub should_compress: bool,
}

/// Aggregates response metadata and payloads for an asset request.
struct AssetResponse<'t, B> {
    options: &'t ServeOptions,
    headers: &'t HeaderMap,
    status: StatusCode,
    asset: &'t Asset,
    etag: &'t str,
    bytes: B,
    bytes_len: usize,
    brotli_bytes: B,
    brotli_bytes_len: usize,
    gzip_bytes: B,
    gzip_bytes_len: usize,
}

impl<B: IntoResponse> AssetResponse<'_, B> {
    /// Construct an Axum `Response` from the gathered asset data.
    fn into_response(self) -> Response {
        let content_type = self.asset.content_type();
        let cache_control = self.asset.cache_control(self.options);
        let etag_header = (ETAG, HeaderValue::from_str(self.etag).unwrap());

        if let Some(if_none_match) = self.headers.get(IF_NONE_MATCH)
            && if_none_match == self.etag
        {
            return (
                StatusCode::NOT_MODIFIED,
                [content_type, cache_control, etag_header],
            )
                .into_response();
        }

        if self.options.enable_brotli
            && self.brotli_bytes_len > 0
            && supports_encoding(self.headers, BROTLI_ENCODING)
        {
            return (
                self.status,
                [
                    content_length(self.brotli_bytes_len),
                    BROTLI_HEADER,
                    content_type,
                    cache_control,
                    etag_header,
                ],
                self.brotli_bytes,
            )
                .into_response();
        }

        if self.options.enable_gzip
            && self.gzip_bytes_len > 0
            && supports_encoding(self.headers, GZIP_ENCODING)
        {
            return (
                self.status,
                [
                    content_length(self.gzip_bytes_len),
                    GZIP_HEADER,
                    content_type,
                    cache_control,
                    etag_header,
                ],
                self.gzip_bytes,
            )
                .into_response();
        }

        (
            self.status,
            [
                content_length(self.bytes_len),
                content_type,
                cache_control,
                etag_header,
            ],
            self.bytes,
        )
            .into_response()
    }
}

impl Asset {
    /// Pick the cache policy for the asset based on its MIME type.
    fn cache_control(&self, options: &ServeOptions) -> (HeaderName, HeaderValue) {
        match self.content_type {
            "text/html" => options.html_cache_control.as_header(),
            _ => options.cache_control.as_header(),
        }
    }

    /// Produce the Content-Type header tuple for the asset.
    fn content_type(&self) -> (HeaderName, HeaderValue) {
        (CONTENT_TYPE, HeaderValue::from_static(self.content_type))
    }

    /// Get the bytes for the asset, which is possibly compressed in the binary
    pub(crate) fn leak_bytes(
        &self,
        options: &'static ServeOptions,
    ) -> (&'static [u8], &'static [u8], &'static [u8]) {
        let mut uncompressed = self.bytes.unwrap_or_default();

        if self.is_compressed {
            uncompressed = Box::new(decompress_brotli(uncompressed).unwrap_or_default()).leak()
        }

        let gzip_bytes = if self.should_compress && options.enable_gzip {
            Box::new(compress_gzip(uncompressed).unwrap_or_default()).leak()
        } else {
            Default::default()
        };

        let brotli_bytes = if self.should_compress && options.enable_brotli {
            self.bytes.unwrap_or_default()
        } else {
            Default::default()
        };

        (uncompressed, brotli_bytes, gzip_bytes)
    }

    /// Load the asset bytes from disk, returning a `404` if the file is missing.
    fn read_source_bytes(&self) -> Result<Vec<u8>, StatusCode> {
        std::fs::read(self.path).map_err(|_| StatusCode::NOT_FOUND)
    }

    /// Decide which compression algorithm (if any) to use for a dynamic request.
    fn negotiate_dynamic_encoding(
        &self,
        headers: &HeaderMap,
        options: &ServeOptions,
    ) -> OnDemandEncoding {
        if !self.should_compress {
            return OnDemandEncoding::Identity;
        }

        if options.enable_brotli && supports_encoding(headers, BROTLI_ENCODING) {
            return OnDemandEncoding::Brotli;
        }

        if options.enable_gzip && supports_encoding(headers, GZIP_ENCODING) {
            return OnDemandEncoding::Gzip;
        }

        OnDemandEncoding::Identity
    }

    /// Compress the provided bytes according to the negotiated encoding.
    fn encode_dynamic_bytes(&self, bytes: &[u8], encoding: OnDemandEncoding) -> (Vec<u8>, Vec<u8>) {
        match encoding {
            OnDemandEncoding::Brotli => (compress_brotli(bytes).unwrap_or_default(), Vec::new()),
            OnDemandEncoding::Gzip => (Vec::new(), compress_gzip(bytes).unwrap_or_default()),
            OnDemandEncoding::Identity => (Vec::new(), Vec::new()),
        }
    }

    /// Load an asset from disk and emit a response tailored to client encodings.
    fn dynamic_handler(
        &self,
        headers: &HeaderMap,
        status: StatusCode,
        options: &ServeOptions,
    ) -> Response {
        let bytes = match self.read_source_bytes() {
            Ok(bytes) => bytes,
            Err(status) => return status.into_response(),
        };

        let encoding = self.negotiate_dynamic_encoding(headers, options);
        let (brotli_bytes, gzip_bytes) = self.encode_dynamic_bytes(&bytes, encoding);

        let etag = sha256::digest(&bytes);

        AssetResponse {
            options,
            headers,
            status,
            asset: self,
            etag: &etag,
            bytes_len: bytes.len(),
            bytes,
            brotli_bytes_len: brotli_bytes.len(),
            brotli_bytes,
            gzip_bytes_len: gzip_bytes.len(),
            gzip_bytes,
        }
        .into_response()
    }

    /// Serve an asset using either embedded bytes or on-demand loading.
    pub(super) fn handler(
        &self,
        headers: &HeaderMap,
        status: StatusCode,
        bytes: &'static [u8],
        brotli_bytes: &'static [u8],
        gzip_bytes: &'static [u8],
        options: &ServeOptions,
    ) -> Response {
        if bytes.is_empty() {
            debug!("using dynamic handler for {}", self.path);

            return self.dynamic_handler(headers, status, options);
        }

        AssetResponse {
            options,
            headers,
            status,
            asset: self,
            etag: self.etag,
            bytes_len: bytes.len(),
            bytes,
            brotli_bytes_len: brotli_bytes.len(),
            brotli_bytes,
            gzip_bytes_len: gzip_bytes.len(),
            gzip_bytes,
        }
        .into_response()
    }
}
