# Building from source

FastEmu is a Rust workspace. The desktop app and the Android app both
use Zed's GPUI for the interface.

## Get the source

```
git clone https://github.com/fzn0x/FastEmu.git
cd FastEmu
```

`rust-toolchain.toml` selects the stable Rust toolchain with clippy
and rustfmt. Install [rustup](https://rustup.rs/) and it downloads on
the first build. The oldest Rust that builds FastEmu is the
`rust-version` in `Cargo.toml`.

## Crates

| Crate            | Role                                          |
| ---------------- | --------------------------------------------- |
| `fastemu-core`   | Types shared by the runtime and the apps      |
| `fastemu-jvm`    | The virtual machine and class libraries       |
| `fastemu-gfx`    | 2D graphics                                   |
| `fastemu-audio`  | Sound and MIDI                                |
| `fastemu-midp`   | MIDP, game API, MMAPI and Nokia UI            |
| `fastemu-ui`     | The GPUI interface shared by desktop and Android |
| `fastemu`        | The desktop program and the headless runner   |
| `fastemu-android`| The Android app                               |

## Windows

Install Visual Studio Build Tools with the C++ workload and the Windows
SDK, then:

```
cargo build --release -p fastemu
```

The program is `target\release\fastemu.exe`.

## Linux

Install a C compiler and the development packages GPUI needs for your
display server. On Debian and Ubuntu these include `libxkbcommon-dev`,
`libwayland-dev`, `libxcb1-dev`, `libvulkan-dev` and `libfontconfig-dev`.
Then:

```
cargo build --release -p fastemu
```

The program is `target/release/fastemu`.

## macOS

Install the Xcode command line tools with `xcode-select --install`,
then:

```
cargo build --release -p fastemu
```

GPUI draws with Metal, so the full Xcode app may be needed to compile
its shaders. The program is `target/release/fastemu`.

## Release builds

`.github/workflows/release.yml` builds the files of a release from a
tag: `fastemu-<version>-windows-x64.zip`, `-linux-x64.tar.gz`,
`-linux-x64.deb`, `-linux-arm64.tar.gz`, `-macos-universal.zip` and
`-android.apk`. The scripts in `packaging/` pack each one and run on
your own machine too. Run the workflow by hand on main, as a dry run,
before tagging. `cargo xtask icons` renders `assets/icon.svg` into
`packaging/icons` and the Android launcher icons.

## Android

You need the Android SDK and NDK, and the Rust Android targets:

```
rustup target add aarch64-linux-android
```

The Android build uses the
[Zed Android port](https://github.com/Dylanmurzello/zed-android-port)
of GPUI. The
`fastemu-android` crate documents the exact commands for building and
signing the `.apk`, which may change as that port changes. Check its
readme before you start.

## Tests

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## The site

The documentation site lives in `site/`. It needs Node 20 or later:

```
cd site
npm ci
node build.mjs
```

The output is in `site/public/`.
