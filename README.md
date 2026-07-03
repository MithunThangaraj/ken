# ken

[![CI](https://github.com/MithunThangaraj/ken/actions/workflows/ci.yml/badge.svg)](https://github.com/MithunThangaraj/ken/actions/workflows/ci.yml)

**Tell me what this file is, and what's inside it.**

Point `ken` at any file and it figures out the format from the file's actual
contents (not its name) and reads out the useful facts: how big an image is,
how long an audio clip runs, how many pages a PDF has, what's packed inside a
zip. One command, one small binary, no setup.

```
$ ken photo.jpg
photo.jpg
  type     JPEG image (image)
  mime     image/jpeg
  size     2.1 MiB (2201234 bytes)
  width    4032
  height   3024
  depth    8
  components 3
  mode     baseline
  jfif     true
  exif     false
```

Rename `photo.jpg` to `photo.txt` and `ken` still knows it's a JPEG, because it
reads the bytes rather than trusting the extension.

## Why it exists

A few tools already do parts of this. The `file` command names a file's type
but tells you little else. `exiftool` and `mediainfo` go deep, but only on
photos, audio, and video. `ken` does both jobs, naming the format and pulling
out its details, across many kinds of files, all from a single binary.

## Install

```sh
cargo install --path .
# or, while working on it:
cargo run -- <file>
```

## Usage

```sh
ken report.pdf                # readable summary
ken --json report.pdf         # same data as JSON, for scripts
ken --detect-only mystery.bin # just the file type, skip the details
ken a.png b.pdf c.wav         # several files at once
```

With `--json`, one file gives you a single object and several files give you an
array. If a file can't be read, it shows up as an error and `ken` exits with a
non-zero status.

## What it can read

Right now `ken` handles one common format from each major category:

| Format | Kind of file | What you get |
| ------ | ------------ | ------------ |
| PNG    | image      | size, bit depth, color type, interlace, whether it's animated |
| JPEG   | image      | size, color components, baseline vs progressive, JFIF/Exif tags |
| GIF    | image      | size, frame count, whether it animates and loops |
| PDF    | document   | version, object and page counts, encryption, linearization |
| ZIP    | archive    | number of entries, sizes, compression ratio, file names, and whether it's really a DOCX/XLSX/PPTX/JAR/EPUB |
| ELF    | program    | 32/64-bit, endianness, target OS, executable vs library, CPU architecture |
| WAV    | audio      | codec, channels, sample rate, bit depth, length in seconds |

More formats are easy to add (see below).

## How it works

```
src/
  main.rs          command-line handling and output
  detect.rs        identify the format from the leading bytes
  report.rs        the result, printed as text or JSON
  inspect/
    mod.rs         the Inspector trait and the shared byte helpers
    png.rs jpeg.rs gif.rs pdf.rs zip.rs elf.rs wav.rs
```

Adding a format takes three steps: write an `inspect/<name>.rs` that implements
`Inspector`, add the format's signature to `detect.rs`, and register it in
`inspector_for`.

Every format is parsed by hand with bounds-checked helpers, so bad or truncated
input returns an error instead of crashing, and the only dependencies are clap,
serde, and anyhow. A couple of numbers are estimates rather than exact counts;
PDF page counts are the main one, since pages can hide inside compressed parts
that a quick scan won't see.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
whichever you prefer.
