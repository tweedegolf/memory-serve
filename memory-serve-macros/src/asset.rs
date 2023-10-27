use syn::LitByteStr;

/// Internal data structure
pub(crate) struct Asset {
    pub(crate) route: String,
    pub(crate) path: String,
    pub(crate) etag: String,
    pub(crate) content_type: String,
    pub(crate) bytes: LitByteStr,
    pub(crate) brotli_bytes: LitByteStr,
}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.route == other.route
    }
}

impl Eq for Asset {}

impl PartialOrd for Asset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Asset {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.route.cmp(&other.route)
    }
}
