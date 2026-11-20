use axum::http::{HeaderMap, HeaderValue, header::ACCEPT_ENCODING};

/// Strip an optional weak validator prefix (`W/`) from an entity-tag.
fn strip_weak(etag: &str) -> &str {
    etag.strip_prefix("W/").unwrap_or(etag)
}

/// Check whether an `If-None-Match` header value matches the given (quoted)
/// entity-tag, following the weak comparison function of RFC 7232 §2.3.2.
///
/// Handles the `*` wildcard, comma-separated lists of entity-tags and the
/// weak validator prefix (`W/`).
pub(crate) fn if_none_match_matches(header_value: &str, quoted_etag: &str) -> bool {
    let target = strip_weak(quoted_etag);

    header_value.split(',').any(|candidate| {
        let candidate = candidate.trim();

        candidate == "*" || strip_weak(candidate) == target
    })
}

/// Determine the effective quality value (`q`) the client assigns to the given
/// encoding in its `Accept-Encoding` header.
///
/// Returns `None` when the encoding is not acceptable (absent, or explicitly
/// refused with `q=0`). An explicit entry for the encoding takes precedence
/// over the `*` wildcard. Entries without a `q` parameter default to `1.0`.
pub(crate) fn encoding_qvalue(headers: &HeaderMap, encoding: &str) -> Option<f32> {
    let header_value = headers
        .get(ACCEPT_ENCODING)
        .and_then(|v: &HeaderValue| v.to_str().ok())?;

    let mut wildcard_q: Option<f32> = None;

    for item in header_value
        .split_whitespace()
        .collect::<String>()
        // codings and the `q` parameter are case-insensitive (RFC 9110 §8.4.1)
        .to_ascii_lowercase()
        .split(',')
    {
        if item.is_empty() {
            continue;
        }

        let mut parts = item.splitn(2, ";q=");
        let token = parts.next().unwrap_or_default();
        // A missing `q` parameter defaults to `1.0`; an unparseable one is
        // treated as `0.0` (not acceptable).
        let q = parts
            .next()
            .map_or(1.0, |q| q.parse::<f32>().unwrap_or(0.0));

        if token == encoding {
            // An explicit entry is authoritative, even when it refuses the
            // encoding with `q=0`.
            return (q > 0.0).then_some(q);
        }

        if token == "*" {
            wildcard_q = Some(q);
        }
    }

    wildcard_q.filter(|q| *q > 0.0)
}

#[cfg(test)]
mod tests {
    use super::{encoding_qvalue, if_none_match_matches};
    use axum::http::{HeaderMap, HeaderValue, header::ACCEPT_ENCODING};

    fn check(header: &str, encoding: &str) -> bool {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_str(header).unwrap());

        encoding_qvalue(&headers, encoding).is_some()
    }

    fn qvalue(header: &str, encoding: &str) -> Option<f32> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_str(header).unwrap());

        encoding_qvalue(&headers, encoding)
    }

    #[test]
    fn qvalues() {
        // Missing q defaults to 1.0.
        assert_eq!(qvalue("gzip", "gzip"), Some(1.0));
        // Explicit q is honored.
        assert_eq!(qvalue("br;q=1.0, gzip;q=0.8", "gzip"), Some(0.8));
        assert_eq!(qvalue("br;q=1.0, gzip;q=0.8", "br"), Some(1.0));
        // Wildcard fallback.
        assert_eq!(qvalue("*;q=0.5", "gzip"), Some(0.5));
        // An explicit q=0 overrides a permissive wildcard.
        assert_eq!(qvalue("*, gzip;q=0", "gzip"), None);
        // A permissive explicit entry overrides a wildcard refusal.
        assert_eq!(qvalue("*;q=0, gzip", "gzip"), Some(1.0));
        // Absent and refused.
        assert_eq!(qvalue("gzip", "br"), None);
        assert_eq!(qvalue("gzip;q=0", "gzip"), None);
        // Codings and the q parameter are case-insensitive.
        assert_eq!(qvalue("GZIP;Q=0.5", "gzip"), Some(0.5));
        assert_eq!(qvalue("Br", "br"), Some(1.0));
    }

    #[test]
    fn if_none_match() {
        assert!(if_none_match_matches("\"abc\"", "\"abc\""));
        // Weak validator prefix is stripped for comparison.
        assert!(if_none_match_matches("W/\"abc\"", "\"abc\""));
        assert!(if_none_match_matches("\"abc\"", "W/\"abc\""));
        // Comma-separated list.
        assert!(if_none_match_matches("\"x\", \"abc\", \"y\"", "\"abc\""));
        // Wildcard matches any etag.
        assert!(if_none_match_matches("*", "\"abc\""));
        // No match.
        assert!(!if_none_match_matches("\"x\", \"y\"", "\"abc\""));
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

        // q=0 in any spelling means the client refuses that encoding.
        assert!(!check("gzip;q=0", "gzip"));
        assert!(!check("gzip;q=0.0", "gzip"));
        assert!(!check("gzip;q=0.000", "gzip"));
        assert!(!check("br;q=1.0, gzip;q=0.0", "gzip"));
        assert!(!check("*;q=0", "gzip"));
        assert!(!check("*;q=0.0", "gzip"));
    }
}
