//! Cisco-style prompts: bare `hostname#`/`hostname>`, IOS-XR's
//! `RP/.../CPU0:hostname#` variant, and ASA multi-context prompts.

use once_cell::sync::Lazy;
use regex::Regex;

use super::PromptMatch;

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

/// Try to match a line as a Cisco-style device prompt.
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
    fn does_not_match_juniper_user_at_host_prompt() {
        assert!(match_prompt("admin@router1> show configuration").is_none());
    }
}
