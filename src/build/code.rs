use std::path::Path;

use super::{file_asset::FileAsset, list::list_assets};

/// Generate code with metadata and contents for the assets
pub(super) fn assets_to_code(
    asset_dir: &str,
    path: &Path,
    out_dir: &Path,
    embed: bool,
    log: fn(&str),
) -> String {
    log(&format!("Loading static assets from {asset_dir}"));

    if embed {
        log("Embedding assets into binary");
    } else {
        log("Not embedding assets into binary, assets will load dynamically");
    }

    let assets = list_assets(path, embed, log);

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
            std::fs::write(&file_path, compressed_bytes).expect("Unable to write file to out dir.");

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
