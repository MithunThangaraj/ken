//! ELF inspector. Decodes the identification bytes and the file header.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

use super::Inspector;

pub struct Elf;

impl Inspector for Elf {
    fn inspect(&self, data: &[u8]) -> Result<Map<String, Value>> {
        // e_ident is 16 bytes; the header proper needs at least 20 more.
        if data.len() < 20 {
            bail!("truncated ELF header");
        }

        let class = match data[4] {
            1 => "32-bit",
            2 => "64-bit",
            _ => "unknown",
        };
        let little_endian = data[5] == 1;
        let endianness = if little_endian { "little" } else { "big" };
        let abi = abi_name(data[7]);

        // e_type and e_machine are 2-byte fields at offsets 16 and 18, encoded
        // in the file's own endianness.
        let read_u16 = |offset: usize| -> u16 {
            let b = [data[offset], data[offset + 1]];
            if little_endian {
                u16::from_le_bytes(b)
            } else {
                u16::from_be_bytes(b)
            }
        };
        let e_type = read_u16(16);
        let e_machine = read_u16(18);

        let mut m = Map::new();
        m.insert("class".into(), Value::from(class));
        m.insert("endian".into(), Value::from(endianness));
        m.insert("abi".into(), Value::from(abi));
        m.insert("kind".into(), Value::from(type_name(e_type)));
        m.insert("arch".into(), Value::from(machine_name(e_machine)));
        Ok(m)
    }
}

fn abi_name(v: u8) -> &'static str {
    match v {
        0 => "System V",
        3 => "Linux",
        9 => "FreeBSD",
        6 => "Solaris",
        12 => "OpenBSD",
        _ => "other",
    }
}

fn type_name(v: u16) -> &'static str {
    match v {
        1 => "relocatable",
        2 => "executable",
        3 => "shared object",
        4 => "core dump",
        _ => "unknown",
    }
}

fn machine_name(v: u16) -> &'static str {
    match v {
        0x03 => "x86",
        0x28 => "ARM",
        0x3E => "x86-64",
        0xB7 => "AArch64",
        0xF3 => "RISC-V",
        0x08 => "MIPS",
        0x14 => "PowerPC",
        0x15 => "PowerPC64",
        _ => "unknown",
    }
}
