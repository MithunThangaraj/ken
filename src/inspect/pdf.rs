//! PDF inspector.
//!
//! A complete PDF parse would resolve the cross-reference table and object
//! streams. This inspector reads the header version and scans the raw bytes for
//! a handful of signals instead: object count, page count, encryption, and
//! linearization. The counts are estimates rather than exact figures.

use anyhow::Result;
use serde_json::{Map, Value};

use super::Inspector;

pub struct Pdf;

impl Inspector for Pdf {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        // Header: "%PDF-x.y".
        let version = data
            .get(5..8)
            .and_then(|b| std::str::from_utf8(b).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "?".to_string());

        let objects = count_occurrences(data, b" obj");
        // Page objects appear as `/Type /Page` or, very commonly, `/Type/Page`
        // with no space. Count both spellings and subtract the `/Pages` tree
        // nodes, which share the prefix. (Object streams can hide pages from a
        // raw byte scan; this is a best-effort estimate, not a full parse.)
        let page_nodes =
            count_occurrences(data, b"/Type /Page") + count_occurrences(data, b"/Type/Page");
        let pages_nodes =
            count_occurrences(data, b"/Type /Pages") + count_occurrences(data, b"/Type/Pages");
        let pages = page_nodes.saturating_sub(pages_nodes);
        let encrypted = contains(data, b"/Encrypt");
        // A linearized PDF advertises "/Linearized" in its first object.
        let linearized = contains(&data[..data.len().min(2048)], b"/Linearized");

        let mut m = Map::new();
        m.insert("version".into(), Value::from(version));
        m.insert("objects".into(), Value::from(objects as u64));
        if pages > 0 {
            m.insert("pages".into(), Value::from(pages as u64));
        }
        m.insert("encrypted".into(), Value::from(encrypted));
        m.insert("linearized".into(), Value::from(linearized));
        Ok(m)
    }
}

fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    count_occurrences(haystack, needle) > 0
}
