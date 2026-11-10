#!/bin/sh
# Cross-builds Kibble-481 for 64-bit Windows from macOS or Linux and zips it in dist/:
# both .exe files with the SoundFonts beside them (where soundfont_dir looks).
# Needs the MinGW-w64 toolchain (macOS: brew install mingw-w64); the Rust target is added.
set -e
cd "$(dirname "$0")/.."
NAME="Kibble-481"
TARGET="x86_64-pc-windows-gnu"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
OUT="dist/$NAME-windows"
ZIP="dist/$NAME-$VERSION-windows-x64.zip"

rustup target add "$TARGET" >/dev/null
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  cargo build --release --target "$TARGET" -p dd-desktop -p dd-cli

rm -rf "$OUT" "$ZIP"
mkdir -p "$OUT/$NAME/soundfonts"
cp "target/$TARGET/release/kibble-481.exe" "target/$TARGET/release/kibble-481-cli.exe" "$OUT/$NAME/"
cp soundfonts/*.sf2 soundfonts/*.txt soundfonts/*.md "$OUT/$NAME/soundfonts/" 2>/dev/null || true
if [ -d soundfonts/extra ]; then cp -R soundfonts/extra "$OUT/$NAME/soundfonts/"; fi
cp LICENSE "$OUT/$NAME/LICENSE.txt"
cat > "$OUT/$NAME/README.txt" <<TXT
$NAME $VERSION for Windows (64-bit)

Double-click kibble-481.exe to start. Keep the soundfonts folder beside it.
kibble-481-cli.exe is the same engine from the command line (run it for help).

The app is not signed: if Windows SmartScreen stops it, choose More info, then Run anyway.
https://github.com/catskinroyale/kibble-481
TXT
(cd "$OUT" && zip -qr -9 "../$(basename "$ZIP")" "$NAME")
rm -rf "$OUT"
echo "Built $ZIP"
