//! GIF inspector. Reads the logical screen descriptor and counts frames by
//! walking the block stream.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

use super::{le_u16, Inspector};

pub struct Gif;

impl Inspector for Gif {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        if data.len() < 13 {
            bail!("truncated header");
        }
        let version = std::str::from_utf8(&data[3..6]).unwrap_or("?");
        let width = le_u16(data, 6).unwrap_or(0);
        let height = le_u16(data, 8).unwrap_or(0);
        let packed = data[10];
        let has_gct = packed & 0x80 != 0;
        let gct_size = if has_gct { 2usize.pow((packed & 0x07) as u32 + 1) } else { 0 };

        // Skip the global color table if present: 3 bytes per entry.
        let mut i = 13usize + gct_size * 3;

        let mut frames = 0u32;
        let mut looping = false;

        while i < data.len() {
            match data[i] {
                // Image descriptor introduces one frame.
                0x2C => {
                    frames += 1;
                    // Descriptor is 10 bytes; bit 7 of the packed field (byte 9)
                    // signals a local color table to skip.
                    let packed_local = data.get(i + 9).copied().unwrap_or(0);
                    i += 10;
                    if packed_local & 0x80 != 0 {
                        let lct = 2usize.pow((packed_local & 0x07) as u32 + 1);
                        i += lct * 3;
                    }
                    // LZH minimum code size, then sub-blocks.
                    i += 1;
                    i = skip_sub_blocks(data, i);
                }
                // Extension block.
                0x21 => {
                    let label = data.get(i + 1).copied().unwrap_or(0);
                    // NETSCAPE2.0 application extension signals looping.
                    if label == 0xFF && data.get(i + 3..i + 14) == Some(b"NETSCAPE2.0") {
                        looping = true;
                    }
                    i += 2;
                    i = skip_sub_blocks(data, i);
                }
                // Trailer.
                0x3B => break,
                _ => break,
            }
        }

        let mut m = Map::new();
        m.insert("version".into(), Value::from(version));
        m.insert("width".into(), Value::from(width));
        m.insert("height".into(), Value::from(height));
        m.insert("frames".into(), Value::from(frames));
        m.insert("animated".into(), Value::from(frames > 1));
        m.insert("loops".into(), Value::from(looping));
        Ok(m)
    }
}

/// Advances past a GIF sub-block chain and returns the offset after the
/// terminating zero-length block.
fn skip_sub_blocks(data: &[u8], mut i: usize) -> usize {
    while i < data.len() {
        let block_size = data[i] as usize;
        if block_size == 0 {
            return i + 1;
        }
        i += 1 + block_size;
    }
    i
}
