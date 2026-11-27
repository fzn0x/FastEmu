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
tag: `fastemu-<version>-windows-x64-setup.exe`,
`-windows-x64-portable.zip`, `-linux-x64.tar.gz`, `-linux-x64.deb`,
`-linux-arm64.tar.gz`, `-macos-universal.dmg`, `-macos-universal.zip`
and `-android.apk`. The scripts in `packaging/` pack each one and run on
your own machine too. Run the workflow by hand on main, as a dry run,
before tagging. `cargo xtask icons` renders `assets/icon.svg` into
`packaging/icons` and the Android launcher icons.

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
