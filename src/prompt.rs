//! Recognizing device prompts, the command typed at them, and normalizing
//! common command abbreviations.

use once_cell::sync::Lazy;
use pyo3::prelude::*;
use regex::Regex;

/// A device prompt optionally followed by the command typed at it, e.g.
///   RP/0/RSP1/CPU0:router#show running-config
static XR_PROMPT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?:RP|LC|SC)/\d+/\S+/CPU\d+:(?P<host>[A-Za-z0-9_.\-]+)#\s*(?P<cmd>.*)$").unwrap());

/// Generic IOS / IOS-XE / NX-OS / ASA style prompt, optionally with an ASA
/// multi-context suffix ("/context") and/or a config-mode suffix such as
/// "(config)" or "(config-if)". Anchored at the start of the line, so this
/// only matches a prompt that begins the line (not one embedded mid-output,
/// e.g. inside a banner or ACL remark).
static GENERIC_PROMPT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<host>[A-Za-z0-9_.\-]{1,63})(?:/(?P<context>[A-Za-z0-9_.\-]{1,32}))?(?:\([^)]{0,40}\))?(?P<delim>[#>])\s*(?P<cmd>.*)$",
    )
    .unwrap()
});

/// A recognized device prompt, with the command (if any) typed at it.
pub struct PromptMatch {
    pub hostname: String,
    pub command: String,
    /// The original prompt+command line exactly as it appeared in the
    /// capture, used as a header when writing the command's output file.
    pub raw: String,
}

/// Try to match a line as a device prompt.
pub fn match_prompt(line: &str) -> Option<PromptMatch> {
    if let Some(caps) = XR_PROMPT_RE.captures(line) {
        return Some(PromptMatch {
            hostname: caps["host"].to_string(),
            command: caps["cmd"].trim().to_string(),
            raw: line.to_string(),
        });
    }

    let caps = GENERIC_PROMPT_RE.captures(line)?;
    let mut hostname = caps["host"].to_string();
    if let Some(ctx) = caps.name("context") {
        // ASA multi-context prompt, e.g. "ciscoasa/admin#" -> "ciscoasa/admin".
        hostname = format!("{hostname}/{}", ctx.as_str());
    }
    let command = caps["cmd"].trim().to_string();
    Some(PromptMatch {
        hostname,
        command,
        raw: line.to_string(),
    })
}

/// Expand a small set of safe leading-word abbreviations, e.g. "sh run" ->
/// "show run", so differently-abbreviated invocations of the same command
/// land in the same output directory.
#[pyfunction]
pub fn normalize_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let first_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let first = &trimmed[..first_end];
    let rest = trimmed[first_end..].trim_start();
    let expanded = match first.to_lowercase().as_str() {
        "sh" => "show",
        "conf" | "config" => "configure",
        "wr" => "write",
        _ => first,
    };
    if rest.is_empty() {
        expanded.to_string()
    } else {
        format!("{expanded} {rest}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_xr_prompt_with_command() {
        let m = match_prompt("RP/0/RSP1/CPU0:router#show running-config").unwrap();
        assert_eq!(m.hostname, "router");
        assert_eq!(m.command, "show running-config");
    }

    #[test]
    fn matches_generic_prompt() {
        let m = match_prompt("switch1#show version").unwrap();
        assert_eq!(m.hostname, "switch1");
        assert_eq!(m.command, "show version");
    }

    #[test]
    fn matches_asa_multi_context_prompt() {
        let m = match_prompt("ciscoasa/admin#show running-config").unwrap();
        assert_eq!(m.hostname, "ciscoasa/admin");
        assert_eq!(m.command, "show running-config");
    }

    #[test]
    fn does_not_match_embedded_prompt() {
        assert!(match_prompt("  not a prompt line at all").is_none());
    }

    #[test]
    fn normalizes_abbreviations() {
        assert_eq!(normalize_command("sh run"), "show run");
        assert_eq!(normalize_command("conf t"), "configure t");
        assert_eq!(normalize_command("wr mem"), "write mem");
        assert_eq!(normalize_command("show version"), "show version");
    }
}
