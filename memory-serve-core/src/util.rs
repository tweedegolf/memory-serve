use std::{io::Write, path::Path};

use mime_guess::mime;

pub(crate) fn path_to_route(base: &Path, path: &Path) -> String {
    let relative_path = path
        .strip_prefix(base)
        .expect("Could not strap prefix from path");

    let route = relative_path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<&str>>()
        .join("/");

    format!("/{route}")
}

pub(crate) fn path_to_content_type(path: &Path) -> Option<String> {
    let ext = path.extension()?;

    Some(
        mime_guess::from_ext(&ext.to_string_lossy())
            .first_raw()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM.to_string().as_str())
            .to_owned(),
    )
}

pub(crate) fn compress_brotli(input: &[u8]) -> Option<Vec<u8>> {
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 22);
    writer.write_all(input).ok()?;

    Some(writer.into_inner())
}
