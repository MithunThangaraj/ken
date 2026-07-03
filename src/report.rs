//! The output model and its renderers.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::detect;
use crate::inspect;

/// A single file's inspection result.
#[derive(Serialize)]
pub struct Report {
    pub path: String,
    pub category: String,
    pub format: String,
    pub mime: String,
    pub size_bytes: u64,
    /// Format-specific extracted fields, in a stable insertion order.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub details: Map<String, Value>,
}

/// Reads a file, detects its type, and (unless `detect_only`) extracts details.
pub fn inspect_path(path: &Path, detect_only: bool) -> Result<Report> {
    let data = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let size = data.len() as u64;
    let det = detect::detect(&data);

    let mut details = Map::new();
    if !detect_only {
        if let Some(inspector) = inspect::inspector_for(det.format) {
            match inspector.inspect(&data) {
                Ok(map) => details = map,
                Err(err) => {
                    // Detection worked but parsing the body failed. Report it as
                    // a warning instead of failing the whole file.
                    details.insert(
                        "warning".into(),
                        Value::String(format!("extraction failed: {err:#}")),
                    );
                }
            }
        }
    }

    Ok(Report {
        path: path.display().to_string(),
        category: det.category.to_string(),
        format: det.format_name.to_string(),
        mime: det.mime.to_string(),
        size_bytes: size,
        details,
    })
}

impl Report {
    /// Prints the human-readable report to stdout.
    pub fn print_human(&self) {
        println!("{}", self.path);
        println!("  type     {} ({})", self.format, self.category);
        println!("  mime     {}", self.mime);
        println!("  size     {}", human_size(self.size_bytes));
        for (key, value) in &self.details {
            println!("  {:<8} {}", key, render_value(value));
        }
    }
}

/// Renders a JSON value for the human-readable report.
fn render_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "(none)".to_string(),
        Value::Array(items) => items
            .iter()
            .map(render_value)
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

/// Formats a byte count as a compact human-readable string.
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {} ({bytes} bytes)", UNITS[unit])
    }
}
