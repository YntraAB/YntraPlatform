#!/usr/bin/env bash
# macOS Apple Developer ID Code Signing & Notarization Script
set -e

IDENTITY="${APPLE_DEVELOPER_IDENTITY:-}"
APPLE_ID="${APPLE_ID:-}"
TEAM_ID="${APPLE_TEAM_ID:-}"
APP_PASSWORD="${APPLE_APP_SPECIFIC_PASSWORD:-}"

APP_BUNDLE="target/release/macos/Yntra.app"
DMG_PATH="target/release/YntraPlatform-macOS.dmg"
ENTITLEMENTS="packaging/macos/entitlements.plist"

echo "==================================================="
echo " Yntra Platform - macOS Signing & Notarization"
echo "==================================================="

if [ -z "$IDENTITY" ]; then
    echo "Warning: APPLE_DEVELOPER_IDENTITY not set. Skipping Developer ID code signing and notarization."
    echo "App bundle remains un-notarized for local development testing."
    exit 0
fi

echo "Signing .app bundle with Hardened Runtime..."
codesign --deep --force --options runtime --entitlements "$ENTITLEMENTS" --sign "$IDENTITY" "$APP_BUNDLE"
codesign --verify --verbose "$APP_BUNDLE"

# Re-generate DMG with signed app
bash packaging/macos/create-dmg.sh

echo "Signing DMG file..."
codesign --force --sign "$IDENTITY" "$DMG_PATH"

if [ -n "$APPLE_ID" ] && [ -n "$APP_PASSWORD" ] && [ -n "$TEAM_ID" ]; then
    echo "Submitting DMG to Apple Notary Service..."
    xcrun notarytool submit "$DMG_PATH" \
        --apple-id "$APPLE_ID" \
        --team-id "$TEAM_ID" \
        --password "$APP_PASSWORD" \
        --wait

    echo "Stapling notarization ticket to DMG..."
    xcrun stapler staple "$DMG_PATH"
    echo "Successfully signed, notarized, and stapled macOS release bundle."
else
    echo "Warning: Apple credentials (APPLE_ID / APPLE_APP_SPECIFIC_PASSWORD) incomplete. Skipping notarization submission."
fi
