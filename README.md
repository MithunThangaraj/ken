# ken

A universal file-inspection CLI. `ken` works out what a file is from its
signature and, for the formats it knows, reads out the metadata inside it. The
default output is meant for humans; `--json` gives you something to pipe into
other tools.

Existing tools each cover part of this. `file` tells you the type and stops
there, `exiftool` is aimed at media, `mediainfo` only does audio and video, and
Apache Tika needs a JVM. `ken` does both the detection and the structured
extraction in one static binary.

## Install

```sh
cargo install --path .
# or, during development:
cargo run -- <file>
```

## Usage

```sh
ken photo.png                 # human-readable report
ken --json photo.png          # machine-readable JSON
ken --detect-only mystery.bin # type only, skip the deep extraction
ken a.png b.pdf c.wav         # inspect several files at once
```

Example:

```
$ ken sample.wav
sample.wav
  type     WAV audio (audio)
  mime     audio/wav
  size     86.2 KiB (88244 bytes)
  codec    PCM
  channels 2
  rate     44100 Hz
  depth    16-bit
  duration 0.50 s
```

Pass several files with `--json` and you get an array; pass one and you get a
bare object. A file it cannot read becomes an error entry, and the process
exits non-zero.

## Supported formats

The current set covers one format per category:

| Format | Category   | What it reads |
| ------ | ---------- | ------------- |
| PNG    | image      | dimensions, bit depth, color type, interlace, chunk count, text and animation |
| JPEG   | image      | dimensions, precision, components, baseline vs progressive, JFIF and Exif |
| GIF    | image      | version, dimensions, frame count, animation, looping |
| PDF    | document   | version, object count, page estimate, encryption, linearization |
| ZIP    | archive    | entry count, sizes, compression ratio, container kind (DOCX, XLSX, PPTX, JAR, EPUB, ODF), member names |
| ELF    | executable | class, endianness, ABI, type, architecture |
| WAV    | audio      | codec, channels, sample rate, bit depth, duration |

## Architecture

```
src/
  main.rs          clap CLI, output orchestration
  detect.rs        signature detection into Format
  report.rs        Report model plus human and JSON rendering
  inspect/
    mod.rs         Inspector trait, registry, byte helpers
    png.rs jpeg.rs gif.rs pdf.rs zip.rs elf.rs wav.rs
```

To add a format, write an `inspect/<fmt>.rs` that implements `Inspector`, add
its signature to `detect.rs`, and register it in `inspector_for`.

Parsing is done by hand with bounds-checked byte helpers, so the dependency
list stays short (clap, serde, anyhow) and malformed input returns an error
rather than panicking. A few of the numbers are estimates rather than exact
counts. PDF page counts are the main example, since pages can hide inside
compressed object streams that a raw byte scan does not see.

## License

MIT OR Apache-2.0
