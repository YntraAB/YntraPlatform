#!/usr/bin/env bash
# Debian (.deb) Packaging script for Yntra Platform
set -e

DEB_ROOT="target/release/linux/deb_root"
OUT_DEB="target/release/yntra-platform_0.1.0_amd64.deb"
BINARY="target/release/yntra-ui"

echo "==================================================="
echo " Yntra Platform - Linux Debian (.deb) Build"
echo "==================================================="

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary $BINARY not found. Run release build first."
    exit 1
fi

rm -rf "$DEB_ROOT" "$OUT_DEB"
mkdir -p "$DEB_ROOT/usr/bin" \
         "$DEB_ROOT/usr/share/applications" \
         "$DEB_ROOT/DEBIAN"

cp "$BINARY" "$DEB_ROOT/usr/bin/yntra-ui"
cp packaging/linux/com.yntra.YntraPlatform.desktop "$DEB_ROOT/usr/share/applications/"

cat << 'EOF' > "$DEB_ROOT/DEBIAN/control"
Package: yntra-platform
Version: 0.1.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Yntra Technologies <support@yntra.se>
Description: Yntra Platform - Dynamic Modular Workspace Engine
 Local-first operational OS engine built with Rust and Dioxus.
EOF

chmod 755 "$DEB_ROOT/DEBIAN"
chmod 644 "$DEB_ROOT/DEBIAN/control"
chmod 755 "$DEB_ROOT/usr/bin/yntra-ui"

if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --build "$DEB_ROOT" "$OUT_DEB"
    echo "Successfully generated $OUT_DEB"
else
    echo "Warning: dpkg-deb not found. Debian package structure prepared at $DEB_ROOT."
fi
