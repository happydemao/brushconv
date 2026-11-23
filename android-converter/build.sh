#!/bin/sh
# Builds the converter app: the Rust converter library for Android, the Kotlin bindings of it, and
# the APK. Pass Gradle tasks as arguments (the default is `assembleDebug`), e.g.
#
#   ./build.sh assembleRelease
#
# Needs the SDK/NDK, JDK, rustup toolchain and cargo-ndk that ../procraft-env set up (the paths
# below); PROCRAFT_ENV overrides that folder.
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
WORKSPACE=$(cd "$HERE/.." && pwd)
ENV_DIR=${PROCRAFT_ENV:-$WORKSPACE/../procraft-env}

export JAVA_HOME="$ENV_DIR/jdk21/Contents/Home"
export ANDROID_HOME="$ENV_DIR/android-sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_USER_HOME="$ENV_DIR/android-user-home"
export CARGO_HOME="$ENV_DIR/cargo"
export RUSTUP_HOME="$ENV_DIR/rustup"
export CARGO="$CARGO_HOME/bin/cargo"
export GRADLE_USER_HOME="$ENV_DIR/gradle-home"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export PATH="$CARGO_HOME/bin:$JAVA_HOME/bin:$PATH"

ABI=${ABI:-arm64-v8a}
LIB_DIR="$HERE/app/src/main/jniLibs/$ABI"

printf 'Building libbrushconv_ffi.so for %s\n' "$ABI"
# cargo-ndk skips the copy when the destination looks newer (a stripped copy from the last build),
# so clear it first.
rm -f "$LIB_DIR"/*.so
"$CARGO" ndk -t "$ABI" -o "$HERE/app/src/main/jniLibs" build --release -p brushconv-ffi

# cargo-ndk copies every shared library cargo built; only the converter's ships (as the painting
# app does with its own).
rm -f "$LIB_DIR/libprocraft_core.so"

printf 'Generating the Kotlin bindings\n'
rm -rf "$HERE/app/src/main/java/io/github/brushconv/core"
"$CARGO" run --quiet -p procraft-bindgen -- generate "$LIB_DIR/libbrushconv_ffi.so" \
    --language kotlin --out-dir "$HERE/app/src/main/java" --no-format
# The library links procraft-core, whose UniFFI metadata the generator reads too; the converter app
# doesn't use those bindings (and has no procraft_core.so to load them from).
rm -rf "$HERE/app/src/main/java/io/github/artcraftandroid"

# Stripped after the bindings: the metadata they are generated from is in the symbols, which
# --strip-unneeded removes.
printf 'Stripping the library\n'
"$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip --strip-unneeded "$LIB_DIR/libbrushconv_ffi.so"

if [ "$#" -eq 0 ]; then
    set -- assembleDebug
fi
cd "$HERE"
exec ./gradlew "$@" --no-daemon
