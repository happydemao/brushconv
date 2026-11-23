# Brush Converter for Android

A small Android app that does one thing: converts brush files. It reads everything
the converter reads (`.brush`, `.brushset`, `.abr`, `.gih`, `.vbr`, `.kpp`, `.bundle`,
`.gbr`, `.pat`, `.sut`, `.sutg`, `.prbr`) and writes ProCraft brushes and sets,
Photoshop `.abr`, Krita presets and bundles, GIMP `.gbr`, and JSON. There is no
painting, no canvas, no other feature.

<p align="center"><em>Pick files → pick a format → pick a folder. That's the app.</em></p>

## How it works

The conversion itself is the same Rust code as the command-line tool in
`../tools/brushconv`. `../tools/brushconv-ffi` is a thin UniFFI library around it, so
the app gets three Kotlin functions — `formats()`, `inspect()`, `convert()` — and the
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

- The formats the CLI can only read (`.sut`, `.sutg`, `.prbr`, `.gih`, `.vbr`,
  `.pat`, `.brushlibrary`) are input only; the same fidelity table as the CLI
  applies — see `../tools/brushconv/README.md`.
- The native library is ~23 MB (it shares the Rust brush core with ProCraft), so the
  APK is ~24 MB.
