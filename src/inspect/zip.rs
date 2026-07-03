//! ZIP inspector.
//!
//! Reads the End Of Central Directory (EOCD) record to get an authoritative
//! entry count, then walks the central directory for names and sizes. Also
//! recognizes common ZIP-based container formats from their member files.

use anyhow::Result;
use serde_json::{Map, Value};

use super::{le_u16, le_u32, Inspector};

const MAX_LISTED_NAMES: usize = 8;

pub struct Zip;

impl Inspector for Zip {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        let eocd =
            find_eocd(data).ok_or_else(|| anyhow::anyhow!("no end-of-central-directory record"))?;

        let total_entries = le_u16(data, eocd + 10).unwrap_or(0);
        let cd_offset = le_u32(data, eocd + 16).unwrap_or(0) as usize;

        let mut names: Vec<String> = Vec::new();
        let mut total_compressed = 0u64;
        let mut total_uncompressed = 0u64;
        let mut any_encrypted = false;

        // Walk central directory records (signature "PK\x01\x02").
        let mut i = cd_offset;
        while i + 46 <= data.len() && &data[i..i + 4] == b"PK\x01\x02" {
            let flags = le_u16(data, i + 8).unwrap_or(0);
            let compressed = le_u32(data, i + 20).unwrap_or(0);
            let uncompressed = le_u32(data, i + 24).unwrap_or(0);
            let name_len = le_u16(data, i + 28).unwrap_or(0) as usize;
            let extra_len = le_u16(data, i + 30).unwrap_or(0) as usize;
            let comment_len = le_u16(data, i + 32).unwrap_or(0) as usize;

            total_compressed += compressed as u64;
            total_uncompressed += uncompressed as u64;
            if flags & 0x0001 != 0 {
                any_encrypted = true;
            }

            let name_start = i + 46;
            if let Some(raw) = data.get(name_start..name_start + name_len) {
                names.push(String::from_utf8_lossy(raw).into_owned());
            }
            i = name_start + name_len + extra_len + comment_len;
        }

        let mut m = Map::new();
        m.insert("entries".into(), Value::from(total_entries));
        if let Some(kind) = container_kind(&names) {
            m.insert("format".into(), Value::from(kind));
        }
        m.insert("compressed".into(), Value::from(total_compressed));
        m.insert("uncompressed".into(), Value::from(total_uncompressed));
        if total_uncompressed > 0 {
            let ratio = 100.0 * (1.0 - total_compressed as f64 / total_uncompressed as f64);
            m.insert("saved".into(), Value::from(format!("{ratio:.0}%")));
        }
        m.insert("encrypted".into(), Value::from(any_encrypted));

        let shown: Vec<Value> = names
            .iter()
            .take(MAX_LISTED_NAMES)
            .cloned()
            .map(Value::from)
            .collect();
        m.insert("names".into(), Value::from(shown));
        Ok(m)
    }
}

/// Scans backward from the end of the file for the EOCD signature
/// ("PK\x05\x06"), tolerating a trailing comment.
fn find_eocd(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    // The comment can be up to 65535 bytes; bound the search accordingly.
    let earliest = data.len().saturating_sub(22 + 0xFFFF);
    let mut i = data.len() - 22;
    loop {
        if &data[i..i + 4] == b"PK\x05\x06" {
            return Some(i);
        }
        if i == 0 || i <= earliest {
            return None;
        }
        i -= 1;
    }
}

/// Recognizes well-known ZIP-based container formats from their members.
fn container_kind(names: &[String]) -> Option<&'static str> {
    let has = |target: &str| names.iter().any(|n| n == target);
    let has_prefix = |prefix: &str| names.iter().any(|n| n.starts_with(prefix));

    if has("[Content_Types].xml") {
        // OOXML: narrow it down by the part directory when possible.
        if has_prefix("word/") {
            return Some("DOCX (OOXML)");
        }
        if has_prefix("xl/") {
            return Some("XLSX (OOXML)");
        }
        if has_prefix("ppt/") {
            return Some("PPTX (OOXML)");
        }
        return Some("OOXML");
    }
    if has("mimetype") && has_prefix("META-INF/") && has_prefix("OEBPS") {
        return Some("EPUB");
    }
    if has("mimetype") {
        return Some("OpenDocument");
    }
    if has("META-INF/MANIFEST.MF") {
        return Some("JAR");
    }
    None
}
