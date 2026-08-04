#!/usr/bin/env bash
# Linux AppImage Packaging script for Yntra Platform
set -e

APP_DIR="target/release/linux/AppDir"
OUT_APPIMAGE="target/release/YntraPlatform-x86_64.AppImage"
BINARY="target/release/yntra-ui"

echo "==================================================="
echo " Yntra Platform - Linux AppImage Build"
echo "==================================================="

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary $BINARY not found. Run release build first."
    exit 1
fi

rm -rf "$APP_DIR" "$OUT_APPIMAGE"
mkdir -p "$APP_DIR/usr/bin" "$APP_DIR/usr/share/applications" "$APP_DIR/usr/share/icons/hicolor/256x256/apps"

cp "$BINARY" "$APP_DIR/usr/bin/yntra-ui"
cp packaging/linux/com.yntra.YntraPlatform.desktop "$APP_DIR/usr/share/applications/"
cp packaging/linux/com.yntra.YntraPlatform.desktop "$APP_DIR/"

# Dummy / default app icon
touch "$APP_DIR/usr/share/icons/hicolor/256x256/apps/yntra-platform.png"
touch "$APP_DIR/yntra-platform.png"

# Create AppRun script
cat << 'EOF' > "$APP_DIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/yntra-ui" "$@"
EOF
chmod +x "$APP_DIR/AppRun"

if command -v appimagetool >/dev/null 2>&1; then
    appimagetool "$APP_DIR" "$OUT_APPIMAGE"
    echo "Successfully generated $OUT_APPIMAGE"
else
    echo "Warning: appimagetool not found in PATH. AppDir prepared at $APP_DIR."
fi
