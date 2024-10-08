use mime_guess::mime;
use std::{io::Write, path::Path};
use tracing::{info, warn};
use walkdir::WalkDir;

/// Internal data structure
pub struct Asset {
    pub route: String,
    pub path: String,
    pub etag: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub is_compressed: bool,
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

const COMPRESS_TYPES: &[&str] = &[
    "text/html",
    "text/css",
    "application/json",
    "text/javascript",
    "application/javascript",
    "application/xml",
    "text/xml",
    "image/svg+xml",
    "application/wasm",
];

fn path_to_route(base: &Path, path: &Path) -> String {
    let relative_path = path
        .strip_prefix(base)
        .expect("Could not strap prefix from path");

    let route = relative_path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<&str>>()
        .join("/");

    format!("/{route}")
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

// skip if compressed data is larger than the original
fn skip_larger(compressed: Vec<u8>, original: &[u8]) -> Vec<u8> {
    if compressed.len() >= original.len() {
        Default::default()
    } else {
        compressed
    }
}

pub fn list_assets(base_path: &Path) -> Vec<Asset> {
    let mut assets: Vec<Asset> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let Some(path) = entry.path().to_str() else {
                warn!("invalid file path {:?}", entry.path());
                return None;
            };

            let route = path_to_route(base_path, entry.path());

            let Ok(metadata) = entry.metadata() else {
                warn!("skipping file {route}, could not get file metadata");
                return None;
            };

            // skip directories
            if !metadata.is_file() {
                return None;
            };

            // skip empty
            if metadata.len() == 0 {
                warn!("skipping file {route}: file empty");
                return None;
            }

            let Some(content_type) = path_to_content_type(entry.path()) else {
                warn!("skipping file {route}, could not determine file extension");
                return None;
            };

            // do not load assets into the binary in debug / development mode
            if cfg!(debug_assertions) {
                return Some(Asset {
                    route,
                    path: path.to_owned(),
                    content_type,
                    etag: Default::default(),
                    bytes: Default::default(),
                    is_compressed: false,
                });
            }

            let Ok(bytes) = std::fs::read(entry.path()) else {
                warn!("skipping file {route}: file is not readable");
                return None;
            };

            let etag = sha256::digest(&bytes);
            let original_size = bytes.len();

            let (bytes, is_compressed) = if COMPRESS_TYPES.contains(&content_type.as_str()) {
                let brotli_bytes = compress_brotli(&bytes)
                    .map(|v| skip_larger(v, &bytes))
                    .unwrap_or_default();

                (brotli_bytes, true)
            } else {
                (bytes, false)
            };

            if is_compressed {
                info!(
                    "including {route} {original_size} -> {} bytes (compressed)",
                    bytes.len()
                );
            } else {
                info!("including {route} {} bytes", bytes.len());
            };

            Some(Asset {
                route,
                path: path.to_owned(),
                content_type,
                etag,
                bytes,
                is_compressed,
            })
        })
        .collect();

    assets.sort();

    assets
}

const ASSET_FILE: &str = "memory_serve_assets.rs";

fn main() {
    let out_dir: String = std::env::var("OUT_DIR").unwrap();
    let pkg_name: String = std::env::var("CARGO_PKG_NAME").unwrap();

    let memory_serve_dir = match std::env::var("ASSET_DIR") {
        Ok(dir) => dir,
        Err(_) if pkg_name == "memory-serve" => "../static".to_string(),
        Err(_) => {
            panic!("Please specify the ASSET_DIR environment variable.");
        }
    };

    let path = Path::new(&memory_serve_dir);
    let path = path
        .canonicalize()
        .expect("Unable to canonicalize the path specified by ASSET_DIR.");

    if !path.exists() {
        panic!("The path {memory_serve_dir} specified by ASSET_DIR does not exists!");
    }

    let target = Path::new(&out_dir).join(ASSET_FILE);
    let assets = list_assets(&path);

    let route = assets.iter().map(|a| &a.route);
    let path = assets.iter().map(|a| &a.path);
    let content_type = assets.iter().map(|a| &a.content_type);
    let etag = assets.iter().map(|a| &a.etag);
    let is_compressed = assets.iter().map(|a| &a.is_compressed);
    let bytes = assets.iter().map(|a| {
        let file_name = Path::new(&a.path).file_name().unwrap().to_str().unwrap();
        let target = Path::new(&out_dir).join(file_name);
        std::fs::write(&target, &a.bytes).expect("Unable to write file to out dir.");

        target.to_str().unwrap().to_string()
    });

    let code = quote::quote! {
        &[
            #(Asset {
                route: #route,
                path: #path,
                content_type: #content_type,
                etag: #etag,
                bytes: include_bytes!(#bytes),
                is_compressed: #is_compressed,
            }),*
        ]
    };

    std::fs::write(target, code.to_string()).expect("Unable to write memory-serve asset file.");

    println!("cargo::rerun-if-changed={memory_serve_dir}");
}
