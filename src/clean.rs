//! Stripping terminal artifacts (ANSI escapes, backspace erasures, pagination
//! prompts) out of raw capture lines.

use once_cell::sync::Lazy;
use regex::Regex;

static MORE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*--\s*[Mm]ore\s*--\s*$").unwrap());
static ANSI_RE: Lazy<Regex> = Lazy::new(|| Regex::new("\x1b\\[[0-9;?]*[A-Za-z]").unwrap());

/// Strip terminal artifacts from a line, or return `None` if it's a
/// pagination prompt (e.g. "--More--") that should be dropped entirely.
pub fn clean_line(raw: &str) -> Option<String> {
    let no_ansi = ANSI_RE.replace_all(raw, "");
    let mut out = String::with_capacity(no_ansi.len());
    for ch in no_ansi.chars() {
        match ch {
            '\u{8}' => {
                out.pop();
            }
            '\r' => {}
            _ => out.push(ch),
        }
    }
    if MORE_RE.is_match(&out) {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_escapes() {
        assert_eq!(clean_line("\x1b[1mbold\x1b[0m text"), Some("bold text".to_string()));
    }

    #[test]
    fn applies_backspace_erasures() {
        assert_eq!(clean_line("abcd\u{8}\u{8}ef"), Some("abef".to_string()));
    }

    #[test]
    fn strips_carriage_returns() {
        assert_eq!(clean_line("line\r\n"), Some("line\n".to_string()));
    }

    #[test]
    fn drops_more_prompts() {
        assert_eq!(clean_line("  --More--  "), None);
    }
}
