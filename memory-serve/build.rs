use memory_serve_core::assets_to_code;
use std::path::{Path, PathBuf};

const ASSET_FILE: &str = "memory_serve_assets.rs";
const ENV_NAME: &str = "ASSET_DIR";
const QUIET_ENV_NAME: &str = "MEMORY_SERVE_QUIET";

fn resolve_asset_dir(out_dir: &Path, key: &str, asset_dir: &str) -> PathBuf {
    let path = Path::new(&asset_dir);

    let path: PathBuf = if path.is_relative() {
        if let Ok(root_dir) = std::env::var("MEMORY_SERVE_ROOT") {
            let root_dir = Path::new(&root_dir);
            root_dir.join(path)
        } else {
            // assume the out dit is in the target directory
            let crate_root = out_dir
                .parent() // memory-serve
                .and_then(|p| p.parent()) // build
                .and_then(|p| p.parent()) // debug/release
                .and_then(|p| p.parent()) // target
                .and_then(|p| p.parent()) // crate root
                .expect("Unable to get crate root directory.");

            crate_root.join(path)
        }
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

    fn log(msg: &str) {
        if std::env::var(QUIET_ENV_NAME) != Ok("1".to_string()) {
            println!("cargo:warning={}", msg);
        }
    }

    // deternmine wheter to dynamically load assets or embed them in the binary
    let force_embed = std::env::var("CARGO_FEATURE_FORCE_EMBED").unwrap_or_default();
    let embed = !cfg!(debug_assertions) || force_embed == "1";

    // using a string is faster than using quote ;)
    let mut code = "&[".to_string();

    for (key, asset_dir) in std::env::vars() {
        if key.starts_with(ENV_NAME) {
            let name = key.trim_start_matches(format!("{ENV_NAME}_").as_str());
            let path = resolve_asset_dir(&out_dir, &key, &asset_dir);

            let assets = assets_to_code(&asset_dir, &path, embed, log);

            println!("cargo::rerun-if-changed={asset_dir}");

            code = format!("{code}(\"{name}\", {assets}),");
        }
    }

    code.push(']');

    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_FORCE_EMBED");
    println!("cargo::rerun-if-env-changed={ENV_NAME}");
    println!("cargo::rerun-if-env-changed={QUIET_ENV_NAME}");

    let target = out_dir.join(ASSET_FILE);
    std::fs::write(target, code).expect("Unable to write memory-serve asset file.");
}
