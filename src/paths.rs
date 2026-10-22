//! Making arbitrary strings safe to use as filesystem path segments.

use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;

static ILLEGAL_PATH_CHARS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[\\/:*?"<>|\x00-\x1f]"#).unwrap());

/// Make a string safe to use as a single path segment.
#[pyfunction]
pub fn sanitize_path_component(name: &str) -> String {
    let cleaned = ILLEGAL_PATH_CHARS_RE.replace_all(name, "_");
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_illegal_characters() {
        assert_eq!(sanitize_path_component("a/b:c*d"), "a_b_c_d");
    }

    #[test]
    fn falls_back_to_underscore_for_empty_result() {
        assert_eq!(sanitize_path_component("..."), "_");
        assert_eq!(sanitize_path_component(""), "_");
    }

    #[test]
    fn leaves_normal_names_untouched() {
        assert_eq!(sanitize_path_component("router1"), "router1");
    }
}
