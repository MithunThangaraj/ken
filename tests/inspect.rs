//! End-to-end checks: build a minimal valid file for each supported format,
//! then confirm detection and extraction agree on the facts we put in.

use ken::detect::{detect, Format};
use ken::inspect::inspector_for;
use serde_json::Value;

/// Detects `data` and runs the matching inspector, returning both results.
fn analyze(data: &[u8]) -> (Format, serde_json::Map<String, Value>) {
    let det = detect(data);
    let fields = inspector_for(det.format)
        .expect("a supported format should have an inspector")
        .inspect(data)
        .expect("inspection should succeed on a valid fixture");
    (det.format, fields)
}

// ---- fixture builders ------------------------------------------------------

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out.extend_from_slice(&[0, 0, 0, 0]); // CRC is not validated by the inspector
}

/// A PNG with the given geometry, a text chunk, and an IEND terminator.
fn png(width: u32, height: u32, depth: u8, color_type: u8) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[depth, color_type, 0, 0, 0]); // depth, color, comp, filter, interlace
    png_chunk(&mut v, b"IHDR", &ihdr);
    png_chunk(&mut v, b"tEXt", b"note\x00made for tests");
    png_chunk(&mut v, b"IEND", b"");
    v
}

/// A baseline JPEG carrying one SOF0 frame of the given size.
fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF]; // SOI, then the marker prefix detect() looks for
    v.extend_from_slice(&[0xE0, 0x00, 0x10]); // APP0 (JFIF), length 16
    v.extend_from_slice(b"JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    v.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11]); // SOF0, length 17
    v.push(8); // sample precision
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&width.to_be_bytes());
    v.push(3); // component count
    v.extend_from_slice(&[1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]); // 3 component specs
    v.extend_from_slice(&[0xFF, 0xDA]); // start of scan; inspector stops here
    v
}

/// A single-frame GIF89a of the given size with a 2-entry global color table.
fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut v = b"GIF89a".to_vec();
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.extend_from_slice(&[0x80, 0, 0]); // GCT present, size 2
    v.extend_from_slice(&[0, 0, 0, 0xFF, 0xFF, 0xFF]); // the color table
    v.push(0x2C); // image descriptor
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v.push(0); // no local color table
    v.extend_from_slice(&[0x02, 0x02, 0x4C, 0x01, 0x00]); // LZW min code size + one sub-block
    v.push(0x3B); // trailer
    v
}

/// A PCM WAV with the given parameters and `nframes` of silence.
fn wav(sample_rate: u32, channels: u16, bits: u16, nframes: u32) -> Vec<u8> {
    let block = channels as u32 * bits as u32 / 8;
    let byte_rate = sample_rate * block;
    let data = vec![0u8; (nframes * block) as usize];

    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1u16.to_le_bytes()); // PCM
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&byte_rate.to_le_bytes());
    fmt.extend_from_slice(&(block as u16).to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    body.extend_from_slice(&fmt);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(&data);

    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&((4 + body.len()) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(&body);
    v
}

/// A 64-bit little-endian x86-64 executable ELF header.
fn elf() -> Vec<u8> {
    let mut v = vec![0x7F, b'E', b'L', b'F', 2, 1, 1, 0]; // 64-bit, LE, SysV
    v.extend_from_slice(&[0u8; 8]); // rest of e_ident
    v.extend_from_slice(&2u16.to_le_bytes()); // e_type = executable
    v.extend_from_slice(&0x3Eu16.to_le_bytes()); // e_machine = x86-64
    v.extend_from_slice(&1u32.to_le_bytes()); // e_version
    v.extend_from_slice(&[0u8; 8]); // e_entry, room to spare
    v
}

/// A stored (uncompressed) ZIP archive containing the given files.
fn zip_stored(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::new();

    for (name, data) in files {
        offsets.push(out.len() as u32);
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // version..crc
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed size
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed size
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0]); // extra len
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
    }

    let cd_offset = out.len() as u32;
    let mut central = Vec::new();
    for (i, (name, data)) in files.iter().enumerate() {
        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // versions..crc
        central.extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed size
        central.extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed size
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // extra, comment, disk, internal attr
        central.extend_from_slice(&[0, 0, 0, 0]); // external attr
        central.extend_from_slice(&offsets[i].to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);

    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&[0, 0, 0, 0]); // disk numbers
    let n = files.len() as u16;
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&[0, 0]); // comment len
    out
}

// ---- tests -----------------------------------------------------------------

#[test]
fn png_reports_geometry_and_text() {
    let (fmt, f) = analyze(&png(2, 3, 8, 6));
    assert_eq!(fmt, Format::Png);
    assert_eq!(f["width"], 2);
    assert_eq!(f["height"], 3);
    assert_eq!(f["color"], "RGBA");
    assert_eq!(f["text"], true);
    assert_eq!(f["animated"], false);
}

#[test]
fn jpeg_reads_frame_dimensions() {
    let (fmt, f) = analyze(&jpeg(64, 48));
    assert_eq!(fmt, Format::Jpeg);
    assert_eq!(f["width"], 64);
    assert_eq!(f["height"], 48);
    assert_eq!(f["components"], 3);
    assert_eq!(f["mode"], "baseline");
    assert_eq!(f["jfif"], true);
}

#[test]
fn gif_counts_single_frame() {
    let (fmt, f) = analyze(&gif(3, 3));
    assert_eq!(fmt, Format::Gif);
    assert_eq!(f["version"], "89a");
    assert_eq!(f["width"], 3);
    assert_eq!(f["frames"], 1);
    assert_eq!(f["animated"], false);
}

#[test]
fn wav_computes_duration() {
    // 22050 frames at 44100 Hz is exactly half a second.
    let (fmt, f) = analyze(&wav(44100, 2, 16, 22050));
    assert_eq!(fmt, Format::Wav);
    assert_eq!(f["codec"], "PCM");
    assert_eq!(f["channels"], 2);
    assert_eq!(f["rate"], "44100 Hz");
    assert_eq!(f["duration"], "0.50 s");
}

#[test]
fn elf_decodes_header() {
    let (fmt, f) = analyze(&elf());
    assert_eq!(fmt, Format::Elf);
    assert_eq!(f["class"], "64-bit");
    assert_eq!(f["endian"], "little");
    assert_eq!(f["kind"], "executable");
    assert_eq!(f["arch"], "x86-64");
}

#[test]
fn pdf_counts_pages_with_and_without_spaces() {
    let body = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                 2 0 obj\n<< /Type/Pages /Count 2 >>\nendobj\n\
                 3 0 obj\n<< /Type /Page >>\nendobj\n\
                 4 0 obj\n<< /Type/Page >>\nendobj\ntrailer\n<< >>\n%%EOF";
    let (fmt, f) = analyze(body);
    assert_eq!(fmt, Format::Pdf);
    assert_eq!(f["version"], "1.7");
    assert_eq!(f["pages"], 2); // one spaced, one unspaced, minus the Pages node
    assert_eq!(f["encrypted"], false);
}

#[test]
fn zip_lists_entries() {
    let data = zip_stored(&[("note.txt", b"hi"), ("data.csv", b"a,b,c")]);
    let (fmt, f) = analyze(&data);
    assert_eq!(fmt, Format::Zip);
    assert_eq!(f["entries"], 2);
    assert_eq!(f["names"][0], "note.txt");
    assert_eq!(f["names"][1], "data.csv");
    assert_eq!(f.get("format"), None); // a plain zip, not a container format
}

#[test]
fn zip_recognizes_docx_container() {
    let data = zip_stored(&[
        ("[Content_Types].xml", b"<Types/>"),
        ("_rels/.rels", b"<Relationships/>"),
        ("word/document.xml", b"<document/>"),
    ]);
    let (_, f) = analyze(&data);
    assert_eq!(f["format"], "DOCX (OOXML)");
}

#[test]
fn unknown_input_has_no_inspector() {
    let det = detect(b"this is just some plain text\n");
    assert_eq!(det.format, Format::Unknown);
    assert!(inspector_for(det.format).is_none());
}
