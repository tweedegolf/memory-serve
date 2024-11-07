mod asset;
mod code;
mod list;
mod util;

pub use asset::Asset;
pub use code::assets_to_code;

/// File mime types that can possibly be compressed
pub const COMPRESS_TYPES: &[&str] = &[
    "text/html",
    "text/css",
    "application/json",
    "text/javascript",
    "application/javascript",
    "application/xml",
    "text/xml",
    "image/svg+xml",
    "application/wasm",
];
