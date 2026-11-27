#!/bin/sh
# Installs FastEmu from a release, on Linux or macOS:
#
#   curl -fsSL https://github.com/fzn0x/FastEmu/releases/latest/download/install.sh | sh
#   sh install.sh [--version X.Y.Z] [--from FOLDER_OR_ADDRESS] [--uninstall]
#
# On Linux it installs the tar.gz for this machine, x64 or Arm64, for the
# current user: the program in ~/.local/lib/fastemu, the fastemu command
# in ~/.local/bin, and the menu entry and icons in ~/.local/share. On macOS
# it installs FastEmu.app from the zip into /Applications, or into
# ~/Applications when /Applications cannot be written.
#
# It checks every download against the release's SHA256SUMS before
# installing anything. --version picks a release, by default the one this
# script came with. --from takes the files from a folder or another
# address instead, as CI does with the files a release has just built.
# --uninstall removes what the script installed; games, saves and
# settings stay. It needs curl or wget, tar, and sha256sum or shasum.

set -eu

repo=fzn0x/FastEmu
app_id=io.github.fzn0x.fastemu
# The publish job of release.yml writes the release's version here.
release_version="@VERSION@"

usage="usage: install.sh [--version X.Y.Z] [--from FOLDER_OR_ADDRESS] [--uninstall]"

say() {
  printf 'install.sh: %s\n' "$*"
}

die() {
  printf 'install.sh: %s\n' "$*" >&2
  exit 1
}

version=""
from=""
uninstall=false
while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      [ $# -ge 2 ] || die "--version needs a version, as 0.2.0"
      version="${2#v}"
      shift 2
      ;;
    --from)
      [ $# -ge 2 ] || die "--from needs a folder or an address"
      from="${2%/}"
      shift 2
      ;;
    --uninstall)
      uninstall=true
      shift
      ;;
    -h | --help)
      echo "$usage"
      exit 0
      ;;
    *) die "unknown option $1; $usage" ;;
  esac
done
case "$release_version" in
  @*) ;;
  *) [ -n "$version" ] || version="$release_version" ;;
esac
case "$version" in
  "" | [0-9]*.[0-9]*.[0-9]*) ;;
  *) die "--version takes a version such as 0.2.0, not $version" ;;
esac
if [ -n "$from" ]; then
  base="$from"
elif [ -n "$version" ]; then
  base="https://github.com/$repo/releases/download/v$version"
else
  base="https://github.com/$repo/releases/latest/download"
fi

# Copies the release's file NAME to DEST, from the folder or the address.
fetch() {
  case "$base" in
    http://* | https://*)
      if command -v curl > /dev/null 2>&1; then
        curl -fsSL --retry 5 --retry-delay 2 -o "$2" "$base/$1" || die "cannot download $base/$1"
      elif command -v wget > /dev/null 2>&1; then
        wget -q -O "$2" "$base/$1" || die "cannot download $base/$1"
      else
        die "it needs curl or wget to download FastEmu"
      fi
      ;;
    *) cp "$base/$1" "$2" || die "$base has no $1" ;;
  esac
}

sha256() {
  if command -v sha256sum > /dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  elif command -v shasum > /dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d ' ' -f 1
  else
    die "it needs sha256sum or shasum to check the download"
  fi
}

# Downloads the release's file whose name matches the extended regular
# expression PATTERN, checked against SHA256SUMS, into the work folder,
# and prints its name. WHAT says which file it is, for a message.
download() {
  fetch SHA256SUMS "$work/SHA256SUMS"
  # A name follows its sum after two spaces, or after " *" where
  # sha256sum read the file in binary mode.
  line="$(grep -E "^[0-9a-f]{64} [ *]$1\$" "$work/SHA256SUMS" | head -n 1 || true)"
  [ -n "$line" ] || die "the release${version:+ $version} has no $2"
  want="${line%% *}"
  name="${line##* }"
  name="${name#\*}"
  fetch "$name" "$work/$name"
  [ "$(sha256 "$work/$name")" = "$want" ] ||
    die "$name does not match the release's SHA256SUMS: it was damaged or changed on the way"
  echo "$name"
}

# The version a release file name such as fastemu-0.2.0-linux-x64.tar.gz
# holds.
version_of() {
  printf '%s\n' "$1" | sed -n 's/^fastemu-\([0-9][0-9.]*\)-.*/\1/p'
}

if [ -n "$version" ]; then
  match="$(printf '%s' "$version" | sed 's/\./\\./g')"
else
  match='[0-9]+\.[0-9]+\.[0-9]+'
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM

install_linux() {
  [ "$(id -u)" -ne 0 ] || die "run it as yourself, not as root: it installs FastEmu for the current user"
  case "$(uname -m)" in
    x86_64 | amd64) arch=x64 ;;
    aarch64 | arm64) arch=arm64 ;;
    *) die "FastEmu has no build for $(uname -m)" ;;
  esac
  data="${XDG_DATA_HOME:-$HOME/.local/share}"
  lib="$HOME/.local/lib/fastemu"
  bin="$HOME/.local/bin"
  link="$bin/fastemu"
  entry="$data/applications/$app_id.desktop"
  icons="$data/icons/hicolor"

  if [ "$uninstall" = true ]; then
    if [ -L "$link" ] && [ "$(readlink "$link")" = "$lib/fastemu" ]; then
      rm -f "$link"
    fi
    rm -f "$entry"
    for icon in "$icons"/*/apps/"$app_id".png "$icons"/*/apps/"$app_id".svg; do
      [ ! -e "$icon" ] || rm -f "$icon"
    done
    rm -rf "$lib"
    refresh_linux
    say "uninstalled FastEmu; your games and saves stay in $data/fastemu"
    return
  fi

  # A command of the same name that is not this script's link stays.
  if { [ -e "$link" ] || [ -L "$link" ]; } &&
    ! { [ -L "$link" ] && [ "$(readlink "$link")" = "$lib/fastemu" ]; }; then
    die "$link is there already and is not FastEmu's: move it away, then run this again"
  fi
  case "$lib" in
    *\"* | *\`* | *\$* | *\\*) die "the home folder's path $HOME has a character a menu entry cannot hold" ;;
  esac

  name="$(download "fastemu-$match-linux-$arch\.tar\.gz" "tar.gz for Linux $arch")"
  v="$(version_of "$name")"
  folder="fastemu-$v-linux-$arch"
  # Every file inside the one folder, with no way out of it.
  tar -tzf "$work/$name" > "$work/list" || die "$name is not a tar.gz"
  while IFS= read -r path; do
    case "$path" in
      "$folder" | "$folder/" | "$folder/"*) ;;
      *) die "$name holds $path, outside its folder" ;;
    esac
    case "/$path/" in
      */../*) die "$name holds $path, which leaves its folder" ;;
    esac
  done < "$work/list"
  mkdir "$work/unpacked"
  tar -xzf "$work/$name" -C "$work/unpacked"
  new="$work/unpacked/$folder"
  if ! said="$("$new/fastemu" --version 2>&1)"; then
    die "FastEmu $v does not start here: $said. It needs the ALSA, xkbcommon and Vulkan libraries; the release's .deb or .rpm installs them with it."
  fi
  [ "$said" = "fastemu $v" ] || die "the program in $name says '$said', not 'fastemu $v'"

  # The old version goes only once the new one is in place.
  mkdir -p "$HOME/.local/lib" "$bin" "$data/applications"
  rm -rf "$lib.new" "$lib.old"
  cp -R "$new" "$lib.new"
  if [ -d "$lib" ]; then
    mv "$lib" "$lib.old"
  fi
  mv "$lib.new" "$lib"
  rm -rf "$lib.old"
  ln -sfn "$lib/fastemu" "$link"
  for dir in "$lib/icons/hicolor"/*/apps; do
    size="$(basename "$(dirname "$dir")")"
    mkdir -p "$icons/$size/apps"
    cp "$dir"/* "$icons/$size/apps/"
  done
  # The entry runs the program by its full path, which needs no PATH.
  while IFS= read -r field; do
    case "$field" in
      Exec=*) printf 'Exec="%s" %%F\n' "$lib/fastemu" ;;
      *) printf '%s\n' "$field" ;;
    esac
  done < "$lib/$app_id.desktop" > "$entry"
  refresh_linux

  say "installed FastEmu $v: open it from your menu, or type fastemu"
  case ":$PATH:" in
    *":$bin:"*) ;;
    *) say "$bin is not on your PATH: add it there to type fastemu" ;;
  esac
}

# Tells the desktop about the entry and the icons, where it has the tools.
refresh_linux() {
  if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database -q "$data/applications" 2> /dev/null || true
  fi
  if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -q -t "$icons" 2> /dev/null || true
  fi
}

# The bundle ID of the app at PATH, or nothing.
bundle_id() {
  /usr/bin/plutil -extract CFBundleIdentifier raw "$1/Contents/Info.plist" 2> /dev/null || true
}

install_macos() {
  if [ "$uninstall" = true ]; then
    removed=false
    for dir in /Applications "$HOME/Applications"; do
      if [ -d "$dir/FastEmu.app" ] && [ "$(bundle_id "$dir/FastEmu.app")" = "$app_id" ]; then
        rm -rf "$dir/FastEmu.app"
        say "removed $dir/FastEmu.app"
        removed=true
      fi
    done
    [ "$removed" = true ] || say "FastEmu is in neither /Applications nor ~/Applications"
    say "your games and saves stay in ~/Library/Application Support/FastEmu"
    return
  fi

  name="$(download "fastemu-$match-macos-universal\.zip" "zip for macOS")"
  v="$(version_of "$name")"
  mkdir "$work/unpacked"
  # ditto keeps the bundle as it was signed.
  ditto -x -k "$work/$name" "$work/unpacked" || die "$name is not a zip"
  [ "$(ls -A "$work/unpacked")" = FastEmu.app ] || die "$name holds more than FastEmu.app"
  new="$work/unpacked/FastEmu.app"
  [ "$(bundle_id "$new")" = "$app_id" ] || die "the app in $name is not FastEmu"
  codesign --verify --deep --strict "$new" 2> /dev/null || die "the app in $name has a broken signature"
  said="$("$new/Contents/MacOS/fastemu" --version 2>&1)" || die "FastEmu $v does not start here: $said"
  [ "$said" = "fastemu $v" ] || die "the program in $name says '$said', not 'fastemu $v'"

  # Where it is already, else /Applications, else ~/Applications.
  if [ -d /Applications/FastEmu.app ] && [ -w /Applications ]; then
    dir=/Applications
  elif [ -d "$HOME/Applications/FastEmu.app" ]; then
    dir="$HOME/Applications"
  elif [ -w /Applications ]; then
    dir=/Applications
  else
    dir="$HOME/Applications"
  fi
  app="$dir/FastEmu.app"
  if [ -e "$app" ] && [ "$(bundle_id "$app")" != "$app_id" ]; then
    die "$app is another app: move it away, then run this again"
  fi
  if pgrep -f "$app/Contents/MacOS/fastemu" > /dev/null 2>&1; then
    say "closing FastEmu"
    osascript -e "quit app id \"$app_id\"" > /dev/null 2>&1 || true
    tries=0
    while pgrep -f "$app/Contents/MacOS/fastemu" > /dev/null 2>&1; do
      tries=$((tries + 1))
      [ "$tries" -le 20 ] || die "FastEmu is still open: close it, then run this again"
      sleep 0.5
    done
  fi

  mkdir -p "$dir"
  rm -rf "$dir/.FastEmu.app.new" "$dir/.FastEmu.app.old"
  ditto "$new" "$dir/.FastEmu.app.new"
  if [ -d "$app" ]; then
    mv "$app" "$dir/.FastEmu.app.old"
  fi
  mv "$dir/.FastEmu.app.new" "$app"
  rm -rf "$dir/.FastEmu.app.old"
  say "installed FastEmu $v in $app: open it from Launchpad or Applications"
}

case "$(uname -s)" in
  Linux) install_linux ;;
  Darwin) install_macos ;;
  *) die "this installer is for Linux and macOS; on Windows, run install.ps1" ;;
esac
