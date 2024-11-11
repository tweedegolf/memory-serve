use axum::{
    http::{
        header::{CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
        HeaderMap, HeaderName, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
};
use memory_serve_core::COMPRESS_TYPES;
use tracing::debug;

use crate::{
    util::{compress_brotli, compress_gzip, content_length, supports_encoding},
    ServeOptions,
};

const BROTLI_ENCODING: &str = "br";
#[allow(clippy::declare_interior_mutable_const)]
const BROTLI_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(BROTLI_ENCODING));
const GZIP_ENCODING: &str = "gzip";
#[allow(clippy::declare_interior_mutable_const)]
const GZIP_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(GZIP_ENCODING));

/// Represents a static asset that can be served
#[derive(Debug)]
pub struct Asset {
    pub route: &'static str,
    pub path: &'static str,
    pub etag: &'static str,
    pub content_type: &'static str,
    pub bytes: Option<&'static [u8]>,
    pub is_compressed: bool,
}

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

impl<'t, B: IntoResponse> AssetResponse<'t, B> {
    fn into_response(self) -> Response {
        let content_type = self.asset.content_type();
        let cache_control = self.asset.cache_control(self.options);
        let etag_header = (ETAG, HeaderValue::from_str(self.etag).unwrap());

        if let Some(if_none_match) = self.headers.get(IF_NONE_MATCH) {
            if if_none_match == self.etag {
                return (
                    StatusCode::NOT_MODIFIED,
                    [content_type, cache_control, etag_header],
                )
                    .into_response();
            }
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
    fn cache_control(&self, options: &ServeOptions) -> (HeaderName, HeaderValue) {
        match self.content_type {
            "text/html" => options.html_cache_control.as_header(),
            _ => options.cache_control.as_header(),
        }
    }

    fn content_type(&self) -> (HeaderName, HeaderValue) {
        (CONTENT_TYPE, HeaderValue::from_static(self.content_type))
    }

    fn dynamic_handler(
        &self,
        headers: &HeaderMap,
        status: StatusCode,
        options: &ServeOptions,
    ) -> Response {
        let Ok(bytes) = std::fs::read(self.path) else {
            return StatusCode::NOT_FOUND.into_response();
        };

        let brotli_bytes = if options.enable_brotli && COMPRESS_TYPES.contains(&self.content_type) {
            compress_brotli(&bytes).unwrap_or_default()
        } else {
            Default::default()
        };

        let gzip_bytes = if options.enable_gzip && COMPRESS_TYPES.contains(&self.content_type) {
            compress_gzip(&bytes).unwrap_or_default()
        } else {
            Default::default()
        };

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
