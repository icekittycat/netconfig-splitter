//! Rust core for netconfig-splitter, exposed to Python via pyo3 as
//! `netconfig_splitter._core`.
//!
//! Parses combined IOS/IOS-XE/IOS-XR/NX-OS/ASA CLI capture files by looking
//! for device prompts (to figure out which device/command each block of
//! output belongs to) and timestamp banners (to figure out which date to
//! stamp it with), then writes the results out to
//! `output_dir/<hostname>/<command>.txt` by default (opt into
//! `output_dir/<hostname>/<command>/<yyyy-mm-dd>.txt` with `flat=False`).

mod capture;
mod clean;
mod date;
mod parser;
mod paths;
mod prompt;
mod writer;

use pyo3::prelude::*;

use capture::PyCapture;
use parser::parse;
use paths::sanitize_path_component;
use prompt::normalize_command;
use writer::write_captures;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCapture>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(write_captures, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_command, m)?)?;
    m.add_function(wrap_pyfunction!(sanitize_path_component, m)?)?;
    Ok(())
}

