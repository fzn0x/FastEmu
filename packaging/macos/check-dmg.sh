#!/usr/bin/env bash
# Tries the disk image that package.sh makes, on a Mac, as CI does:
#
#   packaging/macos/check-dmg.sh DMG VERSION [ARCHS]
#
# It mounts the image read-only, where Finder does not show it, and checks
# that it holds FastEmu.app beside the link to /Applications, the window's
# layout and the volume's icon. Then it verifies the app's ad hoc signature,
# that its program holds ARCHS (default arm64 x86_64) and that it prints
# its version. Run it from the repository's root.

set -euo pipefail

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
  echo "usage: $0 DMG VERSION [ARCHS]" >&2
  exit 2
fi
dmg="$1"
version="$2"
archs="${3:-arm64 x86_64}"

fail() {
  echo "check-dmg: $*" >&2
  exit 1
}

work="$(mktemp -d)"
disk=""
cleanup() {
  if [ -n "$disk" ]; then
    hdiutil detach -force "$disk" >/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

mnt="$work/mnt"
mkdir "$mnt"
disk="$(hdiutil attach -readonly -noautoopen -nobrowse -mountpoint "$mnt" "$dmg" | awk '!disk && $1 ~ /^\/dev\/disk/ { disk = $1 } END { print disk }')"
[ -n "$disk" ] || fail "hdiutil attached no disk for $dmg"

app="$mnt/FastEmu.app"
[ -d "$app" ] || fail "the image has no FastEmu.app: $(ls -A "$mnt" | tr '\n' ' ')"
[ "$(readlink "$mnt/Applications")" = /Applications ] || fail "Applications is not a link to /Applications"
python3 packaging/macos/dmg-layout.py "$work/want.DS_Store" FastEmu.app
cmp -s "$mnt/.DS_Store" "$work/want.DS_Store" || fail "the window's layout is not the one dmg-layout.py writes"
[ -s "$mnt/.VolumeIcon.icns" ] || fail "the volume has no icon"
cmp -s "$mnt/.VolumeIcon.icns" packaging/icons/fastemu.icns || fail "the volume's icon is not FastEmu's"
flags="$(xattr -px com.apple.FinderInfo "$mnt" | tr -d ' \n' | cut -c17-20)"
[ "$flags" = 0400 ] || fail "the volume's Finder flags are $flags, without the custom icon flag 0400"

codesign --verify --deep --strict --verbose=2 "$app" || fail "the app's signature does not verify"
signature="$(codesign --display --verbose=2 "$app" 2>&1)"
grep -qx 'Signature=adhoc' <<<"$signature" || fail "the app is not signed ad hoc"
program="$app/Contents/MacOS/fastemu"
# shellcheck disable=SC2086 # one word per architecture
lipo "$program" -verify_arch $archs || fail "the program does not hold $archs: $(lipo -archs "$program")"
said="$("$program" --version)"
[ "$said" = "fastemu $version" ] || fail "fastemu --version says '$said', not 'fastemu $version'"

echo "check-dmg: $dmg holds FastEmu $version for $archs, signed ad hoc, in its laid out window"
