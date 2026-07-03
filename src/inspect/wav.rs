//! WAV inspector. Walks the RIFF chunks to read the `fmt ` block and works out
//! the duration from the `data` chunk.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

use super::{le_u16, le_u32, Inspector};

pub struct Wav;

impl Inspector for Wav {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        if data.len() < 12 {
            bail!("truncated RIFF header");
        }

        let mut audio_format = 0u16;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut byte_rate = 0u32;
        let mut bits_per_sample = 0u16;
        let mut data_len = 0u32;

        // Chunks begin after the 12-byte RIFF/WAVE header.
        let mut i = 12usize;
        while i + 8 <= data.len() {
            let id = &data[i..i + 4];
            let size = le_u32(data, i + 4).unwrap_or(0) as usize;
            let body = i + 8;
            match id {
                b"fmt " => {
                    audio_format = le_u16(data, body).unwrap_or(0);
                    channels = le_u16(data, body + 2).unwrap_or(0);
                    sample_rate = le_u32(data, body + 4).unwrap_or(0);
                    byte_rate = le_u32(data, body + 8).unwrap_or(0);
                    bits_per_sample = le_u16(data, body + 14).unwrap_or(0);
                }
                b"data" => data_len = size as u32,
                _ => {}
            }
            // Chunks are word-aligned: odd sizes are followed by a pad byte.
            i = body + size + (size & 1);
        }

        let mut m = Map::new();
        m.insert("codec".into(), Value::from(format_name(audio_format)));
        m.insert("channels".into(), Value::from(channels));
        m.insert("rate".into(), Value::from(format!("{sample_rate} Hz")));
        m.insert(
            "depth".into(),
            Value::from(format!("{bits_per_sample}-bit")),
        );
        if byte_rate > 0 && data_len > 0 {
            let seconds = data_len as f64 / byte_rate as f64;
            m.insert("duration".into(), Value::from(format!("{seconds:.2} s")));
        }
        Ok(m)
    }
}

fn format_name(tag: u16) -> &'static str {
    match tag {
        0x0001 => "PCM",
        0x0003 => "IEEE float",
        0x0006 => "A-law",
        0x0007 => "µ-law",
        0xFFFE => "extensible",
        _ => "unknown",
    }
}
