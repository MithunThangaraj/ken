//! Signature-based file-type detection.
//!
//! This only looks at the leading bytes of a file and never parses the full
//! structure. Format-aware parsing lives in the [`crate::inspect`] modules.

/// A format ken knows how to detect (and, usually, extract).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Png,
    Jpeg,
    Gif,
    Pdf,
    Zip,
    Elf,
    Wav,
    Unknown,
}

/// The result of detection: what the file is, plus how to describe it.
pub struct Detection {
    pub format: Format,
    /// Human-friendly format name, e.g. `"PNG image"`.
    pub format_name: &'static str,
    /// Broad category, e.g. `"image"`, `"archive"`, `"executable"`.
    pub category: &'static str,
    /// Best-guess MIME type.
    pub mime: &'static str,
}

/// Returns `true` if `data` begins with `sig`.
fn starts_with(data: &[u8], sig: &[u8]) -> bool {
    data.len() >= sig.len() && &data[..sig.len()] == sig
}

/// Detects the format of `data` from its leading bytes.
pub fn detect(data: &[u8]) -> Detection {
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if starts_with(data, &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Detection { format: Format::Png, format_name: "PNG image", category: "image", mime: "image/png" };
    }
    // JPEG: FF D8 FF
    if starts_with(data, &[0xFF, 0xD8, 0xFF]) {
        return Detection { format: Format::Jpeg, format_name: "JPEG image", category: "image", mime: "image/jpeg" };
    }
    // GIF: "GIF87a" or "GIF89a"
    if starts_with(data, b"GIF87a") || starts_with(data, b"GIF89a") {
        return Detection { format: Format::Gif, format_name: "GIF image", category: "image", mime: "image/gif" };
    }
    // PDF: "%PDF-"
    if starts_with(data, b"%PDF-") {
        return Detection { format: Format::Pdf, format_name: "PDF document", category: "document", mime: "application/pdf" };
    }
    // ELF: 7F 45 4C 46
    if starts_with(data, &[0x7F, b'E', b'L', b'F']) {
        return Detection { format: Format::Elf, format_name: "ELF binary", category: "executable", mime: "application/x-elf" };
    }
    // WAV: "RIFF" .... "WAVE"
    if starts_with(data, b"RIFF") && data.len() >= 12 && &data[8..12] == b"WAVE" {
        return Detection { format: Format::Wav, format_name: "WAV audio", category: "audio", mime: "audio/wav" };
    }
    // ZIP: local file header "PK\x03\x04", empty archive "PK\x05\x06",
    // or spanned archive "PK\x07\x08".
    if starts_with(data, b"PK\x03\x04") || starts_with(data, b"PK\x05\x06") || starts_with(data, b"PK\x07\x08") {
        return Detection { format: Format::Zip, format_name: "ZIP archive", category: "archive", mime: "application/zip" };
    }

    Detection { format: Format::Unknown, format_name: "unknown", category: "unknown", mime: "application/octet-stream" }
}
