use proc_macro::TokenStream;
use tracing::warn;
use std::{env, path::Path};
use memory_serve_core::assets_to_code;

#[proc_macro]
pub fn load_assets(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let asset_dir = input.trim_matches('"');
    let mut path = Path::new(&asset_dir).to_path_buf();

    // skip if a subscriber is already registered (for instance by rust_analyzer)
    let _ = tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .try_init();

    fn log (msg: &str) {
        warn!("{msg}");
    }

    if path.is_relative() {
        if let Ok(root_dir) = env::var("MEMORY_SERVE_ROOT") {
            path = Path::new(&root_dir).join(path);
        } else if let Ok(crate_dir) = env::var("CARGO_MANIFEST_DIR") {
            path = Path::new(&crate_dir).join(path);
        } else {
            panic!("Relative path provided but CARGO_MANIFEST_DIR environment variable not set");
        }
    }

    path = path
        .canonicalize()
        .expect("Could not canonicalize the provided path");

    if !path.exists() {
        panic!("The path {path:?} does not exists!");
    }

    let embed = !cfg!(debug_assertions) || cfg!(feature = "force-embed");

    let assets = assets_to_code(asset_dir, &path, &path, embed, log);

    assets.parse().expect("Could not parse assets to code")
}
