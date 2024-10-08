use std::path::Path;

mod utils;

const ASSET_FILE: &str = "memory_serve_assets.bin";

fn main() {
    let out_dir: String = std::env::var("OUT_DIR").unwrap();

    let Ok(memory_serve_dir) = std::env::var("ASSET_DIR") else {
        panic!("Please specify the ASSET_DIR environment variable.");
    };

    let path = Path::new(&memory_serve_dir);
    let path = path.canonicalize().expect("Unable to canonicalize the path specified by ASSET_DIR.");

    if !path.exists() {
        panic!("The path {memory_serve_dir} specified by ASSET_DIR does not exists!");
    }

    let target = Path::new(&out_dir).join(ASSET_FILE);

    let assets = utils::list_assets(&path);
    let data = postcard::to_allocvec(&assets).expect("Unable to serialize memory-serve assets.");
    std::fs::write(target, data).expect("Unable to write memory-serve asset file.");

    println!("cargo::rerun-if-changed={memory_serve_dir}");
}