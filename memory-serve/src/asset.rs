use axum::{
    http::{
        header::{CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
        HeaderMap, HeaderName, HeaderValue, StatusCode,
    },
    response::IntoResponse,
};

use crate::{
    util::{content_length, supports_encoding},
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

#[derive(Debug)]
pub struct Asset {
    pub route: &'static str,
    pub etag: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
    pub brotli_bytes: &'static [u8],
}

impl Asset {
    pub(super) fn handler(
        &self,
        headers: &HeaderMap,
        status: StatusCode,
        bytes: &'static [u8],
        gzip_bytes: &'static [u8],
        options: &ServeOptions,
    ) -> impl IntoResponse {
        let content_type = HeaderValue::from_static(self.content_type);
        let etag = HeaderValue::from_static(self.etag);
        let cache_control = match self.content_type {
            "text/html" => options.html_cache_control.as_header(),
            _ => options.cache_control.as_header(),
        };

        if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
            if if_none_match == self.etag {
                return (
                    StatusCode::NOT_MODIFIED,
                    [(CONTENT_TYPE, content_type), cache_control, (ETAG, etag)],
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
                    (CONTENT_TYPE, content_type),
                    cache_control,
                    (ETAG, etag),
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
                    (CONTENT_TYPE, content_type),
                    cache_control,
                    (ETAG, etag),
                ],
                gzip_bytes,
            )
                .into_response()
        } else {
            (
                status,
                [
                    content_length(bytes.len()),
                    (CONTENT_TYPE, content_type),
                    cache_control,
                    (ETAG, etag),
                ],
                bytes,
            )
                .into_response()
        }
    }
}
