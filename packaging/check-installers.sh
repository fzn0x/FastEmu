#!/usr/bin/env bash
# Tries install.sh on Linux or macOS, as release.yml does:
#
#   packaging/check-installers.sh VERSION FOLDER
#
# FOLDER holds the release's files for this system and their SHA256SUMS.
# install.sh installs FastEmu VERSION from it, the installed program
# prints its version, and --uninstall removes all of it. Then the same
# runs once more with FastEmu 0.1.0 downloaded from GitHub, for the
# download path. Run it from the repository's root.

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: $0 VERSION FOLDER" >&2
  exit 2
fi
version="$1"
folder="$2"
app_id=io.github.fzn0x.fastemu

fail() {
  echo "check-installers: $*" >&2
  exit 1
}

# Installs WANT with install.sh ARGS..., checks it and uninstalls it.
try() {
  local want="$1"
  shift
  sh packaging/install.sh "$@"
  case "$(uname -s)" in
    Linux)
      local lib="$HOME/.local/lib/fastemu" entry="$HOME/.local/share/applications/$app_id.desktop"
      [ "$(readlink "$HOME/.local/bin/fastemu")" = "$lib/fastemu" ] || fail "~/.local/bin/fastemu is not FastEmu's link"
      [ "$("$HOME/.local/bin/fastemu" --version)" = "fastemu $want" ] || fail "the installed fastemu is not $want"
      grep -qxF "Exec=\"$lib/fastemu\" %F" "$entry" || fail "the menu entry does not run $lib/fastemu"
      [ -f "$HOME/.local/share/icons/hicolor/256x256/apps/$app_id.png" ] || fail "no icon was installed"
      sh packaging/install.sh --uninstall
      for path in "$lib" "$HOME/.local/bin/fastemu" "$entry" "$HOME/.local/share/icons/hicolor/256x256/apps/$app_id.png"; do
        [ ! -e "$path" ] && [ ! -L "$path" ] || fail "--uninstall left $path"
      done
      ;;
    Darwin)
      local app
      if [ -d /Applications/FastEmu.app ]; then app=/Applications/FastEmu.app; else app="$HOME/Applications/FastEmu.app"; fi
      [ "$("$app/Contents/MacOS/fastemu" --version)" = "fastemu $want" ] || fail "the installed app is not $want"
      codesign --verify --deep --strict "$app" || fail "the installed app's signature does not verify"
      sh packaging/install.sh --uninstall
      [ ! -e "$app" ] || fail "--uninstall left $app"
      ;;
  esac
  echo "check-installers: install.sh installed and uninstalled FastEmu $want"
}

try "$version" --version "$version" --from "$folder"
# The script without --from downloads from the release of --version.
try 0.1.0 --version 0.1.0
