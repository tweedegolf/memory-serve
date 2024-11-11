use std::path::PathBuf;

/// Internal data structure
pub struct Asset {
    pub route: String,
    pub path: PathBuf,
    pub etag: String,
    pub content_type: String,
    pub compressed_bytes: Option<Vec<u8>>,
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
