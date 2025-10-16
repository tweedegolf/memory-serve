use std::path::PathBuf;

mod code;
mod file_asset;
mod list;

const ASSET_FILE: &str = "memory_serve_assets.rs";
const QUIET_ENV_NAME: &str = "MEMORY_SERVE_QUIET";

/// Load a directory of assets, keeping an administration of all files
/// and optionally embedding them into the binary
pub fn load_directory<P: Into<PathBuf>>(path: P) {
    // determine whether to dynamically load assets or embed them in the binary
    let embed = !cfg!(debug_assertions) || cfg!(feature = "force-embed");

    load_directory_with_embed(path, embed);
}

/// Load a directory of assets, optionally embedding them into the binary
pub fn load_directory_with_embed<P: Into<PathBuf>>(path: P, embed: bool) {
    load_names_directories(vec![("default", path)], embed);
}

/// Load multiple named directories of assets, optionally embedding them into the binary
pub fn load_names_directories<N, P>(named_paths: impl IntoIterator<Item = (N, P)>, embed: bool)
where
    N: Into<String>,
    P: Into<PathBuf>,
{
    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .expect("OUT_DIR environment variable not set, make sure you call this from a build.rs")
        .into();

    println!("cargo::rerun-if-env-changed={QUIET_ENV_NAME}");
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
        let assets = code::assets_to_code(&asset_dir_label, &asset_dir, &out_dir, embed, log);

        println!("cargo::rerun-if-changed={asset_dir_label}");

        code = format!("{code}(\"{}\", {assets}),", name.into());
    }

    code.push(']');

    let target = out_dir.join(ASSET_FILE);

    std::fs::write(target, code).expect("Unable to write memory-serve asset file.");
}

#[cfg(test)]
/// Load assets directly from disk for use in integration tests.
pub(super) fn load_test_assets<P: Into<PathBuf>>(path: P) -> &'static [crate::Asset] {
    fn log(msg: &str) {
        println!("{}", msg);
    }

    let embed = !cfg!(debug_assertions) || cfg!(feature = "force-embed");
    let assets = list::list_assets(&path.into(), embed, log);

    let assets = assets
        .into_iter()
        .map(|fa| crate::Asset {
            route: fa.route.leak(),
            is_compressed: fa.compressed_bytes.is_some(),
            path: fa.path.to_string_lossy().to_string().leak(),
            etag: fa.etag.leak(),
            content_type: fa.content_type.leak(),
            bytes: fa.compressed_bytes.map(|v| {
                let s: &'static [u8] = v.leak();

                s
            }),
            should_compress: fa.should_compress,
        })
        .collect::<Vec<_>>();

    Box::leak(assets.into_boxed_slice())
}
