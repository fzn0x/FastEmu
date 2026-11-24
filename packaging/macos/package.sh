#!/usr/bin/env bash
# Packs FastEmu.app for a release, on a Mac:
#
#   packaging/macos/package.sh VERSION ARM64_BINARY X86_64_BINARY OUTDIR
#
# It joins the two builds into one universal program with lipo, puts it in
# FastEmu.app with its Info.plist, icon, README and licenses, signs the app
# ad hoc and writes OUTDIR/fastemu-VERSION-macos-universal.zip. Run it from
# the repository's root.
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
trap 'rm -rf "$work"' EXIT

app="$work/FastEmu.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/fastemu" "$arm64" "$x86_64"
lipo "$app/Contents/MacOS/fastemu" -verify_arch arm64 x86_64
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
