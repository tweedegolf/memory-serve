use axum::http::{header::CACHE_CONTROL, HeaderName, HeaderValue};

/// Options to choose from to configure the Cache-Control header for served files.
/// See [Cache control](index.html#cache-control)
#[derive(Debug, Clone, Copy)]
pub enum CacheControl {
    Long,
    Medium,
    Short,
    NoCache,
    Custom(&'static str),
}

impl CacheControl {
    pub(crate) fn as_header(&self) -> (HeaderName, HeaderValue) {
        let value = match self {
            Self::Long => {
                // clients can keep assets that have cache busting for a year
                "max-age=31536000, immutable"
            }
            Self::Medium => {
                // assets without cache busting are revalidated after a day and can be kept for a week
                "max-age=604800, stale-while-revalidate=86400"
            }
            Self::Short => {
                // cache kept for max 5 minutes, only at the client (not in a proxy)
                "max-age:300, private"
            }
            Self::NoCache => {
                // do not cache if freshness is really vital
                "no-cache"
            }
            Self::Custom(value) => value,
        };

        (CACHE_CONTROL, HeaderValue::from_static(value))
    }
}
