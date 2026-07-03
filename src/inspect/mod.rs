//! Format-aware extraction.
//!
//! Each supported format implements [`Inspector`]. New formats are added by
//! writing a module here and wiring it into [`inspector_for`].

mod elf;
mod gif;
mod jpeg;
mod pdf;
mod png;
mod wav;
mod zip;

use anyhow::Result;
use serde_json::{Map, Value};

use crate::detect::Format;

/// Extracts structured metadata from a fully-loaded file.
pub trait Inspector {
    /// Returns an ordered map of format-specific fields.
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>>;
}

/// Returns the inspector for a detected format, if one exists.
pub fn inspector_for(format: Format) -> Option<Box<dyn Inspector>> {
    match format {
        Format::Png => Some(Box::new(png::Png)),
        Format::Jpeg => Some(Box::new(jpeg::Jpeg)),
        Format::Gif => Some(Box::new(gif::Gif)),
        Format::Pdf => Some(Box::new(pdf::Pdf)),
        Format::Zip => Some(Box::new(zip::Zip)),
        Format::Elf => Some(Box::new(elf::Elf)),
        Format::Wav => Some(Box::new(wav::Wav)),
        Format::Unknown => None,
    }
}

// ---- shared byte-reading helpers -------------------------------------------
// All return `None` on out-of-bounds rather than panicking, so inspectors can
// parse untrusted input safely with `?` / `ok_or`.

pub(crate) fn be_u16(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

pub(crate) fn be_u32(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn le_u16(data: &[u8], offset: usize) -> Option<u16> {
    data.get(offset..offset + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) fn le_u32(data: &[u8], offset: usize) -> Option<u32> {
    data.get(offset..offset + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
