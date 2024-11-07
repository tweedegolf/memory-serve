use std::path::Path;

use walkdir::WalkDir;

use crate::{asset::Asset, util::{compress_brotli, path_to_content_type, path_to_route}, COMPRESS_TYPES};

pub fn list_assets(base_path: &Path, embed: bool, log: fn(&str)) -> Vec<Asset> {
    let mut assets: Vec<Asset> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path().to_owned();
            let route = path_to_route(base_path, entry.path());

            let Ok(metadata) = entry.metadata() else {
                log(&format!("skipping file {route}, could not get file metadata"));
                return None;
            };

            // skip directories
            if !metadata.is_file() {
                return None;
            };

            // skip empty
            if metadata.len() == 0 {
                log(&format!("skipping file {route}: file empty"));
                return None;
            }

            let Some(content_type) = path_to_content_type(entry.path()) else {
                log(&format!("skipping file {route}, could not determine file extension"));
                return None;
            };

            // do not load assets into the binary in debug / development mode
            if !embed {
                log(&format!("including {route} (dynamically)"));

                return Some(Asset {
                    route,
                    path: path.to_owned(),
                    content_type,
                    etag: Default::default(),
                    compressed_bytes: None,
                });
            }

            let Ok(bytes) = std::fs::read(entry.path()) else {
                log(&format!("skipping file {route}: file is not readable"));
                return None;
            };

            let etag: String = sha256::digest(&bytes);
            let original_size = bytes.len();
            let is_compress_type = COMPRESS_TYPES.contains(&content_type.as_str());
            let brotli_bytes = if is_compress_type {
                compress_brotli(&bytes)
            } else {
                None
            };

            let mut asset = Asset {
                route: route.clone(),
                path: path.to_owned(),
                content_type,
                etag,
                compressed_bytes: None,
            };

            if is_compress_type {
                match brotli_bytes {
                    Some(brotli_bytes) if brotli_bytes.len() >= original_size => {
                        log(&format!("including {route} {original_size} bytes (compression unnecessary)"));
                    }
                    Some(brotli_bytes) => {
                        log(&format!(
                            "including {route} {original_size} -> {} bytes (compressed)",
                            brotli_bytes.len()
                        ));

                        asset.compressed_bytes = Some(brotli_bytes);
                    }
                    None => {
                        log(&format!("including {route} {original_size} bytes (compression failed)"));
                    }
                }
            } else {
                log(&format!("including {route} {original_size} bytes"));
            }

            Some(asset)
        })
        .collect();

    assets.sort();

    assets
}