#!/usr/bin/env bash
# Builds target/release/bundle/appimage/Dotnote-x86_64.AppImage: the app for
# any Linux desktop, and what the in-app updater downloads for AppImage copies
# (it replaces the .AppImage file in place, see dotnote-core/src/update_install.rs).
# Usage: packaging/linux/appimage.sh   (on Linux, x86_64)
#
# No libraries are bundled: the app uses the system's Wayland/X11, fontconfig
# and Vulkan driver, which every desktop has. glibc must be at least as new as
# where it was built, so the release workflow builds on an older Ubuntu.
set -euo pipefail
cd "$(dirname "$0")/../.."

OUT=target/release/bundle/appimage
APPDIR="$OUT/Dotnote.AppDir"
TOOL="$OUT/appimagetool-x86_64.AppImage"

cargo build --release -p dotnote

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
cp target/release/dotnote "$APPDIR/usr/bin/dotnote"
cp packaging/linux/id.sofyan.Dotnote.desktop "$APPDIR/id.sofyan.Dotnote.desktop"
cp packaging/linux/icons/hicolor/256x256/apps/dotnote.png "$APPDIR/dotnote.png"
for size in 16 32 48 64 128 256 512; do
  mkdir -p "$APPDIR/usr/share/icons/hicolor/${size}x${size}/apps"
  cp "packaging/linux/icons/hicolor/${size}x${size}/apps/dotnote.png" \
     "$APPDIR/usr/share/icons/hicolor/${size}x${size}/apps/dotnote.png"
done
# The AppImage runtime starts AppRun; it starts the app from inside the image.
cat > "$APPDIR/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/dotnote" "$@"
APPRUN
chmod +x "$APPDIR/AppRun"

if [ ! -x "$TOOL" ]; then
  curl --fail --location --silent --show-error -o "$TOOL" \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$TOOL"
fi
# Extract-and-run: CI containers have no FUSE to mount the tool's own image.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$TOOL" --no-appstream "$APPDIR" "$OUT/Dotnote-x86_64.AppImage"
echo "Built $OUT/Dotnote-x86_64.AppImage"
