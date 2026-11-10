#!/bin/sh
# Builds Kibble-481.app in dist/ with the icon and the SoundFonts (adapted from Ambient MIDI
# Generator's scripts/bundle_macos.sh).
set -e
cd "$(dirname "$0")/.."
NAME="Kibble-481"
APP="dist/$NAME.app"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)

cargo build --release -p dd-desktop -p dd-cli

ICONSET="target/AppIcon.iconset"
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  sips -z $s $s packaging/macos/icon_1024.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  sips -z $d $d packaging/macos/icon_1024.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/soundfonts"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
cp target/release/ddream target/release/ddream-cli "$APP/Contents/MacOS/"
cp soundfonts/*.sf2 soundfonts/*.txt soundfonts/*.md "$APP/Contents/Resources/soundfonts/" 2>/dev/null || true
if [ -d soundfonts/extra ]; then cp -R soundfonts/extra "$APP/Contents/Resources/soundfonts/"; fi
cp LICENSE "$APP/Contents/Resources/"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$NAME</string>
  <key>CFBundleDisplayName</key><string>$NAME</string>
  <key>CFBundleIdentifier</key><string>com.local.kibble481</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>ddream</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Kibble-481 doesn't record audio; macOS asks because it opens the audio device.</string>
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
echo "Built $APP ($VERSION)"
