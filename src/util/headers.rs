use axum::http::{
    HeaderMap, HeaderName, HeaderValue,
    header::{ACCEPT_ENCODING, CONTENT_LENGTH},
};

/// Check if the client supports the given encoding.
pub(crate) fn supports_encoding(headers: &HeaderMap, encoding: &str) -> bool {
    let Some(header_value) = headers
        .get(ACCEPT_ENCODING)
        .and_then(|v: &HeaderValue| v.to_str().ok())
    else {
        return false;
    };

    header_value
        .split_whitespace()
        .collect::<String>()
        .split(',')
        .filter_map(|item| {
            let mut parts = item.splitn(2, ";q=");
            let encoding = parts.next();

            if parts.next() == Some("0") {
                None
            } else {
                encoding
            }
        })
        .any(|v| v == encoding || v == "*")
}

/// Build a `Content-Length` tuple for the given byte length.
pub(crate) fn content_length(len: usize) -> (HeaderName, HeaderValue) {
    (CONTENT_LENGTH, HeaderValue::from(len))
}

#[cfg(test)]
mod tests {
    use super::supports_encoding;
    use axum::http::{HeaderMap, HeaderValue, header::ACCEPT_ENCODING};

    fn check(header: &str, encoding: &str) -> bool {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_str(header).unwrap());

        supports_encoding(&headers, encoding)
    }

    #[test]
    fn accept_encoding() {
        assert!(check("gzip", "gzip"));
        assert!(check("gzip, compress, br", "gzip"));
        assert!(check("br;q=1.0, gzip;q=0.8, *;q=0.1", "gzip"));
        assert!(!check("gzip", "br"));
        assert!(check("gzip, compress, br", "br"));
        assert!(check("br;q=1.0, gzip;q=0.8, *;q=0.1", "br"));
        assert!(!check("gzip", "compress"));
        assert!(check("gzip, compress, br", "compress"));
        assert!(check("br;q=1.0, gzip;q=0.8, *;q=0.1", "compress"));
        assert!(!check("gzip", "zstd"));
        assert!(!check("gzip, compress, br", "zstd"));
        assert!(check("br;q=1.0, gzip;q=0.8, *;q=0.1", "zstd"));
    }
}
