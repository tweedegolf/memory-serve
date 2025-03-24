use memory_serve_core::{QUIET_ENV_NAME, ROOT_ENV_NAME, assets_to_code};
use proc_macro::TokenStream;
use std::{env, path::Path};

#[proc_macro]
pub fn load_assets(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let asset_dir = input.trim_matches('"');
    let mut path = Path::new(&asset_dir).to_path_buf();

    fn log(msg: &str) {
        if std::env::var(QUIET_ENV_NAME) != Ok("1".to_string()) {
            println!("  memory_serve: {msg}");
        }
    }

    if path.is_relative() {
        if let Ok(root_dir) = env::var(ROOT_ENV_NAME) {
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

    let assets = assets_to_code(asset_dir, &path, None, embed, log);

    assets.parse().expect("Could not parse assets to code")
}
