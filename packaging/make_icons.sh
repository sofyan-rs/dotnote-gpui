#!/usr/bin/env bash
# Regenerates every platform icon from the 1024px master `assets/icon/dotnote.png`.
# Runs on macOS (needs `sips` and `iconutil`, both built in). Re-run after changing the master.
set -euo pipefail
cd "$(dirname "$0")/.."

MASTER=assets/icon/dotnote.png
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

resize() { sips -z "$1" "$1" "$MASTER" --out "$2" >/dev/null; }

# macOS: .icns from an .iconset folder (sizes and names are fixed by Apple).
ICONSET="$WORK/Dotnote.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  resize "$size" "$ICONSET/icon_${size}x${size}.png"
  resize "$((size * 2))" "$ICONSET/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$ICONSET" -o packaging/macos/Dotnote.icns

# Windows: multi-size .ico. Windows icons fill the whole canvas, so crop away the
# macOS-style padding first (the rounded square spans 824 of the 1024 px, centered).
sips -c 824 824 "$MASTER" --out "$WORK/win_master.png" >/dev/null
ICO_PNGS=()
for size in 16 24 32 48 64 128 256; do
  sips -z "$size" "$size" "$WORK/win_master.png" --out "$WORK/ico_$size.png" >/dev/null
  ICO_PNGS+=("$WORK/ico_$size.png")
done
python3 packaging/make_ico.py packaging/windows/dotnote.ico "${ICO_PNGS[@]}"

# Linux (Fedora): freedesktop hicolor theme layout, used by the .desktop file and .rpm.
for size in 16 32 48 64 128 256 512; do
  dir="packaging/linux/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$dir"
  resize "$size" "$dir/dotnote.png"
done

echo "Icons regenerated from $MASTER"
