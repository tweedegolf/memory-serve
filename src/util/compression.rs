use std::io::Write;

/// Decompress a byte slice using brotli.
pub(crate) fn decompress_brotli(input: &[u8]) -> Option<Vec<u8>> {
    let mut writer = brotli::DecompressorWriter::new(Vec::new(), 1024);
    writer.write_all(input).ok()?;

    writer.into_inner().ok()
}

/// Compress a byte slice using brotli.
pub(crate) fn compress_brotli(input: &[u8]) -> Option<Vec<u8>> {
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 22);
    writer.write_all(input).ok()?;

    Some(writer.into_inner())
}

/// Compress a byte slice using gzip.
pub(crate) fn compress_gzip(input: &[u8]) -> Option<Vec<u8>> {
    let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    writer.write_all(input).ok()?;

    writer.finish().ok()
}
