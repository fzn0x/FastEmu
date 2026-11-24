# Contributing

Thank you for helping. There are three ways in: fixes and features of the
emulator and its apps, Java classes that a game needs, and reports of games
that do not work.

## Setting up

You need:

- **Rust**, stable. `rust-toolchain.toml` pins the channel and the
  components, so [rustup](https://rustup.rs) installs the right ones on the
  first build.
- **A JDK, version 8 or newer**, for the virtual machine's differential
  tests. They compile small Java programs, run them on a real JVM and on
  `fastemu-jvm`, and compare the output. Without a JDK they skip, unless
  `FASTEMU_REQUIRE_JAVA=1` is set, as CI does, in which case they fail.
- **Linux system packages** for the GPUI apps. On Debian and Ubuntu:

  ```sh
  sudo apt-get install build-essential pkg-config clang cmake lld \
    libasound2-dev libfontconfig-dev libglib2.0-dev libssl-dev \
    libvulkan-dev libwayland-dev libx11-xcb-dev libxcb1-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libzstd-dev mesa-vulkan-drivers
  ```

  The list follows what Zed's own `script/linux` installs. CI uses
  [.github/actions/linux-deps](.github/actions/linux-deps/action.yml).
- **On macOS**, Xcode. GPUI compiles its Metal shaders with Xcode's Metal
  toolchain, which newer Xcode versions download separately.
- **On Windows**, Visual Studio Build Tools with the C++ workload and the
  Windows SDK.
- **For the Android app**: the Android SDK and NDK, the Rust targets
  `aarch64-linux-android` and `x86_64-linux-android`, and
  [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) (`cargo install
  cargo-ndk`). [docs/building-android.md](docs/building-android.md) has the
  steps.

```sh
git clone https://github.com/fzn0x/FastEmu.git
cd FastEmu
cargo test
cargo run -p fastemu
```

## The workspace

| Path | What it is |
| --- | --- |
| `crates/fastemu-core` | The API that the runtime and the apps share: frames, input, device profiles, suites and the launcher |
| `crates/fastemu-jvm` | The virtual machine and the `java.*` classes written in Rust |
| `crates/fastemu-gfx` | 2D graphics |
| `crates/fastemu-audio` | Sound and MIDI |
| `crates/fastemu-midp` | MIDP, the game API, MMAPI and the Nokia UI API |
| `crates/fastemu-ui` | The GPUI interface that desktop and Android share |
| `crates/fastemu` | The desktop program and the headless runner |
| `crates/fastemu-android` | The Android app's Rust side |
| `tools/xtask` | Repository tasks, run as `cargo xtask` |
| `android/` | The Android app's Gradle project |
| `site/` | The documentation site |
| `docs/` | Architecture and building notes |
| `changelog.d/` | Changelog fragments waiting for a release |

`fastemu-core` is the contract between the runtime and the apps. Change it
in a pull request of its own, with the reason in the description, because
every other crate builds on it.

## Commands

Plain `cargo build`, `cargo test` and `cargo clippy` cover the emulator
crates, which build in seconds. The GPUI apps pull in a large dependency
tree, so they build only when named or with `--workspace`.

```sh
cargo fmt --all                                     # format
cargo test                                          # the emulator crates
cargo test -p fastemu-jvm                           # one crate
cargo run -p fastemu                                # the desktop app
cargo clippy --workspace --all-targets -- -D warnings   # lint everything
cargo xtask changelog --check                       # the changelog fragments
```

Before you open a pull request, run the first, second and fifth commands
for the crates you touched. CI runs them on Linux, Windows and macOS, and
checks that no em-dash or en-dash is in the tree. Use a comma, a colon, a
period or a plain hyphen.

Tests need no network and no game files. A test that needs a game uses a
small one made for the test, which the repository can hold.

## Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org),
one small change each, in lowercase imperative:

```
feat(jvm): add java.util.Random
fix(gfx): clip drawRegion at the frame's edge
docs: explain the launcher's lifetime
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`,
`chore`. A scope names the crate or area: `core`, `jvm`, `gfx`, `audio`,
`midp`, `ui`, `desktop`, `android`, `xtask`, `site`. The body, wrapped at 72
columns, says why the change is needed. End it with `Part of #N`, or
`Closes #N` when it finishes the issue. Each commit builds and passes its
tests.

## Pull requests

- One change per pull request, from a branch off the latest `main`. No pull
  request stacks on another one's branch: one that needs another waits for
  it to merge.
- Link the issue with `Part of #N` or `Closes #N`.
- The history stays linear: pull requests merge by rebase, so rebase your
  branch on `main` instead of merging `main` into it.
- Keep changes apart: add new files rather than editing a shared list where
  you can, so that open pull requests do not conflict.
- A change people will notice adds a fragment to
  [changelog.d](changelog.d/README.md), in the folder of its section and
  named after its issue, such as `changelog.d/fixed/101-audio-stutter.md`.
  Do not edit CHANGELOG.md itself. `cargo xtask changelog --print` shows
  what the next release will say.
- Public items have doc comments that say what they do and why.
- No `todo!()` or `unimplemented!()` where a game can reach. Return an
  error that names what is missing.
- Plain, concrete English in code, docs and messages. No marketing words.

## Adding a Java class implemented in Rust

Games call classes of `java.*`, `javax.microedition.*` and vendor packages,
and FastEmu implements them in Rust inside `fastemu-jvm` and
`fastemu-midp`. [docs/architecture/jvm.md](docs/architecture/jvm.md) tells
how a class is declared, how its methods get native bodies, how it reaches
the heap and the threads, and where its tests go. In short:

1. Read the class's Javadoc, or the JSR for it, and follow its behavior,
   including the exceptions it throws. Where MicroEmulator or J2ME Loader
   differ from the specification because games rely on it, follow them and
   say so in a comment.
2. Put the class in the module of its package and keep the file small.
3. Test it. For `java.*` classes the differential tests run the same Java
   program on a JDK and on FastEmu and compare the output.
4. Add a fragment to `changelog.d/added/`, if people will notice.

## Reporting a game that does not work

Open a [game report](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).
Give the game's name, vendor, version and JAR size, which tells releases of
the same game apart, where the file comes from, the FastEmu version and
system, and whether it works, is playable, starts or does not start. Attach
the log's last lines when it does not start. Do not attach the JAR itself:
many of these files are not free to share.

A problem in FastEmu rather than in one game, such as a crash or a wrong
key mapping, is a [bug report](https://github.com/fzn0x/FastEmu/issues/new?template=bug_report.yml).
Security problems go through
[private reporting](SECURITY.md), not a public issue.

## Releases

1. Make sure every issue of the version's milestone is closed and `main`'s
   CI is green.
2. Run `cargo xtask changelog --release X.Y.Z`. It folds the fragments into
   CHANGELOG.md under `## [X.Y.Z] - date`, adds an empty `## [Unreleased]`
   and deletes the fragments.
3. Set `version` in the root `Cargo.toml` to `X.Y.Z`, and commit both as
   `chore: release vX.Y.Z`. This is the last commit before the tag.
4. Tag the merged commit and push the tag. The release workflow builds the
   downloads and fills in [.github/release-notes.md](.github/release-notes.md).

## Code of conduct

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).
