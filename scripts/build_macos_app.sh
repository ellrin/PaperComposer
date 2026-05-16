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

mkdir -p "$ROOT_DIR/dist"
ditto "$APP_DIR" "$ROOT_DIR/dist/Paper Composer.app"
