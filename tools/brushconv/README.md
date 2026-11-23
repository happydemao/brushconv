# brushconv

A command-line brush converter built on ProCraft's brush engine. It reads the formats
ProCraft itself imports and writes ProCraft, Photoshop, Krita and GIMP brushes, so a
set painted for one app can be carried to another.

```
brushconv [OPTIONS] <INPUT>...
```

## Formats

| Format | Read | Write | Notes |
| --- | --- | --- | --- |
| ProCraft `.brush` | yes | yes | full fidelity: tip, grain, size/opacity ranges, curves, dual brush |
| ProCraft `.brushset` | yes | yes | several brushes, folders and thumbnails |
| ProCraft `.brushlibrary` | yes | — | read as its sets; writing keeps `.brushset` |
| Photoshop `.abr` | yes | yes | writes version 2 sampled tips; size/settings come from the tip |
| Photoshop `.gih` / `.vbr` | yes | — | read as brush tips |
| Krita `.kpp` | yes | yes | paintbrush preset with an embedded tip (and grain) |
| Krita `.bundle` | yes | yes | preset bundle with manifest and md5 resource names |
| GIMP `.gbr` | yes | yes | grey tip with GIMP v2 header |
| GIMP `.pat` | yes | — | read as a textured brush stamp |
| Clip Studio `.sut` / `.sutg` | yes | — | read from the SQLite brush library; there is no writer |
| Infinite Painter `.prbr` | yes | — | zip of `properties.json` and a sample stroke; the app's own heads and textures are named, not held, so a head becomes a round tip (a custom head comes as its picture) |

Formats are recognised by file name first, then by their contents (PNG signature,
ZIP entries, SQLite header, GIMP header, ABR version word), so a renamed file still
reads.

## Usage

```sh
# one brush to one brush
brushconv Soft.abr -o Soft.brush

# a whole ProCraft set to a Photoshop file
brushconv Watercolour.brushset --to abr -o Watercolour.abr

# a set to Krita presets, one .kpp per brush
brushconv Inking.brushset --to kpp -o krita-inking/

# a set to a Krita bundle
brushconv Inking.brushset --to bundle

# look at what a file holds without writing anything
brushconv Inking.brushset --list

# a folder of GIMP brushes into one ProCraft set
brushconv ~/brushes --to brushset -o Gimp.brushes --recursive

# the same set as JSON, for scripts
brushconv Inking.brushset --to json -o inking.json
```

Options:

- `-o, --output <PATH>` — the output file, or a folder for several brushes or inputs.
- `-t, --to <FORMAT>` — `brush`, `brushset`, `abr`, `kpp`, `bundle`, `gbr`, `json`
  (aliases: `procreate`, `photoshop`, `ps`, `krita`, `krita-preset`, `krita-bundle`,
  `gimp`, `meta`).
- `-n, --name <NAME>` — the name of the written set.
- `-l, --list` — print each brush's size range, spacing and opacity.
- `-r, --recursive` — look inside subfolders when given a folder.
- `--flat` — when a folder holds several sets, keep their per-brush files together in
  one folder instead of one folder per set.

Without `--to`, the output file's extension decides the format. Without either, a
set of several brushes becomes a `.brushset` and a single brush a `.brush`. The tool
prints every file it writes and exits non-zero if any input failed.

Several inputs given at once are merged into one set for the formats that hold one
(`brushset`, `abr`, `bundle`, `json`), so `brushconv folder/ --to abr` makes a single
`.abr` of the whole folder. The per-brush formats (`brush`, `kpp`, `gbr`) write one
file per brush into the output folder instead.

## What survives a conversion

ProCraft's own formats are lossless. The others carry what their format can say:

| Setting | `.brush`/`.brushset` | `.abr` | `.kpp`/`.bundle` | `.gbr` |
| --- | --- | --- | --- | --- |
| brush tip image | yes | yes | yes | yes |
| grain/texture | yes | no | yes (pattern) | no |
| size range | yes | no (tip size) | default 1–120 | no (tip size) |
| opacity range | yes | no | max opacity | no |
| spacing | yes | percent | fraction | percent |
| pressure → size/opacity | yes, with curves | no | yes (pressure sensors) | no |
| dual brush | yes | no | no | no |
| thumbnail | yes | no | stroke preview | no |

The ABR writer emits version 2 sampled brushes, the variant Photoshop and most other
apps read without extra metadata; a brush's Photoshop size therefore follows its
embedded tip, and the size slider extremes are not carried. Krita presets encode the
ProCraft size range only as the pressure curve, so Krita opens them at its usual
1–120 px default. `.sut` is import-only because writing Clip Studio's SQLite library
is out of scope; the reader covers the material and settings columns ProCraft uses.

## The Android app

`../../android-converter` is a converter-only Android app (no painting) around the same
code, through the UniFFI crate in `../brushconv-ffi`. Build it with
`android-converter/build.sh`.

## Building

The tool is a member of the ProCraft Cargo workspace:

```sh
cargo build --release -p brushconv
# target/release/brushconv
cargo test -p brushconv
```

It needs no GPU: the conversion path uses `procraft-core`'s readers and
`procraft-engine`'s brush model only. The preview embedded in Krita presets is drawn
on the CPU.

`.github/workflows/brushconv.yml` builds release binaries for Linux (x64/arm64),
macOS (x64/arm64) and Windows (x64); push a `brushconv-v*` tag or run the workflow by
hand to produce them.

## Verification

The tests convert `legacy-brushes/` (21 real ProCraft sets) to ABR and every preset
format, read the results back, and check the names and counts. Beyond that:

- Krita 5's own preset loader renders the written `.kpp` (checked through the macOS
  QuickLook thumbnailer, which uses Krita's resource code).
- Every Krita bundle is unpacked and its `manifest.xml` md5 sums are checked against
  the actual entries.
- ABR output round-trips through ProCraft's Photoshop reader; it has not yet been
  opened in Adobe Photoshop itself (loading it there would add the test brushes to
  the brush panel), so report if a real Photoshop rejects the file.

## Layout

- `src/read.rs` — format detection and reading through `procraft_core::read_any_brushes`.
- `src/images.rs` — tips, grains and PNG encoding for the writers.
- `src/preview.rs` — the CPU stroke drawn into Krita presets and thumbnail fallbacks.
- `src/md5.rs` — MD5 for Krita's resource names.
- `src/write/brush.rs` — ProCraft `.brush`/`.brushset` through `engine::brushfile`.
- `src/write/abr.rs`, `gbr.rs`, `krita.rs`, `json.rs` — the other writers.
