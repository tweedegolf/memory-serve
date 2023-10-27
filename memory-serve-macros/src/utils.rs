use mime_guess::mime;
use proc_macro2::Span;
use std::{io::Write, path::Path};
use syn::LitByteStr;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::Asset;

const COMPRESS_TYPES: &[&str] = &[
    "text/html",
    "text/css",
    "application/json",
    "application/javascript",
    "text/javascript",
    "application/xml",
    "text/xml",
    "image/svg+xml",
];

fn path_to_route(path: &Path) -> String {
    "/".to_owned()
        + path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .skip(1)
            .collect::<Vec<&str>>()
            .join("/")
            .as_str()
}

fn path_to_content_type(path: &Path) -> Option<String> {
    let ext = path.extension()?;

    Some(
        mime_guess::from_ext(&ext.to_string_lossy())
            .first_raw()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM.to_string().as_str())
            .to_owned(),
    )
}

fn compress_brotli(input: &[u8]) -> Option<Vec<u8>> {
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 22);
    writer.write_all(input).ok()?;

    Some(writer.into_inner())
}

fn literal_bytes(bytes: Vec<u8>) -> LitByteStr {
    LitByteStr::new(&bytes, Span::call_site())
}

// skip if compressed data is larger than the original
fn skip_larger(compressed: Vec<u8>, original: &[u8]) -> Vec<u8> {
    if compressed.len() >= original.len() {
        Default::default()
    } else {
        compressed
    }
}

pub(crate) fn list_assets<P: AsRef<Path>>(path: P) -> Vec<Asset> {
    let mut assets: Vec<Asset> = WalkDir::new(path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let Some(path) = entry.path().to_str() else {
                warn!("invalid file path {:?}", entry.path());
                return None;
            };

            let Ok(metadata) = entry.metadata() else {
                warn!(
                    "skipping file {:?}, could not get file metadata",
                    entry.path()
                );
                return None;
            };

            // skip directories
            if !metadata.is_file() {
                return None;
            };

            // skip empty
            if metadata.len() == 0 {
                warn!("skipping file {:?}: file empty", entry.path());
                return None;
            }

            let Some(content_type) = path_to_content_type(entry.path()) else {
                warn!(
                    "skipping file {:?}, could not determine file extension",
                    entry.path()
                );
                return None;
            };

            let route = path_to_route(entry.path());

            // do not load assets into the binary in debug / development mode
            if cfg!(debug_assertions) {
                return Some(Asset {
                    route,
                    path: path.to_owned(),
                    content_type,
                    etag: Default::default(),
                    bytes: literal_bytes(Default::default()),
                    brotli_bytes: literal_bytes(Default::default()),
                });
            }

            let Ok(bytes) = std::fs::read(entry.path()) else {
                warn!("skipping file {:?}: file is not readable", entry.path());
                return None;
            };

            let etag = sha256::digest(&bytes);

            let brotli_bytes = if COMPRESS_TYPES.contains(&content_type.as_str()) {
                compress_brotli(&bytes)
                    .map(|v| skip_larger(v, &bytes))
                    .unwrap_or_default()
            } else {
                Default::default()
            };

            if brotli_bytes.is_empty() {
                info!("including {:?} {} bytes", route, bytes.len());
            } else {
                info!(
                    "including {:?} {} -> {} bytes (compressed)",
                    route,
                    bytes.len(),
                    brotli_bytes.len()
                );
            };

            Some(Asset {
                route,
                path: path.to_owned(),
                content_type,
                etag,
                bytes: literal_bytes(if brotli_bytes.is_empty() {
                    bytes
                } else {
                    Default::default()
                }),
                brotli_bytes: literal_bytes(brotli_bytes),
            })
        })
        .collect();

    assets.sort();

    assets
}
