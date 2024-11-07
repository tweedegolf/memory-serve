
mod asset;
mod list;
mod code;
mod util;

pub use asset::Asset;
pub use code::assets_to_code;

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