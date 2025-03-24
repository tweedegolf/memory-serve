mod asset;
mod code;
mod list;
mod util;

use std::path::PathBuf;

pub use asset::Asset;
pub use code::assets_to_code;

/// File mime types that can possibly be compressed
pub const COMPRESS_TYPES: &[&str] = &[
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

const ASSET_FILE: &str = "memory_serve_assets.rs";
const QUIET_ENV_NAME: &str = "MEMORY_SERVE_QUIET";

pub fn load_directory<P: Into<PathBuf>>(path: P) {
    // determine whether to dynamically load assets or embed them in the binary
    let force_embed = std::env::var("CARGO_FEATURE_FORCE_EMBED").unwrap_or_default();
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_FORCE_EMBED");
    let embed = !cfg!(debug_assertions) || force_embed == "1";

    load_directory_with_embed(path, embed);
}

pub fn load_directory_with_embed<P: Into<PathBuf>>(path: P, embed: bool) {
    load_names_directories(vec![("ASSET_DIR", path)], embed);
}

pub fn load_names_directories<N, P>(named_paths: impl IntoIterator<Item = (N, P)>, embed: bool)
where
    N: Into<String>,
    P: Into<PathBuf>,
{
    let out_dir: String = std::env::var("OUT_DIR").expect("OUT_DIR environment variable not set.");
    let out_dir = PathBuf::from(&out_dir);

    fn log(msg: &str) {
        if std::env::var(QUIET_ENV_NAME) != Ok("1".to_string()) {
            println!("cargo:warning={}", msg);
        }
    }

    // using a string is faster than using quote ;)
    let mut code = "&[".to_string();

    for (name, asset_dir) in named_paths {
        let asset_dir = asset_dir
            .into()
            .canonicalize()
            .expect("Could not canonicalize the provided path");
        let asset_dir_label = asset_dir.to_string_lossy();
        let assets = assets_to_code(
            &asset_dir_label,
            &asset_dir,
            Some(out_dir.as_path()),
            embed,
            log,
        );

        println!("cargo::rerun-if-changed={asset_dir_label}");

        code = format!("{code}(\"{}\", {assets}),", name.into());
    }

    code.push(']');

    println!("cargo::rerun-if-env-changed={QUIET_ENV_NAME}");

    let target = out_dir.join(ASSET_FILE);

    std::fs::write(target, code).expect("Unable to write memory-serve asset file.");
}
