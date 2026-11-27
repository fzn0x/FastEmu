#!/usr/bin/env bash
# Tries the packages that package.sh makes, as CI does:
#
#   packaging/linux/check-packages.sh VERSION ARCH OUTDIR
#
# On Debian or Ubuntu with sudo and Docker, on a machine of ARCH:
#
# - The .deb installs with apt, which resolves every dependency it names,
#   and the installed program prints its version. Then it is removed.
# - The .rpm installs with dnf in a Fedora container, which resolves every
#   library it requires, and the program prints its version there. The
#   .rpm must require each library GPUI opens.
# - The AppImage holds the program, the .desktop file and the icon at the
#   top, and prints its version.
#
# Run it from the repository's root.

set -euo pipefail

if [ $# -ne 3 ]; then
  echo "usage: $0 VERSION ARCH OUTDIR" >&2
  exit 2
fi
version="$1"
arch="$2"
outdir="$(cd "$3" && pwd)"
name="fastemu-$version-linux-$arch"
app_id=io.github.fzn0x.fastemu
want="fastemu $version"
# The Fedora release the .rpm installs on.
fedora=fedora:45

fail() {
  echo "check-packages: $*" >&2
  exit 1
}

# What GPUI opens with dlopen, which the .rpm must require.
opened_so=(libvulkan.so.1 libwayland-client.so.0 libwayland-cursor.so.0 libxkbcommon.so.0
  libxkbcommon-x11.so.0 libX11-xcb.so.1 libxcb.so.1 libfontconfig.so.1 libfreetype.so.6 libasound.so.2)

for ext in tar.gz deb rpm AppImage; do
  [ -s "$outdir/$name.$ext" ] || fail "$name.$ext is missing"
done

# The .deb. apt reads a local file only by a path with a slash.
bash packaging/retry.sh sudo apt-get install -y --no-install-recommends "$outdir/$name.deb"
said="$(/usr/bin/fastemu --version)"
[ "$said" = "$want" ] || fail "the .deb's fastemu --version says '$said'"
[ -f "/usr/share/applications/$app_id.desktop" ] || fail "the .deb installed no .desktop file"
[ -f "/usr/share/icons/hicolor/256x256/apps/$app_id.png" ] || fail "the .deb installed no icon"
sudo apt-get remove -y fastemu > /dev/null
[ ! -e /usr/bin/fastemu ] || fail "removing the .deb left /usr/bin/fastemu"
echo "check-packages: $name.deb installs, runs and uninstalls"

# The .rpm, in Fedora.
bash packaging/retry.sh docker pull --quiet "$fedora"
requires="$(docker run --rm -v "$outdir:/packages:ro" "$fedora" rpm -qpR "/packages/$name.rpm")"
for library in "${opened_so[@]}"; do
  grep -qxF "$library()(64bit)" <<<"$requires" || fail "the .rpm does not require $library, which GPUI opens"
done
# dnf downloads from Fedora's mirrors, so a failed install is tried again.
docker run --rm -v "$outdir:/packages:ro" "$fedora" bash -euo pipefail -c "
  for attempt in 1 2 3 4 5; do
    if dnf install -y --setopt=install_weak_deps=False /packages/$name.rpm > /dev/null; then
      break
    fi
    [ \$attempt -lt 5 ] || exit 1
    sleep \$((attempt * 15))
  done
  said=\"\$(fastemu --version)\"
  [ \"\$said\" = '$want' ] || { echo \"the .rpm's fastemu --version says '\$said'\" >&2; exit 1; }
  test -f /usr/share/applications/$app_id.desktop
  test -f /usr/share/icons/hicolor/256x256/apps/$app_id.png
  test -f /usr/share/licenses/fastemu/LICENSE
  dnf remove -y fastemu > /dev/null
  test ! -e /usr/bin/fastemu
"
echo "check-packages: $name.rpm installs, runs and uninstalls on $fedora"

# The AppImage, unpacked, as runners have no FUSE to mount it.
appimage="$outdir/$name.AppImage"
chmod +x "$appimage"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
(cd "$work" && "$appimage" --appimage-extract > /dev/null)
for file in AppRun usr/bin/fastemu "$app_id.desktop" "$app_id.png" "usr/share/applications/$app_id.desktop"; do
  [ -e "$work/squashfs-root/$file" ] || fail "the AppImage has no $file"
done
said="$(APPIMAGE_EXTRACT_AND_RUN=1 "$appimage" --version)"
[ "$said" = "$want" ] || fail "the AppImage's fastemu --version says '$said'"
echo "check-packages: $name.AppImage holds FastEmu and runs"
