use std::path::PathBuf;

/// Internal data structure
pub(super) struct FileAsset {
    pub(super) route: String,
    pub(super) path: PathBuf,
    pub(super) etag: String,
    pub(super) content_type: String,
    pub(super) compressed_bytes: Option<Vec<u8>>,
    pub(super) should_compress: bool,
}

impl PartialEq for FileAsset {
    fn eq(&self, other: &Self) -> bool {
        self.route == other.route
    }
}

impl Eq for FileAsset {}

impl PartialOrd for FileAsset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileAsset {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.route.cmp(&other.route)
    }
}
