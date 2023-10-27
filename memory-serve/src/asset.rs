use axum::{
    http::{
        header::{CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
        HeaderMap, HeaderName, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
};
use tracing::{debug, error};

use crate::{
    util::{compress_brotli, compress_gzip, content_length, supports_encoding},
    ServeOptions,
};

pub const COMPRESS_TYPES: &[&str] = &[
    "text/html",
    "text/css",
    "application/json",
    "application/javascript",
    "text/javascript",
    "application/xml",
    "text/xml",
    "image/svg+xml",
];

const BROTLI_ENCODING: &str = "br";
#[allow(clippy::declare_interior_mutable_const)]
const BROTLI_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(BROTLI_ENCODING));
const GZIP_ENCODING: &str = "gzip";
#[allow(clippy::declare_interior_mutable_const)]
const GZIP_HEADER: (HeaderName, HeaderValue) =
    (CONTENT_ENCODING, HeaderValue::from_static(GZIP_ENCODING));

#[derive(Debug)]
pub struct Asset {
    pub route: &'static str,
    pub path: &'static str,
    pub etag: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
    pub brotli_bytes: &'static [u8],
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
        // TODO resolve path from current workspace

        let Ok(bytes) = std::fs::read(self.path) else {
            error!("File not found {}", self.path);
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

        let etag_value = sha256::digest(&bytes);
        let etag = (ETAG, HeaderValue::from_str(&etag_value).unwrap());
        let content_type = self.content_type();
        let cache_control = self.cache_control(options);

        if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
            if if_none_match == &etag_value {
                return (
                    StatusCode::NOT_MODIFIED,
                    [content_type, cache_control, etag],
                )
                    .into_response();
            }
        }

        if options.enable_brotli
            && !brotli_bytes.is_empty()
            && supports_encoding(headers, BROTLI_ENCODING)
        {
            (
                status,
                [
                    content_length(brotli_bytes.len()),
                    BROTLI_HEADER,
                    content_type,
                    cache_control,
                    etag,
                ],
                brotli_bytes,
            )
                .into_response()
        } else if options.enable_gzip
            && !gzip_bytes.is_empty()
            && supports_encoding(headers, GZIP_ENCODING)
        {
            (
                status,
                [
                    content_length(gzip_bytes.len()),
                    GZIP_HEADER,
                    content_type,
                    cache_control,
                    etag,
                ],
                gzip_bytes,
            )
                .into_response()
        } else {
            (
                status,
                [
                    content_length(bytes.len()),
                    content_type,
                    cache_control,
                    etag,
                ],
                bytes,
            )
                .into_response()
        }
    }

    pub(super) fn handler(
        &self,
        headers: &HeaderMap,
        status: StatusCode,
        bytes: &'static [u8],
        gzip_bytes: &'static [u8],
        options: &ServeOptions,
    ) -> Response {
        if bytes.is_empty() {
            debug!("using dynamic handler for {}", self.path);

            return self.dynamic_handler(headers, status, options);
        }

        let etag = (ETAG, HeaderValue::from_static(self.etag));
        let content_type = self.content_type();
        let cache_control = self.cache_control(options);

        if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
            if if_none_match == self.etag {
                return (
                    StatusCode::NOT_MODIFIED,
                    [content_type, cache_control, etag],
                )
                    .into_response();
            }
        }

        if options.enable_brotli
            && !self.brotli_bytes.is_empty()
            && supports_encoding(headers, BROTLI_ENCODING)
        {
            (
                status,
                [
                    content_length(self.brotli_bytes.len()),
                    BROTLI_HEADER,
                    content_type,
                    cache_control,
                    etag,
                ],
                self.brotli_bytes,
            )
                .into_response()
        } else if options.enable_gzip
            && !gzip_bytes.is_empty()
            && supports_encoding(headers, GZIP_ENCODING)
        {
            (
                status,
                [
                    content_length(gzip_bytes.len()),
                    GZIP_HEADER,
                    content_type,
                    cache_control,
                    etag,
                ],
                gzip_bytes,
            )
                .into_response()
        } else {
            (
                status,
                [
                    content_length(bytes.len()),
                    content_type,
                    cache_control,
                    etag,
                ],
                bytes,
            )
                .into_response()
        }
    }
}
