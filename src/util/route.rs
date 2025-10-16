use mime_guess::mime;
use std::path::Path;

const ALLOWED_CHARS: [(&str, &str); 19] = [
    ("%2F", "/"),
    ("%5C", "\\"),
    ("%21", "!"),
    ("%2A", "*"),
    ("%27", "'"),
    ("%28", "("),
    ("%29", ")"),
    ("%3B", ";"),
    ("%3A", ":"),
    ("%40", "@"),
    ("%26", "&"),
    ("%3D", "="),
    ("%2B", "+"),
    ("%24", "$"),
    ("%2C", ","),
    ("%3F", "?"),
    ("%25", "%"),
    ("%5B", "["),
    ("%5D", "]"),
];

/// Convert a path to a (HTTP) path / route
pub(crate) fn path_to_route(base: &Path, path: &Path) -> String {
    let relative_path = path
        .strip_prefix(base)
        .expect("Could not strip prefix from path");

    let route = relative_path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<&str>>()
        .join("/");

    let mut route: String = urlencoding::encode(&route).to_string();

    for (from, to) in ALLOWED_CHARS {
        route = route.replace(from, to);
    }

    format!("/{route}")
}

/// Determine the mime type of a file
pub(crate) fn path_to_content_type(path: &Path) -> Option<String> {
    let ext = path.extension()?;

    Some(
        mime_guess::from_ext(&ext.to_string_lossy())
            .first_raw()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM.to_string().as_str())
            .to_owned(),
    )
}

#[cfg(test)]
mod test {
    use super::path_to_route;

    #[test]
    fn test_path_to_route() {
        let base = std::path::Path::new("/");
        let path = std::path::Path::new(
            "/assets/stars:wow !@%^&*()ama{zi}ng💩! * ' ( ) ; : @ & = + $ , ? % [ ] \\.svg",
        );

        assert_eq!(
            path_to_route(base, path),
            "/assets/stars:wow%20!@%%5E&*()ama%7Bzi%7Dng%F0%9F%92%A9!%20*%20'%20(%20)%20;%20:%20@%20&%20=%20+%20$%20,%20?%20%%20[%20]%20\\.svg"
        );
    }
}
