#!/usr/bin/env bash
set -euo pipefail

image=${1:?usage: audit-appimage.sh path/to/Threadline.AppImage}
image=$(realpath "$image")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(
  cd "$work"
  "$image" --appimage-extract >/dev/null
)
root="$work/squashfs-root"
lib="$root/usr/lib"

for pattern in 'libglib-2.0.so*' 'libgobject-2.0.so*' 'libgio-2.0.so*' 'libgmodule-2.0.so*' 'libnghttp2.so*' 'libcurl.so*' 'libcurl-gnutls.so*'; do
  if compgen -G "$lib/$pattern" >/dev/null; then
    printf 'incompatible host/plugin boundary library was bundled: %s\n' "$pattern" >&2
    exit 1
  fi
done

test -f "$lib/gstreamer-1.0/libgstautodetect.so"
test -f "$root/apprun-hooks/linuxdeploy-plugin-gstreamer.sh"
printf 'AppImage library audit passed (%s libraries, %s GStreamer plugins).\n' \
  "$(find "$lib" -maxdepth 1 -type f | wc -l)" \
  "$(find "$lib/gstreamer-1.0" -maxdepth 1 -type f | wc -l)"
