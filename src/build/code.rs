use std::path::Path;

use fs_err as fs;

use crate::util::path::normalize_path;

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

/// Render a path as a Rust string literal, e.g. `C:\project\static` becomes
/// `"C:\\project\\static"`.
///
/// `Path`'s `Debug` implementation produces a correctly escaped and quoted
/// literal, which `include_bytes!` and the `path` field both accept. Do not
/// wrap the result in a raw string: the escaped backslashes would then be
/// taken literally, so Windows paths could not be read back at runtime.
fn path_literal(path: &Path) -> String {
    format!("{:?}", normalize_path(path))
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

            format!("Some(include_bytes!({}))", path_literal(&file_path))
        } else {
            format!("Some(include_bytes!({}))", path_literal(&path))
        };

        code.push_str(&format!(
            "
            memory_serve::Asset {{
                route: r\"{route}\",
                path: {},
                content_type: \"{content_type}\",
                etag: \"{etag}\",
                bytes: {bytes},
                is_compressed: {is_compressed},
                should_compress: {should_compress},
            }},",
            path_literal(&path)
        ));
    }

    code.push(']');

    code
}

#[cfg(test)]
mod tests {
    use super::{generate_code, path_literal};
    use fs_err as fs;
    use std::path::{Path, PathBuf};

    fn log(_: &str) {}

    /// Reverse the escaping that `Path`'s `Debug` impl applies to the
    /// characters that can appear in a path literal.
    fn unescape_literal(literal: &str) -> String {
        assert!(
            literal.starts_with('"') && literal.ends_with('"'),
            "not a string literal: {literal}"
        );

        let mut result = String::new();
        let mut chars = literal[1..literal.len() - 1].chars();

        while let Some(c) = chars.next() {
            if c != '\\' {
                result.push(c);
                continue;
            }

            match chars.next().expect("dangling escape") {
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                other => panic!("unexpected escape: \\{other}"),
            }
        }

        result
    }

    /// The literal that follows `marker` in the generated code, up to the end
    /// of the field.
    fn literal_after<'s>(code: &'s str, marker: &str) -> &'s str {
        let (_, rest) = code
            .split_once(marker)
            .unwrap_or_else(|| panic!("{marker} not found in {code}"));

        rest.split(",\n").next().expect("end of field")
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("memory-serve-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");

        dir
    }

    /// A directory with a single `index.html`, compressed when embedded
    fn asset_dir(name: &str) -> (PathBuf, PathBuf) {
        let dir = make_temp_dir(name);
        let index = dir.join("index.html");
        fs::write(&index, "<html>hello</html>".repeat(20)).expect("write asset");

        (dir, index)
    }

    /// Dynamic assets must use a plain string literal. Wrapping the escaped
    /// path in a raw string (as was done before) doubles the backslashes, so
    /// the file could not be found at request time on Windows.
    #[test]
    fn dynamic_assets_use_a_plain_string_literal() {
        let (dir, index) = asset_dir("dynamic-literal");
        let out_dir = make_temp_dir("dynamic-literal-out");

        let code = generate_code(&dir.to_string_lossy(), &dir, &out_dir, false, log, log);

        assert!(code.contains("bytes: None"), "{code}");
        assert!(
            !code.contains("path: r"),
            "no raw string for the path: {code}"
        );

        let literal = literal_after(&code, "path: ");

        assert_eq!(literal, path_literal(&index));
        assert_eq!(Path::new(&unescape_literal(literal)), index);

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&out_dir);
    }

    /// Embedded assets `include_bytes!` a plain string literal as well
    #[test]
    fn embedded_assets_use_a_plain_include_bytes_literal() {
        let (dir, index) = asset_dir("embedded-literal");
        let out_dir = make_temp_dir("embedded-literal-out");

        let code = generate_code(&dir.to_string_lossy(), &dir, &out_dir, true, log, log);

        let literal = literal_after(&code, "Some(include_bytes!(");
        let literal = literal.trim_end_matches(')');
        let included = PathBuf::from(unescape_literal(literal));

        assert!(included.is_file(), "{} should exist", included.display());
        // embedded paths are still absolute, and the source file is either
        // included directly or replaced by a compressed copy in `out_dir`
        assert!(included.is_absolute());
        assert!(included.starts_with(&index) || included.starts_with(&out_dir));

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&out_dir);
    }

    /// The path field is emitted as a Rust literal, not as source text that
    /// happens to look like one: it roundtrips through `Path`.
    #[test]
    fn path_literal_roundtrips() {
        for path in ["C:\\project\\static\\index.html", "some/relative/file.css"] {
            let literal = path_literal(Path::new(path));

            assert_eq!(unescape_literal(&literal), path);
        }
    }
}
