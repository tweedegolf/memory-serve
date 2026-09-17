use std::collections::HashMap;

/// Maps asset routes to the routes they are served on, see
/// [`MemoryServe::manifest`](crate::MemoryServe::manifest).
///
/// With hashed routes enabled, `/assets/index.css` maps to e.g.
/// `/assets/index.3f9a1c2b7d84e6a0.css`. HTML files, and all assets when
/// hashed routes are disabled, map to their plain route.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    routes: HashMap<&'static str, String>,
}

impl Manifest {
    pub(crate) fn new(routes: HashMap<&'static str, String>) -> Self {
        Self { routes }
    }

    /// The route an asset is served on, `None` for unknown routes.
    pub fn get(&self, route: &str) -> Option<&str> {
        self.routes.get(route).map(String::as_str)
    }

    /// Iterate over all `(route, served route)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        self.routes.iter().map(|(k, v)| (*k, v.as_str()))
    }

    /// The number of assets in the manifest.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether the manifest contains no assets.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Manifest;
    use std::collections::HashMap;

    #[test]
    fn lookup() {
        let manifest = Manifest::new(HashMap::from([
            ("/a.css", "/a.0123456789abcdef.css".to_string()),
            ("/index.html", "/index.html".to_string()),
        ]));

        assert_eq!(manifest.get("/a.css"), Some("/a.0123456789abcdef.css"));
        assert_eq!(manifest.get("/index.html"), Some("/index.html"));
        assert_eq!(manifest.get("/missing"), None);
        assert_eq!(manifest.len(), 2);
        assert!(!manifest.is_empty());
        assert!(Manifest::default().is_empty());

        let mut pairs: Vec<_> = manifest.iter().collect();
        pairs.sort();
        assert_eq!(
            pairs,
            [
                ("/a.css", "/a.0123456789abcdef.css"),
                ("/index.html", "/index.html")
            ]
        );
    }
}
