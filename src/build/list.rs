use std::path::Path;

use walkdir::WalkDir;

use crate::{
    options::{COMPRESS_TYPES, MIN_COMPRESS_SIZE},
    util::{
        compression::compress_brotli,
        route::{path_to_content_type, path_to_route},
    },
};

use super::file_asset::FileAsset;

/// List all assets in the given directory (recursively) and return a list of assets with metadata
pub(super) fn list_assets(base_path: &Path, embed: bool, log: fn(&str)) -> Vec<FileAsset> {
    let mut assets: Vec<FileAsset> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path().to_owned();
            let route = path_to_route(base_path, entry.path());

            let Ok(metadata) = entry.metadata() else {
                log(&format!(
                    "skipping file {route}, could not get file metadata"
                ));
                return None;
            };

            let original_size = metadata.len();

            // skip directories
            if !metadata.is_file() {
                return None;
            };

            // skip empty
            if original_size == 0 {
                log(&format!("skipping file {route}: file empty"));
                return None;
            }

            let Some(content_type) = path_to_content_type(entry.path()) else {
                log(&format!(
                    "skipping file {route}, could not determine file extension"
                ));
                return None;
            };

            let should_compress = COMPRESS_TYPES.contains(&content_type.as_str())
                && metadata.len() >= MIN_COMPRESS_SIZE;

            // do not load assets into the binary in debug / development mode
            if !embed {
                log(&format!("including {route} (dynamically)"));

                return Some(FileAsset {
                    route,
                    path: path.to_owned(),
                    content_type,
                    etag: Default::default(),
                    compressed_bytes: None,
                    should_compress,
                });
            }

            let Ok(bytes) = std::fs::read(entry.path()) else {
                log(&format!("skipping file {route}: file is not readable"));
                return None;
            };

            let etag: String = sha256::digest(&bytes);
            let enable_compression = embed && should_compress && !cfg!(debug_assertions);

            let compressed_bytes = if enable_compression {
                compress_brotli(&bytes)
            } else {
                None
            };

            if let Some(compressed_size) = compressed_bytes.as_ref().map(|b| b.len()) {
                log(&format!(
                    "including {route} {original_size} -> {compressed_size} bytes (compressed)"
                ));
            } else {
                log(&format!(
                    "including {route} {original_size} bytes (uncompressed)"
                ));
            }

            Some(FileAsset {
                route: route.clone(),
                path: path.to_owned(),
                content_type,
                etag,
                compressed_bytes,
                should_compress,
            })
        })
        .collect();

    assets.sort();

    assets
}
