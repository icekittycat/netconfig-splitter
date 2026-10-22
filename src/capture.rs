//! Per-command captured output: `Pending` accumulates lines while the parser
//! walks the file, then finalizes into the Python-facing `PyCapture`.

use pyo3::prelude::*;

use crate::date::CaptureDate;

#[pyclass]
pub struct PyCapture {
    #[pyo3(get)]
    pub hostname: String,
    #[pyo3(get)]
    pub command: String,
    #[pyo3(get)]
    pub date: String,
    #[pyo3(get)]
    pub text: String,
}

/// A capture still accumulating output lines as the parser walks the file.
pub struct Pending {
    pub hostname: String,
    pub command: String,
    pub date: CaptureDate,
    /// The original prompt+command line, kept as the first header line.
    pub prompt_line: String,
    /// The raw per-command timestamp banner (e.g. IOS-XR's), if one appeared
    /// immediately after the prompt, kept as the second header line.
    pub banner_line: Option<String>,
    pub lines: Vec<String>,
}

impl Pending {
    pub fn new(hostname: String, command: String, date: CaptureDate, prompt_line: String) -> Self {
        Self {
            hostname,
            command,
            date,
            prompt_line,
            banner_line: None,
            lines: Vec::new(),
        }
    }

    /// Whether any accumulated line has real (non-whitespace) content, as
    /// opposed to e.g. "terminal length 0" which produces no output at all.
    pub fn has_content(&self) -> bool {
        self.lines.iter().any(|l| !l.trim().is_empty())
    }

    pub fn finalize(self) -> PyCapture {
        let mut header = self.prompt_line;
        if let Some(banner) = self.banner_line {
            header.push('\n');
            header.push_str(&banner);
        }
        let body = self.lines.join("\n");
        let body = body.trim_end_matches('\n');
        let text = if body.is_empty() {
            format!("{header}\n")
        } else {
            format!("{header}\n{body}\n")
        };
        PyCapture {
            hostname: self.hostname,
            command: self.command,
            date: self.date.to_iso(),
            text,
        }
    }
}
