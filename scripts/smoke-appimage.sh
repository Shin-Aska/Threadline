#!/usr/bin/env bash
set -euo pipefail

image=${1:?usage: smoke-appimage.sh path/to/Threadline.AppImage}
image=$(realpath "$image")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(
  cd "$work"
  "$image" --appimage-extract >/dev/null
)

log="$work/startup.log"
set +e
dbus-run-session -- xvfb-run -a bash -c '
  export G_MESSAGES_DEBUG=all GST_DEBUG=2
  "$1/AppRun" >"$2" 2>&1 &
  app=$!
  sleep 12
  ps -eo pid=,ppid=,args= >"$3"
  kill "$app" 2>/dev/null
  wait "$app" 2>/dev/null || true
' bash "$work/squashfs-root" "$log" "$work/processes"
status=$?
set -e
cat "$log"
test "$status" -eq 0

fatal='undefined symbol|Failed to load module: .*libgiolibproxy|Failed to load module: .*libdconfsettings|GStreamer element autoaudiosink not found|GLib-GObject-CRITICAL'
if grep -E "$fatal" "$log"; then
  echo "fatal AppImage startup diagnostic detected" >&2
  exit 1
fi
grep -q 'WebKitWebProcess' "$work/processes" || { cat "$work/processes"; exit 1; }
grep -Eq '(^| )threadline($| )|/usr/bin/threadline|/AppRun.wrapped' "$work/processes" || { cat "$work/processes"; exit 1; }
echo "AppImage process and WebKit subprocess initialized without fatal diagnostics."
