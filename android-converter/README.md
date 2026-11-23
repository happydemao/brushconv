# Brush Converter for Android

A small Android app that does one thing: converts brush files. There is no painting,
no canvas, no other feature.

<p align="center"><em>Pick files → look at what came in and what an export keeps → pick a folder.</em></p>

| Format | Read | Write | What it holds |
| --- | --- | --- | --- |
| ProCraft `.brush` / `.brushset` | yes | yes | the whole brush model; lossless round trip |
| ProCraft `.brushlibrary` | yes | — | read as its sets, written as a `.brushset` |
| Photoshop `.abr` | yes | yes | writes v2 sampled tips; reads Photoshop's settings |
| Photoshop `.gih` / `.vbr` | yes | — | read as brush tips |
| Clip Studio `.sut` / `.sutg` | yes | — | the SQLite library's names, settings and material PNGs |
| Krita `.kpp` / `.bundle` | yes | yes | preset XML with an embedded tip, grain and a stroke preview |
| GIMP `.gbr` | yes | yes | a grey tip with name and spacing |
| GIMP `.pat` | yes | — | read as a textured stamp |
| Infinite Painter `.prbr` | yes | yes (experimental) | `properties.json`, a sample stroke and pictures |
| JSON | — | yes | every setting, for scripts |

A format's fidelity is the same as the CLI's: the full per-format list of what can
be imported and exported is in `../tools/brushconv/README.md`. In short, ProCraft's
own files keep everything; `.abr` keeps the tip, name and spacing but no ranges,
grain or curves; `.kpp` keeps the tip, grain, size, spacing, softness, ratio, pressure
curves, scatter, mirroring, colour jitter and a dual brush (as a masking brush);
`.gbr` keeps only a grey tip, name and spacing; `.prbr` keeps the tip,
grain, size maximum, opacity, spacing, pressure curves, scatter and jitter, and is
new enough to be marked experimental.

## The brush editor

After a file is read the app opens a brush editor instead of a plain list. Pick a
brush at the top, pick the target format, and every field the converter knows is
listed with two dots:

- **green** — the input file carried this field;
- **blue** — the chosen output format writes this field;
- **grey** — neither, so it is a default or will be dropped.

The header counts both (`12 会保留 9 项；这个文件带入了 11 项`), so the loss of a
conversion is visible before a folder is picked rather than after. The values shown
are the brush's actual ones, so a `.kpp` export showing `尺寸范围 3 – 144` really is
what Krita will get. The editor is read-only for now: it reports, it does not edit.

The field list and the per-format read/write sets live in
`../tools/brushconv/src/fields.rs`; `../tools/brushconv-ffi` exposes them as
`fields(name, bytes, target)`.

## How it works

The conversion itself is the same Rust code as the command-line tool in
`../tools/brushconv`. `../tools/brushconv-ffi` is a thin UniFFI library around it, so
the app gets four Kotlin functions — `formats()`, `inspect()`, `fields()`, `convert()` — and the
conversion code stays in one place. The native library ships for `arm64-v8a`.

The app writes into the folder you pick through Android's Storage Access Framework, so
it needs no storage permission and can't touch anything else.

## Building

```sh
# debug APK, from this folder
./build.sh

# release APK (signed with the debug key, see below)
./build.sh assembleRelease
```

`build.sh` builds the Rust library with cargo-ndk, generates the Kotlin bindings with
ProCraft's `procraft-bindgen`, strips the library, and runs Gradle. It expects the
SDK/NDK/JDK/rustup set up by `../../procraft-env`; `PROCRAFT_ENV` points it elsewhere.

The APKs land in `app/build/outputs/apk/debug/app-debug.apk` and
`app/build/outputs/apk/release/app-release.apk`.

## Installing

```sh
adb install -r app/build/outputs/apk/release/app-release.apk
```

The release build is signed with the Android **debug** certificate (there is no
release keystore in this repository): it installs and runs, but it can't update a
release published with another key. Add a keystore to `app/build.gradle.kts` before
distributing it.

## Limits

- The formats the CLI can only read (`.sut`, `.sutg`, `.gih`, `.vbr`, `.pat`,
  `.brushlibrary`) are input only. `.prbr` is written on a best-effort basis: Infinite
  Painter does not document how it stores embedded art, so check a converted brush in
  the app.
- The native library is ~23 MB (it shares the Rust brush core with ProCraft), so the
  APK is ~24 MB.
