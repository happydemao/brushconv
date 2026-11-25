# brushconv

A command-line brush converter built on ProCraft's brush engine. It reads the formats
ProCraft itself imports and writes ProCraft, Photoshop, Krita, GIMP, Infinite Painter,
PaintTool SAI2 and PhotoCraft brushes, so a set painted for one app can be carried to
another.

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
| Infinite Painter `.prbr` | yes | yes | a zip of `properties.json`, a sample stroke and pictures |
| PaintTool SAI2 `.saitdat` | yes | yes (experimental) | a tool's text sections: size, density, form name, scatter, wet mixing |
| PhotoCraft `.pcbrushes` | yes | yes | a JSON group of presets: size, hardness, spacing, opacity, angle, roundness, pressure switches; a stored tip is read as its preview picture |

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
size (`SizeValue`), softness, ratio, spacing, angle, opacity, flow, the pressure
switches and sensor curves (`UseCurve` false means a straight pen response, not that
the option is off), scatter and its axes, mirroring, colour jitter, texture and
pattern, the masking brush as a dual brush, and the paint-op (build-up or wash) mode.
The writer stores the tip as an embedded PNG resource, draws a 1060×324 stroke preview
into the `.kpp`'s own image, writes a 200×200 `preview.png` into the bundle, and keeps
the size (`SizeValue` plus `diameter`, with the pressure curve from the size range),
softness, ratio, spacing, angle, opacity, flow, pressure-to-opacity/flow curves, flow
pressure, grain as a pattern resource, scatter and its axes, mirroring, colour jitter,
and a dual brush as a masking brush (composite op from its blend mode, its own size
share and settings). The author pictures have no Krita equivalent and are not
written.

**GIMP `.gbr`** — reads a grey tip, its name and spacing; `.gih` ranked tips and
`.vbr` parametric brushes become tips too, and `.pat` becomes a stamp. The writer
emits a GIMP v2 grey brush: an 8-bit single-channel tip, the name and the spacing
percentage. GIMP brushes carry no colour, size range, grain or pressure settings, so
all of those are lost.

**Infinite Painter `.prbr`** — a zip of `properties.json` and a `preview` PNG, plus a
`head` and a `texture` picture when the brush has pictures of its own. It reads the
head, paint size (3.74 pixels a point) and size maximum, paint opacity, the head's
flow, softness, angle, spacing, head rotation, the pressure dynamics with their
profiles, scatter, and size and colour jitter. A brush that paints with a picture
gets that picture as `head` (ink on white) and a `texture` for its grain, named by
the BLAKE2b-512 digest of their bytes in `custom-head` / `custom-stroke texture`; a
round brush keeps the app's own round head and its falloff travels in `softness`.
The app's other settings — tilt and velocity dynamics, the blend and wet sections,
the jitter channels, the profiles — ride along in the brush's extras, so a `.prbr`
written from a `.prbr` comes back with every number, switch, profile and string the
same; a brush converted from another format gets the app's defaults where that
format has nothing to say. Whole numbers are written as integers and the entries are
stored, as the app's own files do.

**PaintTool SAI2 `.saitdat`** — reads a custom tool's text sections: its name, size
in canvas pixels and least size, density and least density, edge hardness, spacing,
angle and whether the stamp follows the stroke, scatter with its count and size
jitter, colour jitter, the brush form's and texture's BMP names, and the water tool's
mixing (how much colour it picks up, its water content, how far its colour
stretches). SAI2 keeps the pictures a tool names (blotmap, brshape and scatter BMPs,
brushtex and papertex) beside the tool in its own settings folder, so a tool read on
its own comes as a round tip at the form's hardness and its texture is left behind.
The writer emits a tool in SAI2's own key order with no form or texture named (SAI2
falls back to its round one) and keeps size, least size, density, hardness, spacing,
angle, follow-the-stroke, scatter, jitter and the wet mixing; drop the file into
`Documents\SYSTEMAX Software Development\SAIv2\settings\custool\nrm\` and add it
to a tool group. Writing is **experimental**: SAI2 documents nothing (the layout here
comes from Brush Bridge's reverse-engineering, MIT), selection tools and SAI2's speed
and dynamics extras are not modelled, and no real SAI2 install was available to open
the result.

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
- `-t, --to <FORMAT>` — `brush`, `brushset`, `abr`, `kpp`, `bundle`, `gbr`, `prbr`,
  `saitdat`, `pcbrushes`, `json` (aliases: `procreate`, `photoshop`, `ps`, `krita`,
  `krita-preset`, `krita-bundle`, `gimp`, `infinite-painter`, `sai`, `sai2`,
  `photocraft`, `meta`).
- `-n, --name <NAME>` — the name of the written set.
- `-l, --list` — print each brush's size range, spacing and opacity.
- `-r, --recursive` — look inside subfolders when given a folder.
- `--flat` — when a folder holds several sets, keep their per-brush files together in
  one folder instead of one folder per set.

Without `--to`, the output file's extension decides the format. Without either, a
set of several brushes becomes a `.brushset` and a single brush a `.brush`. The tool
prints every file it writes and exits non-zero if any input failed.

Several inputs given at once are merged into one set for the formats that hold one
(`brushset`, `abr`, `bundle`, `pcbrushes`, `json`), so `brushconv folder/ --to abr` makes a single
`.abr` of the whole folder. The per-brush formats (`brush`, `kpp`, `gbr`, `prbr`,
`saitdat`) write one file per brush into the output folder instead.

## What survives a conversion

ProCraft's own formats are lossless. The others carry what their format can say:

| Setting | `.brush`/`.brushset` | `.abr` | `.kpp`/`.bundle` | `.gbr` | `.prbr` | `.saitdat` | `.pcbrushes` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| brush tip image | yes | yes | yes | yes | yes | no (round tip; the form's BMP is named, not held) | preview picture only (written as a round tip) |
| grain/texture | yes | no | yes (pattern) | no | yes (texture) | no (texture BMP is named, not held) | no |
| size range | yes | no (tip size) | yes (size, min as the curve's low point) | no (tip size) | yes (paint size × size maximum, 3.74 px a point) | yes (size and least size) | no (one size) |
| opacity range | yes | no | max opacity | no | max opacity | density and least density | max opacity |
| spacing | yes | percent | fraction | percent | fraction | percent | fraction |
| pressure → size | yes, with curves | no | yes (sensor curve) | no | yes, with profile | yes (the two sizes make the curve) | switch only (a straight line) |
| pressure → opacity | yes, with curves | no | yes (sensor) | no | no | yes (least density) | switch only |
| pressure → flow | yes, with curve | no | yes (sensor) | no | yes, with profile | no | no |
| scatter / size jitter | yes | no | yes (scatter, both axes) | no | yes | yes (amount, count, size jitter) | no |
| mirroring | yes | no | yes | no | no | no | yes |
| colour jitter | yes | no | yes | no | no | yes (four channels) | no |
| dual brush | yes | no | yes (masking brush) | no | no | no | no |
| wet mixing | yes | no | partly | no | no | yes (water tool) | no |
| thumbnail | yes | no | stroke preview | no | preview | no | stroke preview |

The ABR writer emits version 2 sampled brushes, the variant Photoshop and most other
apps read without extra metadata; a brush's Photoshop size therefore follows its
embedded tip, and the size slider extremes are not carried. Krita presets keep the
absolute size in pixels (ProCraft's size is canvas pixels too), and the pressure
curve's low point carries the size range's low end. `.sut` is import-only because
writing Clip Studio's SQLite library is out of scope; the reader covers the material
and settings columns ProCraft uses.

## How this compares to Brush Bridge

[Brush Bridge](https://github.com/CanderFlower/brush-bridge) (MIT) is the other project
that reads and writes SAI2 tools and is where the `.saitdat` layout here comes from.
It is a Windows-only Python GUI that also detects installed apps and installs brushes
into them. This tool is a native Rust binary with no runtime to install, it also runs
as an Android app that converts brushes on the phone itself (no desktop at all), and
its file-name conversion is scriptable (`--to`, `-o`, `--list`, recursive folders) with
a JSON dump for pipelines. What Brush Bridge has that this does not yet: installing
into a running app's settings folder, SAI2 brush materials (BMP assets), and the
older SAI 1 material formats.

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
  QuickLook thumbnailer, which uses Krita's resource code), including the richer
  preset built from Krita's own `2.bundle` test file (size 144 px, softness 0.64,
  ratio 0.83, scatter, mirroring, colour jitter, masking brush).
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
