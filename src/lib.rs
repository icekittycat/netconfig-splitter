//! Rust core for netconfig-splitter, exposed to Python via pyo3 as
//! `netconfig_splitter._core`.
//!
//! Parses combined IOS/IOS-XE/IOS-XR/NX-OS/ASA CLI capture files by looking
//! for device prompts (to figure out which device/command each block of
//! output belongs to) and timestamp banners (to figure out which date to
//! stamp it with), then writes the results out to
//! `output_dir/<hostname>/<command>/<yyyy-mm-dd>.txt`.
//!
//! The actual prompt/timestamp/cleanup/path-sanitization primitives live in
//! the [asterust](https://github.com/asterust/asterust) crate; this crate
//! wires them together into a stateful line-by-line parser and adds the
//! PyO3 bindings.

mod capture;
mod parser;
mod writer;

use pyo3::prelude::*;

use capture::PyCapture;
use parser::parse;
use writer::write_captures;

/// Expand a small set of safe leading-word abbreviations, e.g. "sh run" ->
/// "show run", so differently-abbreviated invocations of the same command
/// land in the same output directory.
#[pyfunction]
fn normalize_command(command: &str) -> String {
    asterust::prompt::normalize_command(command)
}

/// Make a string safe to use as a single path segment.
#[pyfunction]
fn sanitize_path_component(name: &str) -> String {
    asterust::paths::sanitize_path_component(name)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCapture>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(write_captures, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_command, m)?)?;
    m.add_function(wrap_pyfunction!(sanitize_path_component, m)?)?;
    Ok(())
}

