//! JPEG inspector. Walks the marker segments to read frame dimensions and the
//! application markers (JFIF, Exif).

use anyhow::Result;
use serde_json::{Map, Value};

use super::{be_u16, Inspector};

pub struct Jpeg;

impl Inspector for Jpeg {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        let mut width = 0u16;
        let mut height = 0u16;
        let mut precision = 0u8;
        let mut components = 0u8;
        let mut progressive = false;
        let mut jfif = false;
        let mut exif = false;

        // Start just after the SOI marker (FF D8).
        let mut i = 2usize;
        while i + 4 <= data.len() {
            if data[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = data[i + 1];
            // Padding fill byte between markers.
            if marker == 0xFF {
                i += 1;
                continue;
            }
            // Standalone markers carry no length payload.
            if marker == 0xD8 || marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
                i += 2;
                continue;
            }
            let seg_len = match be_u16(data, i + 2) {
                Some(l) if l >= 2 => l as usize,
                _ => break,
            };
            let payload = i + 4;

            match marker {
                // APP0 is JFIF, APP1 is typically Exif.
                0xE0 => jfif = true,
                0xE1 => {
                    if data.get(payload..payload + 4) == Some(b"Exif") {
                        exif = true;
                    }
                }
                // Start-of-frame markers (excluding DHT 0xC4, JPG 0xC8, DAC 0xCC).
                0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC5 | 0xC6 | 0xC7 | 0xC9 | 0xCA | 0xCB | 0xCD
                | 0xCE | 0xCF => {
                    if let Some(p) = data.get(payload) {
                        precision = *p;
                    }
                    height = be_u16(data, payload + 1).unwrap_or(0);
                    width = be_u16(data, payload + 3).unwrap_or(0);
                    components = data.get(payload + 5).copied().unwrap_or(0);
                    progressive = marker == 0xC2;
                }
                // Start of scan: image data follows, so stop reading markers.
                0xDA => break,
                _ => {}
            }
            i = i + 2 + seg_len;
        }

        let mut m = Map::new();
        m.insert("width".into(), Value::from(width));
        m.insert("height".into(), Value::from(height));
        m.insert("depth".into(), Value::from(precision));
        m.insert("components".into(), Value::from(components));
        m.insert(
            "mode".into(),
            Value::from(if progressive { "progressive" } else { "baseline" }),
        );
        m.insert("jfif".into(), Value::from(jfif));
        m.insert("exif".into(), Value::from(exif));
        Ok(m)
    }
}
