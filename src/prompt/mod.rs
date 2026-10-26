//! Recognizing device prompts and the command typed at them, across vendors.

pub mod cisco;
pub mod juniper;

use pyo3::prelude::*;

/// A recognized device prompt, with the command (if any) typed at it.
pub struct PromptMatch {
    pub hostname: String,
    pub command: String,
    /// The original prompt+command line exactly as it appeared in the
    /// capture, used as a header when writing the command's output file.
    pub raw: String,
}

/// Try to match a line as a device prompt, trying each known vendor's
/// pattern in turn. A capture file may contain devices from more than one
/// vendor, so callers aren't expected to pick a vendor up front.
pub fn match_prompt(line: &str) -> Option<PromptMatch> {
    cisco::match_prompt(line).or_else(|| juniper::match_prompt(line))
}

/// Expand a small set of safe leading-word abbreviations, e.g. "sh run" ->
/// "show run", so differently-abbreviated invocations of the same command
/// land in the same output directory.
///
/// These are Cisco IOS abbreviations specifically; for other vendors this
/// is a harmless no-op passthrough (the leading word just won't match any
/// of the known abbreviations).
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
    fn dispatches_to_cisco_xr_prompt() {
        let m = match_prompt("RP/0/RSP1/CPU0:router#show running-config").unwrap();
        assert_eq!(m.hostname, "router");
    }

    #[test]
    fn dispatches_to_juniper_prompt() {
        let m = match_prompt("admin@router1> show configuration").unwrap();
        assert_eq!(m.hostname, "router1");
    }

    #[test]
    fn normalizes_abbreviations() {
        assert_eq!(normalize_command("sh run"), "show run");
        assert_eq!(normalize_command("conf t"), "configure t");
        assert_eq!(normalize_command("wr mem"), "write mem");
        assert_eq!(normalize_command("show version"), "show version");
    }
}
