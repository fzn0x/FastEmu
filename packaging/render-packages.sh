#!/usr/bin/env bash
# Writes the release's Homebrew cask and Scoop manifest, as release.yml does:
#
#   packaging/render-packages.sh VERSION FOLDER OUTDIR
#
# FOLDER holds the release's files and its SHA256SUMS. OUTDIR gets
# fastemu.rb, for the tap fzn0x/homebrew-tap, with the disk image's
# SHA-256, and fastemu.json, for the bucket fzn0x/scoop-bucket, with the
# portable zip's. Both come from the templates in packaging/homebrew and
# packaging/scoop. A file the release lacks stops it, so no manifest ever
# points at a download that is not there. It checks the cask's Ruby and
# the manifest's JSON where ruby and python3 are installed. Run it from
# the repository's root.

set -euo pipefail

if [ $# -ne 3 ]; then
  echo "usage: $0 VERSION FOLDER OUTDIR" >&2
  exit 2
fi
version="$1"
folder="$2"
outdir="$3"

# The SHA-256 of the release's file NAME, from SHA256SUMS.
sha256_of() {
  local sum
  sum="$(awk -v name="$1" '$2 == name { print $1 }' "$folder/SHA256SUMS")"
  if ! [[ "$sum" =~ ^[0-9a-f]{64}$ ]]; then
    echo "render-packages: SHA256SUMS has no line for $1" >&2
    exit 1
  fi
  echo "$sum"
}

dmg="$(sha256_of "fastemu-$version-macos-universal.dmg")"
zip="$(sha256_of "fastemu-$version-windows-x64-portable.zip")"
mkdir -p "$outdir"
sed -e "s/@VERSION@/$version/g" -e "s/@DMG_SHA256@/$dmg/g" packaging/homebrew/fastemu.rb.in > "$outdir/fastemu.rb"
sed -e "s/@VERSION@/$version/g" -e "s/@ZIP_SHA256@/$zip/g" packaging/scoop/fastemu.json.in > "$outdir/fastemu.json"
if grep -n '@[A-Z0-9_]*@' "$outdir/fastemu.rb" "$outdir/fastemu.json"; then
  echo "render-packages: a placeholder was left unfilled" >&2
  exit 1
fi
if ruby -e '' > /dev/null 2>&1; then
  ruby -c "$outdir/fastemu.rb" > /dev/null
fi
if python3 -c '' > /dev/null 2>&1; then
  python3 -m json.tool "$outdir/fastemu.json" > /dev/null
fi
echo "$outdir/fastemu.rb"
echo "$outdir/fastemu.json"
