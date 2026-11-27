#!/usr/bin/env bash
# Tries a bundle of FastEmu's Flatpak, in the flatpak-builder container
# where release.yml builds it, on a machine of the bundle's architecture:
#
#   packaging/flatpak/check.sh BUNDLE VERSION
#
# It installs the bundle for the user and checks that the sandbox asks for
# the display, the GPU, sound and the network, and for no files. In the
# sandbox it finds the program, its entry, icon, metadata and license,
# loads each library GPUI opens from the runtime, and runs
# `fastemu --version`. Then it uninstalls the bundle.

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: $0 BUNDLE VERSION" >&2
  exit 2
fi
bundle="$1"
version="$2"
id=io.github.fzn0x.fastemu

fail() {
  echo "check.sh: $*" >&2
  exit 1
}

flatpak install --user --noninteractive --assumeyes --bundle "$bundle"
flatpak info --user "$id"

echo "== The sandbox's permissions"
permissions="$(flatpak info --user --show-permissions "$id")"
echo "$permissions"
# One "key=a;b;" line per kind, under [Context].
granted() {
  sed -n "s/^$1=//p" <<<"$permissions" | tr ';' '\n' | grep -v '^$' | sort | tr '\n' ' '
}
[ "$(granted shared)" = "ipc network " ] || fail "it shares '$(granted shared)', not ipc and network"
[ "$(granted sockets)" = "fallback-x11 pulseaudio wayland " ] || fail "its sockets are '$(granted sockets)'"
[ "$(granted devices)" = "dri " ] || fail "its devices are '$(granted devices)', not dri"
[ -z "$(granted filesystems)" ] || fail "it asks for files: $(granted filesystems)"

echo "== The program and its files"
flatpak run --user --command=sh "$id" -c "
  set -e
  test -x /app/bin/fastemu
  test -f /app/share/applications/$id.desktop
  test -f /app/share/icons/hicolor/256x256/apps/$id.png
  test -f /app/share/icons/hicolor/scalable/apps/$id.svg
  test -f /app/share/metainfo/$id.metainfo.xml
  test -f /app/share/licenses/$id/LICENSE
  cat /app/share/applications/$id.desktop
"

echo "== The libraries GPUI opens, from the runtime"
flatpak run --user --command=python3 "$id" -c '
import ctypes
for library in [
    "libvulkan.so.1", "libwayland-client.so.0", "libwayland-cursor.so.0",
    "libxkbcommon.so.0", "libxkbcommon-x11.so.0", "libX11-xcb.so.1", "libxcb.so.1",
    "libfontconfig.so.1", "libfreetype.so.6", "libasound.so.2",
]:
    ctypes.CDLL(library)
    print("loaded", library)
'

echo "== FastEmu in the sandbox"
said="$(flatpak run --user "$id" --version)"
[ "$said" = "fastemu $version" ] || fail "fastemu --version says '$said', not 'fastemu $version'"

flatpak uninstall --user --noninteractive --assumeyes "$id"
echo "check.sh: $bundle installs, runs FastEmu $version and uninstalls"
