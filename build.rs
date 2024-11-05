use mime_guess::mime;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

macro_rules! log {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

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

pub fn list_assets(base_path: &Path, embed: bool) -> Vec<Asset> {
    let mut assets: Vec<Asset> = WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path().to_owned();
            let route = path_to_route(base_path, entry.path());

            let Ok(metadata) = entry.metadata() else {
                log!("skipping file {route}, could not get file metadata");
                return None;
            };

            // skip directories
            if !metadata.is_file() {
                return None;
            };

            // skip empty
            if metadata.len() == 0 {
                log!("skipping file {route}: file empty");
                return None;
            }

            let Some(content_type) = path_to_content_type(entry.path()) else {
                log!("skipping file {route}, could not determine file extension");
                return None;
            };

            // do not load assets into the binary in debug / development mode
            if !embed{
                log!("including {route} (dynamically)");

                return Some(Asset {
                    route,
                    path: path.to_owned(),
                    content_type,
                    etag: Default::default(),
                    compressed_bytes: None,
                });
            }

            let Ok(bytes) = std::fs::read(entry.path()) else {
                log!("skipping file {route}: file is not readable");
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
                        log!("including {route} {original_size} bytes (compression unnecessary)");
                    }
                    Some(brotli_bytes) => {
                        log!(
                            "including {route} {original_size} -> {} bytes (compressed)",
                            brotli_bytes.len()
                        );

                        asset.compressed_bytes = Some(brotli_bytes);
                    }
                    None => {
                        log!("including {route} {original_size} bytes (compression failed)");
                    }
                }
            } else {
                log!("including {route} {original_size} bytes");
            }

            Some(asset)
        })
        .collect();

    assets.sort();

    assets
}

const ASSET_FILE: &str = "memory_serve_assets";
const ENV_NAME: &str = "ASSET_DIR";

fn include_directory(asset_dir: &str, path: &Path, out_dir: &Path, embed: bool, name: &str) {
    log!("Loading static assets from {asset_dir}");
    let assets = list_assets(&path, embed);

    // using a string is faster than using quote ;)
    let mut code = "&[".to_string();

    for asset in assets {
        let Asset {
            route,
            path,
            etag,
            content_type,
            compressed_bytes,
        } = asset;

        let bytes = if !embed {
            "None".to_string()
        } else if let Some(compressed_bytes) = &compressed_bytes {
            let file_name = path.file_name().expect("Unable to get file name.");
            let file_path = Path::new(&out_dir).join(file_name);
            std::fs::write(&file_path, compressed_bytes).expect("Unable to write file to out dir.");

            format!("Some(include_bytes!(\"{}\"))", file_path.to_string_lossy())
        } else {
            format!("Some(include_bytes!(\"{}\"))", path.to_string_lossy())
        };

        let is_compressed = compressed_bytes.is_some();

        code.push_str(&format!(
            "
            Asset {{
                route: \"{route}\",
                path: {path:?},
                content_type: \"{content_type}\",
                etag: \"{etag}\",
                bytes: {bytes},
                is_compressed: {is_compressed},
            }},"
        ));
    }

    code.push(']');

    log!("NAME {name}");


    let target = if name == ENV_NAME  {
        Path::new(&out_dir).join(format!("{ASSET_FILE}.rs"))
    } else {
        Path::new(&out_dir).join(format!("{ASSET_FILE}_{name}.rs"))
    };

    std::fs::write(target, code).expect("Unable to write memory-serve asset file.");

    println!("cargo::rerun-if-changed={asset_dir}");
    println!("cargo::rerun-if-env-changed={name}");
}


fn resolve_asset_dir(out_dir: &Path, key: &str, asset_dir: &str) -> PathBuf {
    let path = Path::new(&asset_dir);

    let path: PathBuf = if path.is_relative() {
        // assume the out dit is in the target directory
        let crate_root = out_dir
            .parent() // memory-serve
            .and_then(|p| p.parent()) // build
            .and_then(|p| p.parent()) // debug/release
            .and_then(|p| p.parent()) // target
            .and_then(|p| p.parent()) // crate root
            .expect("Unable to get crate root directory.");

        crate_root.join(path)
    } else {
        path.to_path_buf()
    };

    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(e) => panic!("The path {path:?} specified by {key} is not a valid path: {e}"),
    };

    if !path.exists() {
        panic!("The path {path:?} specified by {key} does not exists!");
    }

    path
}

fn main() {
    let out_dir: String = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set.");
    let out_dir = PathBuf::from(&out_dir);

    // deternmine wheter to dynamically load assets or embed them in the binary
    let force_embed = std::env::var("CARGO_FEATURE_FORCE_EMBED").unwrap_or_default();
    let embed = !cfg!(debug_assertions) || force_embed == "1";

    if embed {
        log!("Embedding assets in binary");
    } else {
        log!("Dynamically loading assets");
    }

    let mut found = false;

    for (key, asset_dir) in std::env::vars() {
        if key.starts_with(ENV_NAME) {
            let name = key.trim_start_matches(format!("{ENV_NAME}_").as_str());
            let path = resolve_asset_dir(&out_dir, &key, &asset_dir);
            
            include_directory(&asset_dir, &path, &out_dir, embed, name);

            found = true;
        }
    }

    if !found {
        let target = Path::new(&out_dir).join(format!("{ASSET_FILE}.rs"));
        log!("Please specify the `{ENV_NAME}` environment variable.");
        std::fs::write(target, "&[]").expect("Unable to write memory-serve asset file.");
        println!("cargo::rerun-if-env-changed=ASSET_DIR");
        return;
    };
}
