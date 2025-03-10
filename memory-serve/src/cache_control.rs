use axum::http::{HeaderName, HeaderValue, header::CACHE_CONTROL};

/// Options to choose from to configure the Cache-Control header for served files.
/// See [Cache control](index.html#cache-control)
#[derive(Debug, Clone, Copy)]
pub enum CacheControl {
    /// clients can keep assets that have cache busting for a year: `"max-age=31536000, immutable"`
    Long,
    /// assets without cache busting are revalidated after a day and can be kept for a week: `"max-age=604800, stale-while-revalidate=86400"`
    Medium,
    /// cache kept for max 5 minutes, only at the client (not in a proxy): `"max-age:300, private"`
    Short,
    /// do not cache if freshness is really vital: `"no-cache"`
    NoCache,
    /// custom value
    Custom(&'static str),
}

impl CacheControl {
    pub(crate) fn as_header(&self) -> (HeaderName, HeaderValue) {
        let value = match self {
            Self::Long => "max-age=31536000, immutable",
            Self::Medium => "max-age=604800, stale-while-revalidate=86400",
            Self::Short => "max-age:300, private",
            Self::NoCache => "no-cache",
            Self::Custom(value) => value,
        };

        (CACHE_CONTROL, HeaderValue::from_static(value))
    }
}
