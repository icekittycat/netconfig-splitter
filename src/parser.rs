//! The core line-by-line state machine that turns a raw capture file into a
//! list of per-command `PyCapture`s.

use std::collections::HashSet;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::capture::{Pending, PyCapture};
use crate::clean::clean_line;
use crate::date::{try_parse_date, CaptureDate, ParsedDate};
use crate::prompt::{normalize_command, PromptMatch, PromptStrategy};

/// Walks a capture file line by line, tracking the current device/date
/// context and accumulating output into finished `PyCapture`s.
struct Parser<'a> {
    year_hint: i32,
    normalize_commands: bool,
    prompt_strategy: &'a PromptStrategy,
    known_commands: HashSet<String>,
    current_date: CaptureDate,
    last_month: Option<u32>,
    pending: Option<Pending>,
    captures: Vec<PyCapture>,
}

impl<'a> Parser<'a> {
    fn new(
        default_year: i32,
        fallback_date: CaptureDate,
        normalize_commands: bool,
        prompt_strategy: &'a PromptStrategy,
        known_commands: HashSet<String>,
    ) -> Self {
        Self {
            year_hint: default_year,
            normalize_commands,
            prompt_strategy,
            known_commands,
            current_date: fallback_date,
            last_month: None,
            pending: None,
            captures: Vec::new(),
        }
    }

    fn feed_line(&mut self, raw_line: &str) {
        let Some(line) = clean_line(raw_line) else {
            return;
        };

        if let Some(parsed) = try_parse_date(&line, self.year_hint) {
            self.handle_date(parsed);
            return;
        }

        if let Some(prompt) = self.prompt_strategy.match_prompt(&line) {
            self.handle_prompt(prompt);
            return;
        }

        // Some scripted capture tools send a whole batch of commands at once
        // before reading any output back, so the terminal echoes queued-up
        // commands in the middle of an unrelated command's output. Such a
        // line has no prompt of its own, but exactly matches a command that
        // *is* typed at a real prompt somewhere else in the file.
        if self.known_commands.contains(line.trim()) {
            return;
        }

        // The same queued-echo artifact can also leave only the *tail* of a
        // command visible (e.g. "faces status" left over from "show ethernet
        // cfm interfaces status"), when a later overwrite erases the front of
        // the echoed text. Drop lines that are clearly just such a fragment.
        if is_stray_command_fragment(&line, &self.known_commands) {
            return;
        }

        if let Some(p) = self.pending.as_mut() {
            // Drop blank lines between the prompt header and the first real
            // content line, so there's no gap before the banner/output.
            if p.lines.is_empty() && line.trim().is_empty() {
                return;
            }
            p.lines.push(line);
        }
    }


    fn handle_date(&mut self, parsed: ParsedDate) {
        let mut date = parsed.date();
        if parsed.had_explicit_year() {
            // "show clock" output is the timestamp line itself, unlike XR's
            // auto-printed (year-less) per-command banner below.
            if let Some(p) = self.pending.as_mut() {
                p.lines.push(parsed.banner_text().to_string());
            }
        } else {
            if let Some(last_month) = self.last_month {
                // A large backward jump in an XR-style banner (which has no
                // year) means the capture crossed a Dec -> Jan year boundary.
                if last_month >= 2 && date.month < last_month - 1 {
                    self.year_hint += 1;
                    date.year = self.year_hint;
                }
            }
            // Keep the banner as a header line if it immediately follows the
            // prompt (XR's normal case), rather than mixing it into the output.
            if let Some(p) = self.pending.as_mut() {
                if p.banner_line.is_none() && p.lines.is_empty() {
                    p.banner_line = Some(parsed.banner_text().to_string());
                }
            }
        }
        self.last_month = Some(date.month);
        self.current_date = date;
    }

    fn handle_prompt(&mut self, prompt: PromptMatch) {
        if let Some(p) = self.pending.take() {
            self.finish_pending(p);
        }
        self.pending = if prompt.command.is_empty() {
            None
        } else {
            let command = if self.normalize_commands {
                normalize_command(&prompt.command)
            } else {
                prompt.command
            };
            Some(Pending::new(prompt.hostname, command, self.current_date, prompt.raw))
        };
    }

    fn finish_pending(&mut self, pending: Pending) {
        if pending.has_content() {
            self.captures.push(pending.finalize());
        }
    }

    fn finish(mut self) -> Vec<PyCapture> {
        if let Some(p) = self.pending.take() {
            self.finish_pending(p);
        }
        self.captures
    }
}

/// Pre-scan the capture file for every command actually typed at a device
/// prompt, so stray queued-command echoes (see `Parser::feed_line`) can be
/// recognized regardless of where in the file they show up.
fn collect_known_commands(text: &str, prompt_strategy: &PromptStrategy) -> HashSet<String> {
    let mut commands: HashSet<String> = HashSet::new();
    for raw_line in text.lines() {
        let Some(line) = clean_line(raw_line) else {
            continue;
        };
        if let Some(prompt) = prompt_strategy.match_prompt(&line) {
            if !prompt.command.is_empty() {
                commands.insert(prompt.command);
            }
        }
    }
    commands
}

/// Minimum length (in chars) for a bare line to be considered a leftover
/// fragment of a known command, rather than genuine (if short) output.
const MIN_FRAGMENT_LEN: usize = 6;

/// Whether `line` looks like the tail end of a known command left behind by
/// a queued-echo overwrite artifact (see `Parser::feed_line`), e.g. "faces
/// status" surviving from "show ethernet cfm interfaces status".
fn is_stray_command_fragment(line: &str, known_commands: &HashSet<String>) -> bool {
    let trimmed = line.trim();
    if trimmed.chars().count() < MIN_FRAGMENT_LEN {
        return false;
    }
    known_commands.iter().any(|cmd| cmd.ends_with(trimmed) && cmd != trimmed)
}

/// Parse a raw capture file into a list of per-command captures.
///
/// `vendor` restricts prompt recognition to one known vendor ("cisco" or
/// "juniper"), skipping the others. `prompt_regex` matches a fully
/// caller-supplied pattern instead (must have a named group `host`; `cmd`
/// is optional). At most one of `vendor`/`prompt_regex` may be given.
#[pyfunction]
#[pyo3(signature = (text, default_year, fallback_date, normalize_commands=true, vendor=None, prompt_regex=None))]
pub fn parse(
    text: &str,
    default_year: i32,
    fallback_date: &str,
    normalize_commands: bool,
    vendor: Option<&str>,
    prompt_regex: Option<&str>,
) -> PyResult<Vec<PyCapture>> {
    let fallback = CaptureDate::from_iso(fallback_date)
        .ok_or_else(|| PyValueError::new_err(format!("invalid fallback_date: {fallback_date}")))?;
    let prompt_strategy = PromptStrategy::from_options(vendor, prompt_regex).map_err(PyValueError::new_err)?;

    let known_commands = collect_known_commands(text, &prompt_strategy);
    let mut parser = Parser::new(default_year, fallback, normalize_commands, &prompt_strategy, known_commands);
    for raw_line in text.lines() {
        parser.feed_line(raw_line);
    }
    Ok(parser.finish())
}
