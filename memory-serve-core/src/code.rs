use std::path::Path;

use crate::{asset::Asset, list::list_assets};

/// Generate code with metadata and contents for the assets
pub fn assets_to_code(
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
            memory_serve::Asset {{
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

    code
}
