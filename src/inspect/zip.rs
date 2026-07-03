//! ZIP inspector.
//!
//! Reads the End Of Central Directory (EOCD) record for the entry count, then
//! walks the central directory for names and sizes. When the archive is really
//! an Office Open XML document (DOCX, XLSX, PPTX), it goes a step further and
//! reads the hidden `docProps` metadata: author, who last saved it, how many
//! times it has been revised, and the created and modified times.

use anyhow::Result;
use serde_json::{Map, Value};

use super::{le_u16, le_u32, Inspector};

const MAX_LISTED_NAMES: usize = 8;

/// One central-directory record, enough to locate and read the entry's data.
struct Entry {
    name: String,
    method: u16,
    compressed: u32,
    uncompressed: u32,
    local_offset: u32,
    encrypted: bool,
}

pub struct Zip;

impl Inspector for Zip {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        let eocd =
            find_eocd(data).ok_or_else(|| anyhow::anyhow!("no end-of-central-directory record"))?;

        let total_entries = le_u16(data, eocd + 10).unwrap_or(0);
        let cd_offset = le_u32(data, eocd + 16).unwrap_or(0) as usize;
        let entries = read_central_directory(data, cd_offset);

        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        let total_compressed: u64 = entries.iter().map(|e| e.compressed as u64).sum();
        let total_uncompressed: u64 = entries.iter().map(|e| e.uncompressed as u64).sum();
        let any_encrypted = entries.iter().any(|e| e.encrypted);

        let mut m = Map::new();
        m.insert("entries".into(), Value::from(total_entries));
        if let Some(kind) = container_kind(&names) {
            m.insert("format".into(), Value::from(kind));
        }

        // The part people actually care about: Office document metadata.
        ooxml_metadata(data, &entries, &mut m);

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

/// Walks central-directory records ("PK\x01\x02") starting at `cd_offset`.
fn read_central_directory(data: &[u8], cd_offset: usize) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut i = cd_offset;
    while i + 46 <= data.len() && &data[i..i + 4] == b"PK\x01\x02" {
        let flags = le_u16(data, i + 8).unwrap_or(0);
        let method = le_u16(data, i + 10).unwrap_or(0);
        let compressed = le_u32(data, i + 20).unwrap_or(0);
        let uncompressed = le_u32(data, i + 24).unwrap_or(0);
        let name_len = le_u16(data, i + 28).unwrap_or(0) as usize;
        let extra_len = le_u16(data, i + 30).unwrap_or(0) as usize;
        let comment_len = le_u16(data, i + 32).unwrap_or(0) as usize;
        let local_offset = le_u32(data, i + 42).unwrap_or(0);

        let name_start = i + 46;
        let name = data
            .get(name_start..name_start + name_len)
            .map(|raw| String::from_utf8_lossy(raw).into_owned())
            .unwrap_or_default();

        entries.push(Entry {
            name,
            method,
            compressed,
            uncompressed,
            local_offset,
            encrypted: flags & 0x0001 != 0,
        });
        i = name_start + name_len + extra_len + comment_len;
    }
    entries
}

/// Reads and, if needed, inflates one entry's bytes from its local header.
fn read_entry(data: &[u8], entry: &Entry) -> Option<Vec<u8>> {
    if entry.encrypted {
        return None;
    }
    let lo = entry.local_offset as usize;
    if data.get(lo..lo + 4)? != b"PK\x03\x04" {
        return None;
    }
    // The local header repeats the name and extra lengths, which can differ
    // from the central-directory copy, so the data offset is computed here.
    let name_len = le_u16(data, lo + 26)? as usize;
    let extra_len = le_u16(data, lo + 28)? as usize;
    let start = lo + 30 + name_len + extra_len;
    let end = start.checked_add(entry.compressed as usize)?;
    let raw = data.get(start..end)?;

    match entry.method {
        0 => Some(raw.to_vec()),                                // stored
        8 => miniz_oxide::inflate::decompress_to_vec(raw).ok(), // raw deflate
        _ => None,
    }
}

/// Finds an entry by exact name and returns its decompressed bytes.
fn read_named(data: &[u8], entries: &[Entry], name: &str) -> Option<Vec<u8>> {
    let entry = entries.iter().find(|e| e.name == name)?;
    read_entry(data, entry)
}

/// Pulls Office document metadata out of `docProps/core.xml` and `app.xml`.
fn ooxml_metadata(data: &[u8], entries: &[Entry], m: &mut Map<String, Value>) {
    if let Some(bytes) = read_named(data, entries, "docProps/core.xml") {
        let xml = String::from_utf8_lossy(&bytes);
        put(m, "title", xml_text(&xml, "dc:title"));
        put(m, "author", xml_text(&xml, "dc:creator"));
        put(m, "last_by", xml_text(&xml, "cp:lastModifiedBy"));
        put(m, "revision", xml_text(&xml, "cp:revision"));
        put(m, "created", xml_text(&xml, "dcterms:created"));
        put(m, "modified", xml_text(&xml, "dcterms:modified"));
    }
    if let Some(bytes) = read_named(data, entries, "docProps/app.xml") {
        let xml = String::from_utf8_lossy(&bytes);
        put(m, "app", xml_text(&xml, "Application"));
        if let Some(minutes) = xml_text(&xml, "TotalTime") {
            m.insert("edit_time".into(), Value::from(format!("{minutes} min")));
        }
        put(m, "company", xml_text(&xml, "Company"));
        put(m, "pages", xml_text(&xml, "Pages"));
        put(m, "words", xml_text(&xml, "Words"));
    }
}

/// Inserts `key` only when the value is present and non-empty.
fn put(m: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        if !v.is_empty() {
            m.insert(key.into(), Value::from(v));
        }
    }
}

/// Returns the text content of the first `<tag>...</tag>` in `xml`.
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let open_at = xml.find(&open)?;
    // Skip to the end of the opening tag, past any attributes.
    let text_start = open_at + xml[open_at..].find('>')? + 1;
    let close = format!("</{tag}>");
    let close_at = text_start + xml[text_start..].find(&close)?;
    let text = xml[text_start..close_at].trim();
    if text.is_empty() {
        None
    } else {
        Some(unescape(text))
    }
}

/// Expands the five predefined XML entities.
fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Scans backward from the end of the file for the EOCD signature
/// ("PK\x05\x06"), tolerating a trailing comment.
fn find_eocd(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    // The comment can be up to 65535 bytes, so bound the search accordingly.
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
