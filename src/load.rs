#[allow(unused)]
use crate as memory_serve;

/// Include the generated asset manifest and construct a `MemoryServe` struct.
#[macro_export]
macro_rules! load {
    () => {{
        use memory_serve::{Asset, MemoryServe};

        let assets: &[(&str, &[Asset])] =
            include!(concat!(env!("OUT_DIR"), "/memory_serve_assets.rs"));

        if assets.is_empty() {
            panic!("No assets found, did you call a load_directory* function from your build.rs?");
        }

        MemoryServe::new(assets[0].1)
    }};
    ($title:expr) => {{
        use memory_serve::{Asset, MemoryServe};

        let assets: &[(&str, &[Asset])] =
            include!(concat!(env!("OUT_DIR"), "/memory_serve_assets.rs"));

        let selected_assets = assets
            .into_iter()
            .find(|(n, _)| *n == $title)
            .map(|(_, a)| *a)
            .unwrap_or_default();

        if selected_assets.is_empty() {
            panic!("No assets found, did you call a load_directory* function from your build.rs?");
        }

        MemoryServe::new(selected_assets)
    }};
}
