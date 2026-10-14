#!/usr/bin/env bash
# Builds "Agency Sim.app" (universal: Apple Silicon + Intel) and a .dmg.
# Run on macOS:  ./scripts/bundle-macos.sh
# Options:       ARCH=native ./scripts/bundle-macos.sh   (only this Mac's arch, faster)
set -euo pipefail

[[ "$(uname)" == "Darwin" ]] || { echo "Run this on macOS." >&2; exit 1; }
cd "$(dirname "$0")/.."

APP="Agency Sim"
BIN="agency-sim"
VERSION="$(grep -m1 '^version' crates/game/Cargo.toml | cut -d'"' -f2)"
DIST="dist"
BUNDLE="$DIST/$APP.app"
ARCH="${ARCH:-universal}"

export MACOSX_DEPLOYMENT_TARGET=11.0

if [[ "$ARCH" == "native" ]]; then
  cargo build --release -p game
  BINARY="target/release/game"
else
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release -p game --target aarch64-apple-darwin
  cargo build --release -p game --target x86_64-apple-darwin
  mkdir -p target/universal
  lipo -create \
    target/aarch64-apple-darwin/release/game \
    target/x86_64-apple-darwin/release/game \
    -output target/universal/game
  BINARY="target/universal/game"
fi

rm -rf "$BUNDLE"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$BINARY" "$BUNDLE/Contents/MacOS/$BIN"
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$BUNDLE/Contents/Info.plist"

# Icon: 1024 PNG -> .iconset -> .icns
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
SRC=packaging/macos/icon-1024.png
for s in 16 32 128 256 512; do
  sips -z $s $s "$SRC" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s*2)) $((s*2)) "$SRC" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$BUNDLE/Contents/Resources/AppIcon.icns"

# Ad-hoc signature: required to run on Apple Silicon, no developer account needed.
codesign --force --deep --sign - "$BUNDLE"

# DMG with an Applications shortcut for drag-and-drop install.
STAGE="$(mktemp -d)"
cp -R "$BUNDLE" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DIST/AgencySim-$VERSION.dmg"
hdiutil create -volname "$APP" -srcfolder "$STAGE" -ov -format UDZO "$DIST/AgencySim-$VERSION.dmg" >/dev/null

echo "Gotowe:"
echo "  $BUNDLE"
echo "  $DIST/AgencySim-$VERSION.dmg"
