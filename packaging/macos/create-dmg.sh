#!/usr/bin/env bash
# macOS DMG Packaging script for Yntra Platform
set -e

APP_NAME="Yntra Platform"
BUNDLE_DIR="target/release/macos/Yntra.app"
DMG_PATH="target/release/YntraPlatform-macOS.dmg"
STAGING_DIR="target/release/macos/dmg_staging"

echo "==================================================="
echo " Yntra Platform - macOS DMG Packaging"
echo "==================================================="

if [ ! -d "$BUNDLE_DIR" ]; then
    echo "Error: $BUNDLE_DIR does not exist. Run app bundle build first."
    exit 1
fi

rm -rf "$STAGING_DIR" "$DMG_PATH"
mkdir -p "$STAGING_DIR"

echo "Copying .app bundle to DMG staging area..."
cp -R "$BUNDLE_DIR" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

echo "Creating DMG image..."
hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH"

echo "DMG successfully created at $DMG_PATH"
