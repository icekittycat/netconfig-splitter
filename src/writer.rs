//! Writing finalized captures out to disk, either as
//! `output_dir/<hostname>/<command>/<yyyy-mm-dd>.txt` (dated, the default) or
//! `output_dir/<hostname>/<command>.txt` (flat, one file per command).

// pyo3's #[pyfunction]-generated wrapper re-converts an already-`PyResult`
// return type, which newer clippy flags as useless; the attribute doesn't
// reach the generated wrapper if placed on the function itself.
#![allow(clippy::useless_conversion)]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::paths::sanitize_path_component;

/// Write captures to disk.
///
/// By default, writes `output_dir/<hostname>/<command>/<yyyy-mm-dd>.txt`. If
/// `flat` is true, writes `output_dir/<hostname>/<command>.txt` instead, with
/// no per-date subfolder.
///
/// Either way, if the same host/command (and, in dated mode, date) occurs
/// more than once, later occurrences get a numeric suffix (`-2`, `-3`, ...)
/// so no data is lost.
#[pyfunction]
#[pyo3(signature = (captures, output_dir, flat=false))]
pub fn write_captures(
    captures: Vec<(String, String, String, String)>,
    output_dir: &str,
    flat: bool,
) -> PyResult<Vec<String>> {
    let out_root = Path::new(output_dir);
    let mut written: Vec<String> = Vec::new();
    let mut used_paths: HashSet<PathBuf> = HashSet::new();

    for (hostname, command, date_iso, text) in captures {
        let host_dir = out_root.join(sanitize_path_component(&hostname));
        let command = sanitize_path_component(&command);

        let (dir, base_name) = if flat {
            (host_dir.clone(), command)
        } else {
            (host_dir.join(&command), date_iso)
        };
        fs::create_dir_all(&dir).map_err(|e| PyIOError::new_err(e.to_string()))?;

        let mut suffix = 1u32;
        let mut path = dir.join(format!("{base_name}.txt"));
        while used_paths.contains(&path) {
            suffix += 1;
            path = dir.join(format!("{base_name}-{suffix}.txt"));
        }
        used_paths.insert(path.clone());

        fs::write(&path, &text).map_err(|e| PyIOError::new_err(e.to_string()))?;
        written.push(path.to_string_lossy().to_string());
    }

    Ok(written)
}
