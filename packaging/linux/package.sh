#!/usr/bin/env bash
# Packs a Linux build of FastEmu for a release:
#
#   packaging/linux/package.sh VERSION ARCH BINARY OUTDIR [--deb]
#
# ARCH is x64 or arm64. It writes OUTDIR/fastemu-VERSION-linux-ARCH.tar.gz,
# a folder of the same name holding the program, its .desktop file, its
# icons in the hicolor theme's layout, the README and the licenses. With
# --deb it also writes OUTDIR/fastemu-VERSION-linux-ARCH.deb, which
# installs the same files under /usr. Run it from the repository's root.
#
# The package's dependencies are the libraries the program links, as
# dpkg-shlibdeps finds them, and those GPUI opens at run time, which no tool
# can see: Vulkan, Wayland, xkbcommon and X11's.

set -euo pipefail

if [ $# -lt 4 ]; then
  echo "usage: $0 VERSION ARCH BINARY OUTDIR [--deb]" >&2
  exit 2
fi
version="$1"
arch="$2"
binary="$3"
outdir="$4"
deb=false
if [ "${5:-}" = --deb ]; then deb=true; fi

case "$arch" in
  x64) deb_arch=amd64 ;;
  arm64) deb_arch=arm64 ;;
  *) echo "ARCH is x64 or arm64, not $arch" >&2; exit 2 ;;
esac

app_id=dev.functionx.fastemu
name="fastemu-$version-linux-$arch"
mkdir -p "$outdir"
outdir="$(cd "$outdir" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Copies the icons into DIR/hicolor/<size>x<size>/apps and the SVG into
# DIR/hicolor/scalable/apps, named after the app's id as its .desktop file
# names them.
install_icons() {
  local dir="$1" size
  for size in 16 24 32 48 64 128 256 512; do
    install -Dm644 "packaging/icons/fastemu-$size.png" \
      "$dir/hicolor/${size}x${size}/apps/$app_id.png"
  done
  install -Dm644 assets/icon.svg "$dir/hicolor/scalable/apps/$app_id.svg"
}

# The archive: one folder, to unpack anywhere.
tree="$work/$name"
install -Dm755 "$binary" "$tree/fastemu"
install -Dm644 "packaging/linux/$app_id.desktop" "$tree/$app_id.desktop"
install_icons "$tree/icons"
install -Dm644 README.md "$tree/README.md"
install -Dm644 LICENSE "$tree/LICENSE"
install -Dm644 THIRD_PARTY_NOTICES.md "$tree/THIRD_PARTY_NOTICES.md"
tar -C "$work" --owner=0 --group=0 --numeric-owner -czf "$outdir/$name.tar.gz" "$name"
echo "$outdir/$name.tar.gz"

if [ "$deb" != true ]; then
  exit 0
fi

root="$work/deb"
install -Dm755 "$binary" "$root/usr/bin/fastemu"
install -Dm644 "packaging/linux/$app_id.desktop" "$root/usr/share/applications/$app_id.desktop"
install_icons "$root/usr/share/icons"
install -Dm644 README.md "$root/usr/share/doc/fastemu/README.md"
install -Dm644 THIRD_PARTY_NOTICES.md "$root/usr/share/doc/fastemu/THIRD_PARTY_NOTICES.md"
cat > "$root/usr/share/doc/fastemu/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: FastEmu
Source: https://github.com/fzn0x/FastEmu

Files: *
Copyright: The FastEmu contributors
License: GPL-3.0-or-later
 On Debian systems, the full text of the GNU General Public License
 version 3 is in /usr/share/common-licenses/GPL-3.
 .
 The licenses of the libraries FastEmu bundles are listed in
 /usr/share/doc/fastemu/THIRD_PARTY_NOTICES.md.
EOF

# dpkg-shlibdeps reads a source package's debian/control, so it runs in an
# empty one.
mkdir -p "$work/shlibdeps/debian"
touch "$work/shlibdeps/debian/control"
linked="$(cd "$work/shlibdeps" && dpkg-shlibdeps -O -e"$root/usr/bin/fastemu" 2>/dev/null |
  sed -n 's/^shlibs:Depends=//p')"
if [ -z "$linked" ]; then
  echo "dpkg-shlibdeps found no libraries in $binary" >&2
  exit 1
fi
# What GPUI opens with dlopen. libasound2 is ALSA's library, which Ubuntu
# 24.04 and Debian 13 call libasound2t64 and which provides this name.
opened="libvulkan1, libwayland-client0, libwayland-cursor0, libxkbcommon0, libxkbcommon-x11-0, libx11-xcb1, libxcb1, libfontconfig1, libfreetype6, libasound2"
size="$(du -sk --exclude=DEBIAN "$root" | cut -f1)"
mkdir -p "$root/DEBIAN"
cat > "$root/DEBIAN/control" <<EOF
Package: fastemu
Version: $version
Architecture: $deb_arch
Maintainer: ${FASTEMU_DEB_MAINTAINER:-fzn0x <fncolon@pm.me>}
Installed-Size: $size
Depends: $linked, $opened
Recommends: mesa-vulkan-drivers | vulkan-icd
Section: games
Priority: optional
Homepage: https://fastemu.functionx.dev
Description: Java ME emulator that runs many games at once in one window
 FastEmu runs Java ME games, the .jar and .jad files made for phones of
 the 2000s. It implements CLDC 1.1 and MIDP 2.0 with a virtual machine of
 its own, so it needs no Java installed. Each game runs in its own virtual
 machine, and one window shows as many games as you start.
EOF
dpkg-deb --root-owner-group -Zxz --build "$root" "$outdir/$name.deb" > /dev/null
dpkg-deb --info "$outdir/$name.deb"
echo "$outdir/$name.deb"
