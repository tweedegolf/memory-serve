use proc_macro::TokenStream;
use std::{env, path::Path};
use utils::list_assets;

mod asset;
mod utils;

use crate::asset::Asset;

#[proc_macro]
pub fn load_assets(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let input = input.trim_matches('"');
    let mut asset_path = Path::new(&input).to_path_buf();

    // skip if a subscriber is already registered (for instance by rust_analyzer)
    let _ = tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .try_init();

    if asset_path.is_relative() {
        let crate_dir = env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR environment variable not set");
        asset_path = Path::new(&crate_dir).join(asset_path);
    }

    asset_path = asset_path
        .canonicalize()
        .expect("Could not canonicalize the provided path");

    if !asset_path.exists() {
        panic!("The path {:?} does not exists!", asset_path);
    }

    let files: Vec<Asset> = list_assets(&asset_path);

    let route = files.iter().map(|a| &a.route);
    let path = files.iter().map(|a| &a.path);
    let content_type = files.iter().map(|a| &a.content_type);
    let etag = files.iter().map(|a| &a.etag);
    let bytes = files.iter().map(|a| &a.bytes);
    let brotli_bytes = files.iter().map(|a| &a.brotli_bytes);

    quote::quote! {
        &[
            #(memory_serve::Asset {
                route: #route,
                path: #path,
                content_type: #content_type,
                etag: #etag,
                bytes: #bytes,
                brotli_bytes: #brotli_bytes,
            }),*
        ]
    }
    .into()
}

#[proc_macro]
pub fn load_assets_from_out_dir(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let input = input.trim_matches('"');
    let asset_path = Path::new(&input).to_path_buf();

    // skip if a subscriber is already registered (for instance by rust_analyzer)
    let _ = tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .try_init();

    if !asset_path.is_relative() {
        panic!("The asset path is supposed to be relative to the OUT_DIR");
    }

    let mut asset_path = Path::new(&env::var("OUT_DIR").expect("OUT_DIR environment variable not set")).join(asset_path);

    asset_path = asset_path
        .canonicalize()
        .expect("Could not canonicalize the provided path");

    if !asset_path.exists() {
        panic!("The path {:?} does not exists!", asset_path);
    }

    let files: Vec<Asset> = list_assets(&asset_path);

    let route = files.iter().map(|a| &a.route);
    let path = files.iter().map(|a| &a.path);
    let content_type = files.iter().map(|a| &a.content_type);
    let etag = files.iter().map(|a| &a.etag);
    let bytes = files.iter().map(|a| &a.bytes);
    let brotli_bytes = files.iter().map(|a| &a.brotli_bytes);

    quote::quote! {
        &[
            #(memory_serve::Asset {
                route: #route,
                path: #path,
                content_type: #content_type,
                etag: #etag,
                bytes: #bytes,
                brotli_bytes: #brotli_bytes,
            }),*
        ]
    }
    .into()
}
