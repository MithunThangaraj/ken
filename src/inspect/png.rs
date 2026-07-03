//! PNG inspector. Parses the IHDR header and walks the chunk stream.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

use super::{be_u32, Inspector};

pub struct Png;

impl Inspector for Png {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        // The 8-byte signature is followed immediately by the IHDR chunk:
        // [len:4][type:4][data:13][crc:4]. IHDR data starts at offset 16.
        if data.len() < 33 || &data[12..16] != b"IHDR" {
            bail!("missing IHDR chunk");
        }
        let width = be_u32(data, 16).unwrap_or(0);
        let height = be_u32(data, 20).unwrap_or(0);
        let bit_depth = data[24];
        let color_type = data[25];
        let interlace = data[28];

        let mut m = Map::new();
        m.insert("width".into(), Value::from(width));
        m.insert("height".into(), Value::from(height));
        m.insert("depth".into(), Value::from(bit_depth));
        m.insert("color".into(), Value::from(color_type_name(color_type)));
        m.insert("interlace".into(), Value::from(interlace != 0));

        // Walk chunks starting at the IHDR length field (offset 8).
        let mut offset = 8usize;
        let mut chunk_count = 0u32;
        let mut has_text = false;
        let mut animated = false;
        let mut frames: Option<u32> = None;

        while offset + 8 <= data.len() {
            let len = be_u32(data, offset).unwrap_or(0) as usize;
            let ctype = &data[offset + 4..offset + 8];
            chunk_count += 1;
            match ctype {
                b"tEXt" | b"iTXt" | b"zTXt" => has_text = true,
                b"acTL" => {
                    animated = true;
                    // acTL data: num_frames (4), num_plays (4).
                    frames = be_u32(data, offset + 8);
                }
                b"IEND" => break,
                _ => {}
            }
            // Advance past len(4) + type(4) + data(len) + crc(4).
            offset = offset.saturating_add(12).saturating_add(len);
        }

        m.insert("chunks".into(), Value::from(chunk_count));
        m.insert("text".into(), Value::from(has_text));
        m.insert("animated".into(), Value::from(animated));
        if let Some(f) = frames {
            m.insert("frames".into(), Value::from(f));
        }
        Ok(m)
    }
}

fn color_type_name(color_type: u8) -> &'static str {
    match color_type {
        0 => "grayscale",
        2 => "RGB",
        3 => "palette",
        4 => "grayscale+alpha",
        6 => "RGBA",
        _ => "unknown",
    }
}
