#!/bin/bash
# Package the Tauri build into a signed + notarized Andremul.app (+ zip) on macOS.
#
#   ./scripts/package-mac.sh                          # ad-hoc signed
#   DEV_ID="Developer ID Application: NAME (TEAM)" \
#   NOTARY_PROFILE="andremul-notary" ./scripts/package-mac.sh   # signed + notarized
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true

APP_NAME="Andremul"
BIN_NAME="andremul"
BUNDLE_ID="com.vinesandrushes.andremul"
VERSION="1.0.0"

echo "==> Release build"
( cd src-tauri && cargo build --release )
BIN="src-tauri/target/release/$BIN_NAME"
[ -f "$BIN" ] || { echo "missing binary $BIN"; exit 1; }

APP="dist/$APP_NAME.app"
echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/$BIN_NAME"

echo "==> Icon"
ICONSET="dist/icon.iconset"
rm -rf "$ICONSET"; mkdir -p "$ICONSET"
SRC="src-tauri/icons/icon.png"
for s in 16 32 128 256 512; do
  sips -z $s $s "$SRC" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  d=$((s*2)); sips -z $d $d "$SRC" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/icon.icns"
rm -rf "$ICONSET"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleExecutable</key><string>$BIN_NAME</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.business</string>
</dict>
</plist>
PLIST

echo "==> Signing"
if [ -n "${DEV_ID:-}" ]; then
  echo "    Developer ID: $DEV_ID (hardened runtime)"
  codesign --force --deep --timestamp --options runtime -s "$DEV_ID" "$APP"
else
  echo "    ad-hoc"
  codesign --force --deep -s - "$APP"
fi
codesign --verify --verbose=2 "$APP" 2>&1 | sed 's/^/    /'

if [ -n "${DEV_ID:-}" ] && [ -n "${NOTARY_PROFILE:-}" ]; then
  echo "==> Notarizing (profile: $NOTARY_PROFILE) — a few minutes"
  ditto -c -k --keepParent "$APP" "dist/_notarize.zip"
  xcrun notarytool submit "dist/_notarize.zip" --keychain-profile "$NOTARY_PROFILE" --wait
  rm -f "dist/_notarize.zip"
  echo "==> Stapling"
  xcrun stapler staple "$APP"
  spctl --assess --type execute --verbose=2 "$APP" 2>&1 | sed 's/^/    /' || true
fi

ZIP="dist/$APP_NAME-mac.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
echo "==> Done: $APP  →  $ZIP ($(du -h "$ZIP" | awk '{print $1}'))"
[ -z "${DEV_ID:-}" ] && echo "    ad-hoc: recipients right-click → Open the first time."
echo "    Recipients also need scrcpy for the display (brew install scrcpy) + Android SDK setup."
