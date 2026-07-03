# ken

[![CI](https://github.com/MithunThangaraj/ken/actions/workflows/ci.yml/badge.svg)](https://github.com/MithunThangaraj/ken/actions/workflows/ci.yml)

**See who really wrote that Word document, and what else it's hiding.**

Every Office file carries metadata most people never see: the author's name,
who saved it last, how many times it has been revised, and when it was first
created and last changed. `ken` reads it straight out of the file.

```
$ ken report.docx
report.docx
  type     Word document (document)
  mime     application/vnd.openxmlformats-officedocument.wordprocessingml.document
  size     1.3 KiB (1283 bytes)
  entries  5
  title    Q3 Financial Summary
  author   Jane Doe
  last_by  Bob Smith
  revision 17
  created  2024-01-15T09:12:00Z
  modified 2024-03-02T14:48:00Z
  app      Microsoft Office Word
  edit_time 428 min
  company  Acme Corp
  pages    12
  words    3450
```

A `.docx` is really a zip full of XML, and that metadata sits compressed inside
`docProps/core.xml`, out of reach of a plain unzip. `ken` opens the archive,
decompresses those parts, and pulls the fields out. The same works for `.xlsx`
and `.pptx`.

It isn't only for Office files. Point `ken` at anything and it identifies the
format from the bytes (not the file name) and reports what it finds: image
sizes, audio length, PDF page counts, what a binary targets. One command, one
small binary, no setup.

## Why it exists

A few tools already do parts of this. The `file` command names a file's type
but tells you little else. `exiftool` and `mediainfo` go deep, but only on
photos, audio, and video, and neither cracks open an Office document for you.
`ken` does both jobs, naming the format and pulling out its details, across
many kinds of files, all from a single binary.

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
| ZIP    | archive    | number of entries, sizes, compression ratio, file names |
| DOCX / XLSX / PPTX | document | everything from ZIP, plus author, last editor, revision count, created/modified times, editing minutes, app, page and word counts |
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
input returns an error instead of crashing. The dependencies are small: clap,
serde, anyhow, and miniz_oxide for the deflate decompression that reading zip
entries needs. A couple of numbers are estimates rather than exact counts; PDF
page counts are the main one, since pages can hide inside compressed parts that
a quick scan won't see.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
whichever you prefer.
