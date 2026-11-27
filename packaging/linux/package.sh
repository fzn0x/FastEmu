#!/usr/bin/env bash
# Packs a Linux build of FastEmu for a release:
#
#   packaging/linux/package.sh VERSION ARCH BINARY OUTDIR
#
# ARCH is x64 or arm64, and BINARY a build for it. It writes four files
# into OUTDIR, each holding the program, its .desktop file and its icons
# in the hicolor theme's layout:
#
# - fastemu-VERSION-linux-ARCH.tar.gz: a folder of the same name, with the
#   README and the licenses, to unpack anywhere.
# - fastemu-VERSION-linux-ARCH.deb, for Debian and Ubuntu, and
#   fastemu-VERSION-linux-ARCH.rpm, for Fedora and other RPM systems: both
#   install the same files under /usr.
# - fastemu-VERSION-linux-ARCH.AppImage: one file that runs where it is.
#
# Run it from the repository's root, on Debian or Ubuntu: the .deb needs
# dpkg-deb and dpkg-shlibdeps. The tools for the .rpm and the AppImage are
# downloaded and checked against the SHA-256 below. check-packages.sh then
# installs and tries them.
#
# The packages' dependencies are the libraries the program links, and
# those GPUI opens at run time, which no tool can see: Vulkan, Wayland,
# xkbcommon and X11's, fontconfig, FreeType and ALSA. The AppImage holds
# no libraries: like the tar.gz, it runs on the system's.

set -euo pipefail

if [ $# -ne 4 ]; then
  echo "usage: $0 VERSION ARCH BINARY OUTDIR" >&2
  exit 2
fi
version="$1"
arch="$2"
binary="$3"
outdir="$4"

case "$arch" in
  x64) deb_arch=amd64 rpm_arch=amd64 appimage_arch=x86_64 ;;
  arm64) deb_arch=arm64 rpm_arch=arm64 appimage_arch=aarch64 ;;
  *) echo "ARCH is x64 or arm64, not $arch" >&2; exit 2 ;;
esac

# The tools, pinned, with the SHA-256 of their files when they were
# picked. Each runs on the machine of the build: an Arm64 build is packed
# on Arm64.
nfpm_version=2.48.0
declare -A nfpm_sha256=(
  [x86_64]=6d7f9a28b2f397934403950fcbba83f598b08d02edbe79a0815b3b4897341750
  [aarch64]=a9a75213763783b7669719e505474c748c5ad7dc66591a195619deb50f85c2f4
)
appimagetool_url=https://github.com/AppImage/appimagetool/releases/download/1.9.1
declare -A appimagetool_sha256=(
  [x86_64]=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
  [aarch64]=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
)
runtime_url=https://github.com/AppImage/type2-runtime/releases/download/20251108
declare -A runtime_sha256=(
  [x86_64]=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
  [aarch64]=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
)
host="$(uname -m)"
if [ "$host" != "$appimage_arch" ]; then
  echo "package.sh: pack an $arch build on an $appimage_arch machine, not on $host" >&2
  exit 2
fi

# What GPUI opens with dlopen, as the .deb's packages and as the .rpm's
# library names. Debian and Ubuntu 24.04 call ALSA's library
# libasound2t64, which provides libasound2.
opened_deb="libvulkan1, libwayland-client0, libwayland-cursor0, libxkbcommon0, libxkbcommon-x11-0, libx11-xcb1, libxcb1, libfontconfig1, libfreetype6, libasound2"
opened_so=(libvulkan.so.1 libwayland-client.so.0 libwayland-cursor.so.0 libxkbcommon.so.0
  libxkbcommon-x11.so.0 libX11-xcb.so.1 libxcb.so.1 libfontconfig.so.1 libfreetype.so.6 libasound.so.2)

app_id=io.github.fzn0x.fastemu
name="fastemu-$version-linux-$arch"
mkdir -p "$outdir"
outdir="$(cd "$outdir" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Downloads URL to FILE and checks its SHA-256. GitHub can answer a release
# download with a 504 or break it off for a while: curl tries again on any
# error, for up to 30 minutes.
fetch() {
  local url="$1" sha256="$2" file="$3"
  curl -fsSL --retry 15 --retry-all-errors --retry-max-time 1800 -o "$file" "$url"
  echo "$sha256  $file" | sha256sum -c --quiet -
}

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

# The files both packages install.
root="$work/root"
install -Dm755 "$binary" "$root/usr/bin/fastemu"
install -Dm644 "packaging/linux/$app_id.desktop" "$root/usr/share/applications/$app_id.desktop"
install_icons "$root/usr/share/icons"
install -Dm644 README.md "$root/usr/share/doc/fastemu/README.md"
install -Dm644 THIRD_PARTY_NOTICES.md "$root/usr/share/doc/fastemu/THIRD_PARTY_NOTICES.md"

description="Java ME emulator that runs many games at once in one window"
long_description="FastEmu runs Java ME games, the .jar and .jad files made for phones of
the 2000s. It implements CLDC 1.1 and MIDP 2.0 with a virtual machine of
its own, so it needs no Java installed. Each game runs in its own virtual
machine, and one window shows as many games as you start."
maintainer="${FASTEMU_DEB_MAINTAINER:-fzn0x <fncolon@pm.me>}"

# The .deb.
deb="$work/deb"
cp -a "$root" "$deb"
cat > "$deb/usr/share/doc/fastemu/copyright" <<EOF
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
# empty one. Its messages stay in the log, so a failure says why.
mkdir -p "$work/shlibdeps/debian"
touch "$work/shlibdeps/debian/control"
linked="$(cd "$work/shlibdeps" && dpkg-shlibdeps -O -e"$deb/usr/bin/fastemu" |
  sed -n 's/^shlibs:Depends=//p')"
if [ -z "$linked" ]; then
  echo "dpkg-shlibdeps found no libraries in $binary" >&2
  exit 1
fi
size="$(du -sk "$deb" | cut -f1)"
mkdir -p "$deb/DEBIAN"
cat > "$deb/DEBIAN/control" <<EOF
Package: fastemu
Version: $version
Architecture: $deb_arch
Maintainer: $maintainer
Installed-Size: $size
Depends: $linked, $opened_deb
Recommends: mesa-vulkan-drivers | vulkan-icd
Section: games
Priority: optional
Homepage: https://fastemu.functionx.dev
Description: $description
$(sed 's/^/ /' <<<"$long_description")
EOF
dpkg-deb --root-owner-group -Zxz --build "$deb" "$outdir/$name.deb" > /dev/null
dpkg-deb --info "$outdir/$name.deb"
echo "$outdir/$name.deb"

# The .rpm, made with nfpm, needs no rpmbuild. It requires the libraries by
# the names rpm itself gives them, so whatever packages provide them on
# an RPM system will do: those the program links, from its ELF header,
# and those GPUI opens.
case "$host" in
  x86_64) nfpm_arch=x86_64 ;;
  aarch64) nfpm_arch=arm64 ;;
esac
fetch "https://github.com/goreleaser/nfpm/releases/download/v$nfpm_version/nfpm_${nfpm_version}_Linux_$nfpm_arch.tar.gz" \
  "${nfpm_sha256[$host]}" "$work/nfpm.tar.gz"
tar -xzf "$work/nfpm.tar.gz" -C "$work" nfpm
mapfile -t needed < <(readelf -d "$root/usr/bin/fastemu" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p')
if [ ${#needed[@]} -eq 0 ]; then
  echo "readelf found no libraries in $binary" >&2
  exit 1
fi
config="$work/nfpm.yaml"
{
  echo "name: fastemu"
  echo "arch: $rpm_arch"
  echo "platform: linux"
  echo "version: $version"
  echo "release: 1"
  echo "maintainer: \"$maintainer\""
  echo "description: \"$description\""
  echo "vendor: The FastEmu contributors"
  echo "homepage: https://fastemu.functionx.dev"
  echo "license: GPL-3.0-or-later"
  echo "depends:"
  printf '%s\n' "${needed[@]}" "${opened_so[@]}" | sort -u | while read -r library; do
    echo "  - \"$library()(64bit)\""
  done
  echo "recommends: [mesa-vulkan-drivers]"
  echo "rpm:"
  echo "  summary: \"$description\""
  echo "  group: Amusements/Games"
  echo "  compression: xz"
  echo "contents:"
  (cd "$root" && find . -type f | sed 's|^\./||' | sort) | while read -r file; do
    echo "  - src: \"$root/$file\""
    echo "    dst: \"/$file\""
    if [ "$file" = usr/bin/fastemu ]; then
      echo "    file_info: {mode: 0755}"
    else
      echo "    file_info: {mode: 0644}"
    fi
  done
  echo "  - src: \"$PWD/LICENSE\""
  echo "    dst: /usr/share/licenses/fastemu/LICENSE"
  echo "    type: license"
  echo "    file_info: {mode: 0644}"
} > "$config"
cat "$config"
"$work/nfpm" package --config "$config" --packager rpm --target "$outdir/$name.rpm"
echo "$outdir/$name.rpm"

# The AppImage: the program and its share folder under usr, and its entry
# and icon at the top, where AppImage tools look for them.
appdir="$work/AppDir"
cp -a "$root" "$appdir"
cp "packaging/linux/$app_id.desktop" "$appdir/$app_id.desktop"
cp packaging/icons/fastemu-256.png "$appdir/$app_id.png"
ln -s "$app_id.png" "$appdir/.DirIcon"
cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
here="$(dirname "$(readlink -f "$0")")"
exec "$here/usr/bin/fastemu" "$@"
EOF
chmod 0755 "$appdir/AppRun"
fetch "$appimagetool_url/appimagetool-$host.AppImage" "${appimagetool_sha256[$host]}" "$work/appimagetool"
fetch "$runtime_url/runtime-$appimage_arch" "${runtime_sha256[$appimage_arch]}" "$work/runtime"
chmod 0755 "$work/appimagetool"
# The tool is an AppImage itself, and runners have no FUSE to mount it.
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$appimage_arch" "$work/appimagetool" --no-appstream \
  --runtime-file "$work/runtime" "$appdir" "$outdir/$name.AppImage"
echo "$outdir/$name.AppImage"
