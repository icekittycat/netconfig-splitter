//! Juniper-style prompts: `user@hostname>` (operational mode) and
//! `user@hostname#` (configuration mode).

use once_cell::sync::Lazy;
use regex::Regex;

use super::PromptMatch;

/// A JunOS prompt, e.g. "admin@router1> show configuration" or
/// "root@switch1# show interfaces". The username is discarded; only the
/// hostname matters for grouping output.
static JUNIPER_PROMPT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9_.\-]+@(?P<host>[A-Za-z0-9_.\-]+)[>#]\s*(?P<cmd>.*)$").unwrap());

/// Try to match a line as a JunOS-style device prompt.
pub fn match_prompt(line: &str) -> Option<PromptMatch> {
    let caps = JUNIPER_PROMPT_RE.captures(line)?;
    Some(PromptMatch {
        hostname: caps["host"].to_string(),
        command: caps["cmd"].trim().to_string(),
        raw: line.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_operational_mode_prompt() {
        let m = match_prompt("admin@router1> show configuration").unwrap();
        assert_eq!(m.hostname, "router1");
        assert_eq!(m.command, "show configuration");
    }

    #[test]
    fn matches_configuration_mode_prompt() {
        let m = match_prompt("root@switch1# show interfaces").unwrap();
        assert_eq!(m.hostname, "switch1");
        assert_eq!(m.command, "show interfaces");
    }

    #[test]
    fn matches_prompt_with_no_command() {
        let m = match_prompt("admin@router1>").unwrap();
        assert_eq!(m.hostname, "router1");
        assert_eq!(m.command, "");
    }

    #[test]
    fn does_not_match_cisco_prompt() {
        assert!(match_prompt("switch1#show version").is_none());
    }
}
