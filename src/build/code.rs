use std::path::Path;

use fs_err as fs;

use super::{file_asset::FileAsset, list::list_assets};

/// Generate code with metadata and contents for the assets
pub fn assets_to_code(
    asset_dir: &str,
    path: &Path,
    out_dir: &Path,
    embed: bool,
    log: fn(&str),
) -> String {
    generate_code(asset_dir, path, out_dir, embed, log, log)
}

/// Generate code for the assets, logging a summary and skipped files with
/// `log` and each included file with `log_file`
pub(super) fn generate_code(
    asset_dir: &str,
    path: &Path,
    out_dir: &Path,
    embed: bool,
    log: fn(&str),
    log_file: fn(&str),
) -> String {
    log(&format!("Loading static assets from {asset_dir}"));

    let assets = list_assets(path, embed, log, log_file);

    if embed {
        log(&format!("Embedding {} assets into binary", assets.len()));
    } else {
        log(&format!(
            "Not embedding {} assets into binary, assets will load dynamically",
            assets.len()
        ));
    }

    // using a string is faster than using quote ;)
    let mut code = "&[".to_string();

    for asset in assets {
        let FileAsset {
            route,
            path,
            etag,
            content_type,
            compressed_bytes,
            should_compress,
        } = asset;

        let is_compressed = compressed_bytes.is_some();

        let bytes = if !embed {
            "None".to_string()
        } else if let Some(compressed_bytes) = compressed_bytes {
            let file_path = out_dir.join(&etag);
            fs::write(&file_path, compressed_bytes).expect("Unable to write file to out dir.");

            format!("Some(include_bytes!(r\"{}\"))", file_path.to_string_lossy())
        } else {
            format!("Some(include_bytes!(r\"{}\"))", path.to_string_lossy())
        };

        code.push_str(&format!(
            "
            memory_serve::Asset {{
                route: r\"{route}\",
                path: r{path:?},
                content_type: \"{content_type}\",
                etag: \"{etag}\",
                bytes: {bytes},
                is_compressed: {is_compressed},
                should_compress: {should_compress},
            }},"
        ));
    }

    code.push(']');

    code
}
