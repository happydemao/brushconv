# brushconv

A command-line brush converter built on ProCraft's brush engine. It reads the formats
ProCraft itself imports and writes ProCraft, Photoshop, Krita and GIMP brushes, so a
set painted for one app can be carried to another.

```
brushconv [OPTIONS] <INPUT>...
```

## Formats

| Format | Read | Write | What it holds |
| --- | --- | --- | --- |
| ProCraft `.brush` | yes | yes | everything the engine models: tips, grain, ranges, curves, dual brush |
| ProCraft `.brushset` | yes | yes | a named set of brushes with folders and thumbnails |
| ProCraft `.brushlibrary` | yes | — | read as its sets; written back as a `.brushset` |
| Photoshop `.abr` | yes | yes | written as version 2 sampled tips; Photoshop's own brush settings are read |
| Photoshop `.gih` / `.vbr` | yes | — | read as brush tips (ranked and parametric variants) |
| Clip Studio `.sut` / `.sutg` | yes | — | the SQLite brush library: node names, settings, material PNGs |
| Krita `.kpp` | yes | yes | a paintbrush preset PNG with the settings XML and embedded tip |
| Krita `.bundle` | yes | yes | a preset bundle with `manifest.xml` and md5-named resources |
| GIMP `.gbr` | yes | yes | a grey brush tip with the GIMP v2 header |
| GIMP `.pat` | yes | — | read as a textured brush stamp |
| Infinite Painter `.prbr` | yes | yes (experimental) | a zip of `properties.json`, a sample stroke and pictures |

Formats are recognised by file name first, then by their contents (PNG signature,
ZIP entries, SQLite header, GIMP header, ABR version word), so a renamed file still
reads.

## What each format can import and export

**ProCraft `.brush` / `.brushset` / `.brushlibrary`** — reads and writes the whole
brush model: the tip picture (or a round tip drawn from the hardness), the grain
picture with its mode, scale and depth, spacing, size and opacity ranges, flow,
pressure-to-size, pressure-to-opacity and pressure-to-flow with their curves, size
and opacity jitter, scatter, shape rotation, the dual brush with its own pictures,
the author's name and pictures, the thumbnail and any extra slider settings. The
ProCraft writer is the lossless round trip; `.brushlibrary` is read-only because a
library's extra level of sets has no writer.

**Photoshop `.abr`** — reads sampled tip bitmaps (8- and 16-bit, raw or PackBits),
the brush name, spacing, angle, roundness, hardness, diameter, the pressure and tilt
dynamics flags, texture and pattern references, dual-brush references, blend mode,
colour and noise. ProCraft derives a brush's size from the tip's pixels (0.03277% a
pixel), so an imported ABR opens at the size its stamp implies. The writer emits
version 2 sampled brushes only: one 8-bit grey tip (up to 5000 px a side), the name,
spacing and the tip's implied size. Version 2 has nowhere to say a size range,
opacity, grain, curves or a dual brush, so those are lost. `.gih` and `.vbr` are
read as tips and have no writer.

**Clip Studio `.sut` / `.sutg`** — reads the library's `Node`/`Variant`/`MaterialFile`
tables: brush names, the settings ProCraft models, and the material PNGs (tip,
texture, dual brush) inside the tar blobs. Import-only: writing Clip Studio's SQLite
schema is out of scope, so `.sut` never appears as an output format.

**Krita `.kpp` / `.bundle`** — reads a preset's XML parameters and the pictures in
its `<resources>`: the tip (a drawn `auto_brush` or an embedded PNG/`.gbr` image),
spacing, angle, opacity, flow, the pressure size/opacity switches and sensor curves,
texture and pattern, and the paint-op (build-up or wash) mode. The writer stores the
tip as an embedded PNG resource, draws a 1060×324 stroke preview into the `.kpp`'s
own image, and writes the settings ProCraft models: spacing, angle, opacity, flow,
pressure-to-size and pressure-to-flow curves, grain as a pattern resource. Krita has
no notion of ProCraft's size range, so presets open at Krita's 1–120 px default; the
dual brush and the author pictures have no Krita equivalent and are not written.

**GIMP `.gbr`** — reads a grey tip, its name and spacing; `.gih` ranked tips and
`.vbr` parametric brushes become tips too, and `.pat` becomes a stamp. The writer
emits a GIMP v2 grey brush: an 8-bit single-channel tip, the name and the spacing
percentage. GIMP brushes carry no colour, size range, grain or pressure settings, so
all of those are lost.

**Infinite Painter `.prbr`** — reads a zip of `properties.json` and a `preview` PNG:
the custom head picture, paint size, size maximum, paint opacity, flow, softness,
angle, spacing, the pressure-to-size and pressure-to-flow effects with their
profiles, scatter and size jitter, and the sample stroke. Named built-in heads and
textures, the tilt and velocity dynamics, the blend and wet settings, and the jitter
channels ProCraft does not model stay behind. The writer is **experimental**: it
zips a white-on-transparent `head.png`, a `texture.png` when the brush has a grain,
a 512×128 sample stroke as `preview`, and a `properties.json` with every section the
app's own exports have, defaults left where ProCraft has no control. Infinite
Painter does not document how it stores embedded art (its own heads are named, not
held), so the `custom-head` / `custom-stroke texture` convention is a best guess.
Whole numbers are written as integers and the zip entries are stored, as the app's
own files do, so a strict reader is not tripped by `1.0` or deflate; the file
round-trips through this tool's own reader exactly for size, opacity, softness,
angle, spacing and the pressure switches, but check it in the app before relying on
it.

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

| Setting | `.brush`/`.brushset` | `.abr` | `.kpp`/`.bundle` | `.gbr` | `.prbr` |
| --- | --- | --- | --- | --- | --- |
| brush tip image | yes | yes | yes | yes | yes |
| grain/texture | yes | no | yes (pattern) | no | yes (texture) |
| size range | yes | no (tip size) | no (Krita default) | no (tip size) | size maximum only |
| opacity range | yes | no | max opacity | no | max opacity |
| spacing | yes | percent | fraction | percent | fraction |
| pressure → size | yes, with curves | no | yes (sensor curve) | no | yes, with profile |
| pressure → opacity | yes, with curves | no | yes (sensor) | no | no |
| pressure → flow | yes, with curve | no | no | no | yes, with profile |
| scatter / size jitter | yes | no | no | no | yes |
| dual brush | yes | no | no | no | no |
| thumbnail | yes | no | stroke preview | no | preview |

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

The app opens a read-only brush editor after a file is read: every field the converter
knows is listed with a green dot when the input carried it and a blue dot when the
chosen output format writes it. The field list and the per-format read/write sets are
in `src/fields.rs`; `fields()` in `../brushconv-ffi` hands them to the app.

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
- `src/write/abr.rs`, `gbr.rs`, `krita.rs`, `prbr.rs`, `json.rs` — the other writers.

`../brushconv-ffi` exposes `formats()`, `inspect()`, `fields()` and `convert()` to the Android app;
`../../android-converter` is that app.
