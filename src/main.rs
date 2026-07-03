//! ken: a universal file inspector.
//!
//! Detects a file's type from its signature and, for supported formats, reads
//! out its metadata. Output is human-readable by default; `--json` produces a
//! machine-readable document.

mod detect;
mod inspect;
mod report;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "ken",
    version,
    about = "Universal file inspector: detect types and read out structured metadata"
)]
struct Cli {
    /// File(s) to inspect.
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Emit machine-readable JSON instead of the human-readable report.
    #[arg(long)]
    json: bool,

    /// Only detect the file type; skip format-specific extraction.
    #[arg(long)]
    detect_only: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let mut had_error = false;
    let mut json_reports: Vec<Value> = Vec::new();

    for (idx, path) in cli.files.iter().enumerate() {
        match report::inspect_path(path, cli.detect_only) {
            Ok(report) => {
                if cli.json {
                    json_reports.push(serde_json::to_value(&report).expect("report is serializable"));
                } else {
                    if idx > 0 {
                        println!();
                    }
                    report.print_human();
                }
            }
            Err(err) => {
                had_error = true;
                if cli.json {
                    json_reports.push(serde_json::json!({
                        "path": path.display().to_string(),
                        "error": format!("{err:#}"),
                    }));
                } else {
                    eprintln!("ken: {}: {err:#}", path.display());
                }
            }
        }
    }

    if cli.json {
        // A single file yields a bare object; multiple files yield an array.
        let out = if json_reports.len() == 1 {
            json_reports.pop().unwrap()
        } else {
            Value::Array(json_reports)
        };
        println!("{}", serde_json::to_string_pretty(&out).expect("json serializable"));
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
