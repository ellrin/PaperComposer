#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/Paper Composer.app"
CACHE_DIR="/private/tmp/paper-composer-module-cache"

mkdir -p "$CACHE_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources/web" "$APP_DIR/Contents/Resources/assets"

cargo build --release --bin drive_bridge --manifest-path "$ROOT_DIR/Cargo.toml"

env CLANG_MODULE_CACHE_PATH="$CACHE_DIR" \
  swiftc "$ROOT_DIR/native/macos/PaperComposerApp.swift" \
  -framework Cocoa \
  -framework WebKit \
  -o "$APP_DIR/Contents/MacOS/Paper Composer"

cp -R "$ROOT_DIR/web/." "$APP_DIR/Contents/Resources/web/"
cp "$ROOT_DIR/assets/papericon.png" "$APP_DIR/Contents/Resources/papericon.png"
cp "$ROOT_DIR/assets/papericon.png" "$APP_DIR/Contents/Resources/assets/papericon.png"
cp "$ROOT_DIR/target/release/drive_bridge" "$APP_DIR/Contents/Resources/drive_bridge"
chmod +x "$APP_DIR/Contents/MacOS/Paper Composer" "$APP_DIR/Contents/Resources/drive_bridge"

# --- Generate AppIcon.icns from assets/papericon.png ---
SRC_ICON="$ROOT_DIR/assets/papericon.png"
ICONSET_DIR="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET_DIR"
sips -z 16 16     "$SRC_ICON" --out "$ICONSET_DIR/icon_16x16.png"      >/dev/null
sips -z 32 32     "$SRC_ICON" --out "$ICONSET_DIR/icon_16x16@2x.png"   >/dev/null
sips -z 32 32     "$SRC_ICON" --out "$ICONSET_DIR/icon_32x32.png"      >/dev/null
sips -z 64 64     "$SRC_ICON" --out "$ICONSET_DIR/icon_32x32@2x.png"   >/dev/null
sips -z 128 128   "$SRC_ICON" --out "$ICONSET_DIR/icon_128x128.png"    >/dev/null
sips -z 256 256   "$SRC_ICON" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$SRC_ICON" --out "$ICONSET_DIR/icon_256x256.png"    >/dev/null
sips -z 512 512   "$SRC_ICON" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$SRC_ICON" --out "$ICONSET_DIR/icon_512x512.png"    >/dev/null
sips -z 1024 1024 "$SRC_ICON" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$ICONSET_DIR" -o "$APP_DIR/Contents/Resources/AppIcon.icns"
rm -rf "$(dirname "$ICONSET_DIR")"

# --- Write Info.plist so Finder picks up the icon + bundle metadata ---
cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>Paper Composer</string>
  <key>CFBundleDisplayName</key>
  <string>Paper Composer</string>
  <key>CFBundleIdentifier</key>
  <string>com.papercomposer.app</string>
  <key>CFBundleVersion</key>
  <string>1.0</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleExecutable</key>
  <string>Paper Composer</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
</dict>
</plist>
PLIST

# Bump the bundle's mtime so Finder refreshes the cached icon
touch "$APP_DIR"

mkdir -p "$ROOT_DIR/dist"
ditto "$APP_DIR" "$ROOT_DIR/dist/Paper Composer.app"
touch "$ROOT_DIR/dist/Paper Composer.app"
