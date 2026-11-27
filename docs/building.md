# Building from source

FastEmu is a Rust workspace. The desktop app and the Android app both
use Zed's GPUI for the interface.

## Get the source

```
git clone https://github.com/fzn0x/FastEmu.git
cd FastEmu
```

`rust-toolchain.toml` pins Rust 1.99.0 with clippy and rustfmt. Install
[rustup](https://rustup.rs/) and it downloads them on the first build.
CI and the release builds use the same release.

## Crates

| Crate            | Role                                          |
| ---------------- | --------------------------------------------- |
| `fastemu-core`   | Types shared by the runtime and the apps      |
| `fastemu-jvm`    | The virtual machine and class libraries       |
| `fastemu-gfx`    | 2D graphics                                   |
| `fastemu-audio`  | Sound and MIDI                                |
| `fastemu-midp`   | MIDP, game API, MMAPI, connections, Nokia UI and the other makers' APIs |
| `fastemu-ui`     | The GPUI interface shared by desktop and Android |
| `fastemu`        | The desktop program and the headless runner   |
| `fastemu-android`| The Android app                               |
| `gpui_android`   | GPUI's Android backend                        |
| `xtask`          | Repository tasks, run as `cargo xtask`        |

## Windows

Install Visual Studio Build Tools with the C++ workload and the Windows
SDK, then:

```
cargo build --release -p fastemu
```

The app is `target\release\fastemu.exe`, and its command line is
`target\release\fastemu-console.exe`, which the release names
`fastemu.com` (see [Run without a window](/docs/getting-started/#run-without-a-window)).

## Linux

Install a C compiler, `pkg-config`, the ALSA headers and the development
packages GPUI needs. On Debian and Ubuntu:

```
sudo apt-get install build-essential pkg-config clang cmake lld \
  libasound2-dev libfontconfig-dev libglib2.0-dev libssl-dev \
  libvulkan-dev libwayland-dev libx11-xcb-dev libxcb1-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libzstd-dev mesa-vulkan-drivers
```

Then:

```
cargo build --release -p fastemu
```

The program is `target/release/fastemu`. It needs a Vulkan driver and a
Wayland or X11 session to open its window.

`packaging/linux/package.sh` makes the four Linux files of a release on
Debian or Ubuntu. It downloads nfpm 2.48.0 for the `.rpm` and
appimagetool 1.9.1 with the type2 runtime for the AppImage, and checks
each against its SHA-256. `packaging/linux/check-packages.sh` installs
the `.deb` with apt and the `.rpm` with dnf in a Fedora container, and
runs the AppImage. CI runs both on every change.

## macOS

Install Xcode. GPUI compiles its Metal shaders with Xcode's Metal
toolchain, which newer Xcode versions download separately with
`xcodebuild -downloadComponent MetalToolchain`. Then:

```
cargo build --release -p fastemu
```

The program is `target/release/fastemu`.

## Release builds

`.github/workflows/release.yml` builds the files of a release from a
tag: `fastemu-<version>-windows-x64-setup.exe` and
`-windows-x64-portable.zip`, then `-linux-x64` and `-linux-arm64` each
as `.tar.gz`, `.deb`, `.rpm` and `.AppImage`, then
`-macos-universal.dmg`, `-macos-universal.zip` and `-android.apk`. The
scripts in `packaging/` pack each one and run on your own machine too.
Run the workflow by hand on main, as a dry run, before tagging.
`cargo xtask icons` renders `assets/icon.svg` into `packaging/icons` and
the Android launcher icons.

`packaging/windows/package.ps1` builds the setup from
`packaging/windows/setup.nsi` with NSIS 3.13. It uses `makensis` from
`PATH`, or downloads NSIS's zip and checks its SHA-256 first.
`packaging/windows/check-setup.ps1` installs the setup for the current
user, tries it and uninstalls it. CI runs both on every change.

On a Mac, `packaging/macos/package.sh` makes the zip and the disk image
with `hdiutil`, which comes with macOS. `packaging/macos/dmg-layout.py`
writes the image's Finder window layout with Python's standard library,
so no Finder or screen is needed. `packaging/macos/check-dmg.sh` mounts
the image and tries the app in it. CI runs both on every change.

## Android

The Android app is a Gradle project in `android/` around a native
library that `cargo-ndk` builds. It needs the Android SDK and NDK, a JDK
17, cargo-ndk 4.1.2 and the Rust Android targets:

```
rustup target add aarch64-linux-android x86_64-linux-android
```

[Building the Android app](/docs/building-android/) has the exact
versions and the commands for building, running and signing the `.apk`.

## The Flatpak

`packaging/flatpak/io.github.fzn0x.fastemu.yml` packs the program of a
release's Linux tar.gz on the freedesktop runtime 26.08. The release
builds it with flatpak-builder in Flathub's container, for x64 and
Arm64. `packaging/flatpak/metainfo.py` writes the AppStream metadata with
the releases of `CHANGELOG.md`, and `appstreamcli validate` checks it.
`packaging/flatpak/check.sh` installs the bundle, checks the sandbox's
permissions and runs `fastemu --version` in it. The manifest is not
Flathub's: Flathub builds from source, and FastEmu is not published
there.

## The installers

`packaging/install.sh` and `packaging/install.ps1` install a release from
a terminal. The release attaches them with its version written in, so
each installs its own release by default. `--from` and `-From` take the
files from a folder instead: the release's `installers` job runs
`packaging/check-installers.sh` and `.ps1`, which install the files it
just built that way, try them and uninstall them.

## Homebrew and Scoop

`packaging/render-packages.sh` writes a release's Homebrew cask from
`packaging/homebrew/fastemu.rb.in` and its Scoop manifest from
`packaging/scoop/fastemu.json.in`, with the SHA-256 of the disk image
and of the portable zip. The release's `packages` job puts them in
[fzn0x/homebrew-tap](https://github.com/fzn0x/homebrew-tap) and
[fzn0x/scoop-bucket](https://github.com/fzn0x/scoop-bucket). It needs
the secret `PACKAGES_TOKEN`, a fine-grained token with read and write
access to the contents of those two repositories only. Without it, the
job only says so. A dry run keeps the two files as its `packages`
artifact.

## Tests

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Many tests compile small Java programs, so they need a JDK 9 or newer
(CI uses 17). Without one they skip, unless `FASTEMU_REQUIRE_JAVA=1` is
set, as CI does, in which case they fail. The
[contributing guide](https://github.com/fzn0x/FastEmu/blob/main/CONTRIBUTING.md#test-tools)
describes the test tools.

## The site

The documentation site lives in `site/`. It needs Node 20 or later:

```
cd site
npm ci
node build.mjs
```

The output is in `site/public/`. The build fails on a broken link.
