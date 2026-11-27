#!/usr/bin/env bash
# Packs FastEmu.app for a release, on a Mac:
#
#   packaging/macos/package.sh VERSION ARM64_BINARY X86_64_BINARY OUTDIR
#
# It joins the two builds into one universal program with lipo, puts it in
# FastEmu.app with its Info.plist, icon, README and licenses, and signs the
# app ad hoc. Then it writes two files into OUTDIR:
#
# - fastemu-VERSION-macos-universal.dmg: a disk image holding the app beside
#   a link to /Applications, in a Finder window laid out for dragging one
#   onto the other, with FastEmu's icon for the volume.
# - fastemu-VERSION-macos-universal.zip: the same app, for scripts and
#   package managers.
#
# X86_64_BINARY may be -, as CI gives to try the packaging with one build:
# the app then holds ARM64_BINARY alone. Run it from the repository's root.
# check-dmg.sh then tries the image.
#
# FastEmu has no Apple certificate, so the app is neither signed by a
# developer nor notarized. An ad hoc signature is still needed: Apple
# silicon runs no program without one, and the lipo'd program's own
# signatures do not cover the bundle. Gatekeeper asks once on first launch,
# as the release notes say.

set -euo pipefail

if [ $# -ne 4 ]; then
  echo "usage: $0 VERSION ARM64_BINARY X86_64_BINARY OUTDIR" >&2
  exit 2
fi
version="$1"
arm64="$2"
x86_64="$3"
outdir="$4"

mkdir -p "$outdir"
outdir="$(cd "$outdir" && pwd)"
work="$(mktemp -d)"
disk=""
cleanup() {
  if [ -n "$disk" ]; then
    hdiutil detach -force "$disk" >/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

# The window's layout is checked first against what dmgbuild writes.
python3 packaging/macos/dmg-layout.py --check

app="$work/FastEmu.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
if [ "$x86_64" = - ]; then
  cp "$arm64" "$app/Contents/MacOS/fastemu"
else
  lipo -create -output "$app/Contents/MacOS/fastemu" "$arm64" "$x86_64"
  lipo "$app/Contents/MacOS/fastemu" -verify_arch arm64 x86_64
fi
lipo -info "$app/Contents/MacOS/fastemu"

sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"
test "$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")" = "$version"
printf 'APPL????' > "$app/Contents/PkgInfo"
cp packaging/icons/fastemu.icns "$app/Contents/Resources/fastemu.icns"
cp README.md LICENSE THIRD_PARTY_NOTICES.md "$app/Contents/Resources/"

codesign --force --sign - --timestamp=none "$app"
codesign --verify --strict --verbose=2 "$app"
codesign --display --verbose=2 "$app" 2>&1 | grep -E '^(Identifier|Signature)='

# ditto keeps the bundle as Finder would zip it.
zip="$outdir/fastemu-$version-macos-universal.zip"
rm -f "$zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$zip"
echo "$zip"

# hdiutil fails with "Resource busy" while something such as Spotlight
# holds an image or a volume. It is tried again after 2, 4, 8 and 16
# seconds, as create-dmg does; any other failure stops at once.
hdiutil_retrying() {
  local try delay=2 log="$work/hdiutil.log"
  for try in 1 2 3 4 5; do
    if hdiutil "$@" 2>"$log"; then
      return 0
    fi
    cat "$log" >&2
    if ! grep -q 'Resource busy' "$log" || [ "$try" -eq 5 ]; then
      return 1
    fi
    sleep "$delay"
    delay=$((delay * 2))
  done
}

# The image starts as a folder holding all it shows. ditto copies the
# bundle exactly as signed, without cloning, so hdiutil sizes the image
# by the blocks its files really take.
src="$work/dmg"
mkdir "$src"
ditto --noclone "$app" "$src/FastEmu.app"
ln -s /Applications "$src/Applications"
python3 packaging/macos/dmg-layout.py "$src/.DS_Store" FastEmu.app
cp packaging/icons/fastemu.icns "$src/.VolumeIcon.icns"

# HFS+, or APFS where hdiutil cannot make HFS+ from a folder, as on some
# macOS versions.
rw="$work/rw.dmg"
if ! hdiutil_retrying create -ov -volname FastEmu -srcfolder "$src" -fs HFS+ -fsargs "-c c=64,a=16,e=16" -format UDRW "$rw"; then
  echo "package.sh: hdiutil could not make an HFS+ image; trying APFS" >&2
  hdiutil_retrying create -ov -volname FastEmu -srcfolder "$src" -fs APFS -format UDRW "$rw"
fi

# Finder shows .VolumeIcon.icns as the volume's icon when its root has the
# custom icon flag, 0x0400 at offset 8 of its com.apple.FinderInfo. Only the
# mounted image can take it.
mnt="$work/mnt"
mkdir "$mnt"
disk="$(hdiutil attach -readwrite -noverify -noautoopen -nobrowse -mountpoint "$mnt" "$rw" | awk '!disk && $1 ~ /^\/dev\/disk/ { disk = $1 } END { print disk }')"
if [ -z "$disk" ]; then
  echo "package.sh: hdiutil attached no disk for $rw" >&2
  exit 1
fi
/usr/bin/xattr -wx com.apple.FinderInfo "$(printf '%016d0400%044d' 0 0)" "$mnt"
# macOS makes these while the image is mounted.
rm -rf "$mnt/.fseventsd" "$mnt/.Trashes"
hdiutil_retrying detach "$disk"
disk=""

# Compressed with LZMA, which macOS opens from 10.15 on. Newer macOS asks
# for diskutil instead of hdiutil convert; older diskutil has no image verb.
dmg="$outdir/fastemu-$version-macos-universal.dmg"
rm -f "$dmg"
if diskutil image create from -h >/dev/null 2>&1; then
  diskutil image create from --format ULMO "$rw" "$dmg"
else
  hdiutil convert "$rw" -format ULMO -ov -o "$dmg"
fi
echo "$dmg"
