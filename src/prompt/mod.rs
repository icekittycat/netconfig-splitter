//! Recognizing device prompts and the command typed at them, across vendors.

pub mod cisco;
pub mod juniper;

use regex::Regex;

use pyo3::prelude::*;

/// A recognized device prompt, with the command (if any) typed at it.
pub struct PromptMatch {
    pub hostname: String,
    pub command: String,
    /// The original prompt+command line exactly as it appeared in the
    /// capture, used as a header when writing the command's output file.
    pub raw: String,
}

/// A known, built-in vendor prompt format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vendor {
    Cisco,
    Juniper,
}

impl Vendor {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "cisco" => Some(Vendor::Cisco),
            "juniper" => Some(Vendor::Juniper),
            _ => None,
        }
    }
}

/// How to recognize a device prompt: try every known vendor, restrict to
/// one specific vendor, or match a fully caller-supplied regex instead.
pub enum PromptStrategy {
    /// Try each known vendor's pattern in turn (the default). A capture
    /// file may contain devices from more than one vendor, so callers
    /// aren't expected to pick a vendor up front.
    All,
    /// Only try this vendor's pattern, skipping the others entirely.
    Vendor(Vendor),
    /// Match this regex instead of any built-in pattern, for syntaxes not
    /// yet supported. Must have a named capture group `host`; a `cmd`
    /// group is optional (an empty command is treated as a bare prompt).
    Custom(Regex),
}

impl PromptStrategy {
    /// Build a strategy from the CLI-facing `vendor`/`prompt_regex`
    /// options. Passing both is rejected as ambiguous; passing neither
    /// gives [`PromptStrategy::All`]. Pure Rust (no pyo3 types) so this
    /// can be unit-tested directly - the `#[pyfunction]` boundary in
    /// `parser.rs` is responsible for turning the `Err` string into a
    /// `PyValueError`.
    pub fn from_options(vendor: Option<&str>, prompt_regex: Option<&str>) -> Result<Self, String> {
        match (vendor, prompt_regex) {
            (Some(_), Some(_)) => Err("vendor and prompt_regex are mutually exclusive - pass at most one".to_string()),
            (None, Some(pattern)) => {
                let re = Regex::new(pattern).map_err(|e| format!("invalid prompt_regex: {e}"))?;
                if re.capture_names().flatten().all(|n| n != "host") {
                    return Err("prompt_regex must have a named capture group `host`".to_string());
                }
                Ok(PromptStrategy::Custom(re))
            }
            (Some(name), None) => Vendor::parse(name)
                .map(PromptStrategy::Vendor)
                .ok_or_else(|| format!("unknown vendor: {name} (expected cisco or juniper)")),
            (None, None) => Ok(PromptStrategy::All),
        }
    }

    pub fn match_prompt(&self, line: &str) -> Option<PromptMatch> {
        match self {
            PromptStrategy::All => match_prompt(line),
            PromptStrategy::Vendor(Vendor::Cisco) => cisco::match_prompt(line),
            PromptStrategy::Vendor(Vendor::Juniper) => juniper::match_prompt(line),
            PromptStrategy::Custom(re) => {
                let caps = re.captures(line)?;
                Some(PromptMatch {
                    hostname: caps.name("host")?.as_str().to_string(),
                    command: caps.name("cmd").map(|m| m.as_str().trim().to_string()).unwrap_or_default(),
                    raw: line.to_string(),
                })
            }
        }
    }
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
    fn vendor_strategy_restricts_to_one_vendor() {
        let strategy = PromptStrategy::from_options(Some("juniper"), None).unwrap();
        assert!(strategy.match_prompt("RP/0/RSP1/CPU0:router#show run").is_none());
        assert!(strategy.match_prompt("admin@router1> show configuration").is_some());
    }

    #[test]
    fn custom_regex_strategy_matches_caller_pattern() {
        let strategy = PromptStrategy::from_options(None, Some(r"^(?P<host>\w+)\$\s*(?P<cmd>.*)$")).unwrap();
        let m = strategy.match_prompt("myhost$ show version").unwrap();
        assert_eq!(m.hostname, "myhost");
        assert_eq!(m.command, "show version");
    }

    #[test]
    fn rejects_both_vendor_and_custom_regex() {
        assert!(PromptStrategy::from_options(Some("cisco"), Some(r"^(?P<host>\w+)\$")).is_err());
    }

    #[test]
    fn rejects_unknown_vendor() {
        assert!(PromptStrategy::from_options(Some("huawei"), None).is_err());
    }

    #[test]
    fn rejects_custom_regex_without_host_group() {
        assert!(PromptStrategy::from_options(None, Some(r"^\w+\$")).is_err());
    }

    #[test]
    fn normalizes_abbreviations() {
        assert_eq!(normalize_command("sh run"), "show run");
        assert_eq!(normalize_command("conf t"), "configure t");
        assert_eq!(normalize_command("wr mem"), "write mem");
        assert_eq!(normalize_command("show version"), "show version");
    }
}
