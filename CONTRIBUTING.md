# Contributing

Thank you for helping. There are three ways in: fixes and features of the
emulator and its apps, Java classes that a game needs, and reports of games
that do not work.

## Setting up

You need:

- **Rust 1.99.0**. `rust-toolchain.toml` pins the release and the
  components, so [rustup](https://rustup.rs) installs the right ones on the
  first build.
- **A JDK, version 9 or newer**; CI uses 17. The tests compile small Java
  programs with `javac --release 8`, which JDK 8 lacks. The virtual
  machine's differential tests run them on a real JVM and on
  `fastemu-jvm` and compare the output, and the MIDP tests pack them into
  MIDlets and run them through the launcher. Without a JDK they skip,
  unless `FASTEMU_REQUIRE_JAVA=1` is set, as CI does, in which case they
  fail.
- **Linux system packages**. The emulator crates need only a C toolchain
  (`build-essential`), whose linker Rust uses. The GPUI apps also need
  `pkg-config`, the ALSA headers (`libasound2-dev`) for sound, and more.
  On Debian and Ubuntu, everything:

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
  [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) 4.1.2 (`cargo install
  cargo-ndk --version 4.1.2 --locked`), and a JDK 17.
  [docs/building-android.md](docs/building-android.md) has the versions and
  the steps.

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
| `crates/fastemu-midp` | MIDP, the game API, MMAPI, connections, messaging, and the Nokia and other makers' APIs |
| `crates/fastemu-ui` | The GPUI interface that desktop and Android share |
| `crates/fastemu` | The desktop program and the headless runner |
| `crates/fastemu-android` | The Android app's Rust side |
| `crates/gpui_android` | GPUI's Android backend, a trimmed copy of the Zed Android port's |
| `tools/xtask` | Repository tasks, run as `cargo xtask` |
| `android/` | The Android app's Gradle project |
| `site/` | The website: the landing page, and the guides in `docs/` |
| `docs/` | The guides, the architecture notes and the API inventory |
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
for the crates you touched. CI runs Clippy and the emulator crates' tests
on Linux, Windows and macOS, builds and lints the desktop app on all
three, checks the format and the changelog fragments, and checks that no
em-dash or en-dash is in the tree. Use a comma, a colon, a period or a
plain hyphen.

Tests need no network and no game files. A test that needs a game uses a
small one made for the test, which the repository can hold.

## Test tools

Besides the unit tests beside the code, these check FastEmu against what
games expect:

- **The differential tests** (`cargo test -p fastemu-jvm --test
  differential`) run the Java programs in `crates/fastemu-jvm/tests/java`
  on a JDK and on `fastemu-jvm` and compare what they print.
- **The conformance MIDlets** (`cargo test -p fastemu-midp --test
  conformance_midlets`) are small MIDlets in
  `crates/fastemu-midp/tests/conformance/`, one folder per area: events,
  media, records, resources and suites. They compile against stubs of
  exactly the classes FastEmu registers, so a test that calls something
  FastEmu lacks fails to compile, and run through the launcher like real
  games, with their logs and frames checked. The other MIDP tests
  (`screens`, `game_api`, `media_players`, `nokia_ui`, the `io_*` and
  `vendor_*` tests and more) do the same per API, against stubs in their
  own folders.
- **`cargo xtask stubs`** writes a Java stub of every class FastEmu
  registers, with the throws clauses of `docs/compat/api`, and compiles
  them into `fastemu-stubs.jar` in the `fastemu-stubs` folder of the
  target folder (`--out DIR` puts it in DIR). Compile a test MIDlet
  against it to see whether FastEmu has everything the MIDlet calls. It
  needs a JDK.
- **`cargo xtask coverage`** compares the classes FastEmu registers with
  the API inventory in [docs/compat/api](docs/compat/api/README.md), tier
  by tier, and writes [docs/compat/coverage.md](docs/compat/coverage.md).
  Run it after changing the runtime's classes or the inventory, and commit
  the result.
- **`cargo xtask compat DIR`** runs every JAR under `DIR` without a window,
  each in a process of its own, for 20 seconds (`--seconds`), pressing 5
  and then the arrows and fire (`--keys`), and writes `compat.md` and
  `compat.json` with each game's status, frames, errors, the classes it
  asked for that FastEmu lacks, a screenshot and its log, into
  `compat` in the target folder (`--out DIR`). `--profile ID` and
  `--jobs N` pick the phone and how many games run at once.
  [docs/compatibility.md](docs/compatibility.md) lists the free games it
  was run with.
- **`fastemu run`** runs one game without a window, presses keys and
  touches the screen from a script, and saves a screenshot or every frame.
  [docs/getting-started.md](docs/getting-started.md#run-without-a-window)
  has its options.

Two more tasks serve the releases: `cargo xtask icons` renders
`assets/icon.svg` into `packaging/icons` and the Android launcher icons,
and `cargo xtask release-notes --version X.Y.Z --tag vX.Y.Z --repo
fzn0x/FastEmu` writes a release's notes from CHANGELOG.md and
[.github/release-notes.md](.github/release-notes.md), as the release
workflow does. `--unreleased` takes the Unreleased section instead, for
a version not released yet.

### The soak test with real games

`crates/fastemu-midp/tests/many_instances.rs` holds a soak test that is
ignored by default, since it needs games the repository cannot hold. It
starts every JAR in a folder at once on the default profile, presses keys
on a script (nothing for 1.5 s, then `5`, then the arrows and fire in
turn), stops them all, and then runs each game alone for up to 20 s with
the same keys. Each game keeps its record stores in a temporary folder, so
the JARs are only read. Run it with a release build, since a debug build
runs games several times slower:

```sh
FASTEMU_GAMES_DIR=/path/to/jars FASTEMU_SOAK_SECONDS=60 \
  cargo test --release -p fastemu-midp --test many_instances -- --ignored --nocapture
```

In PowerShell, set the variables first with `$env:FASTEMU_GAMES_DIR =
"D:\games"` and `$env:FASTEMU_SOAK_SECONDS = 60`. `FASTEMU_SOAK_SECONDS`
is the length of the run side by side, 60 by default. For each game it
prints the frames presented, the frame rate over the whole run, the frame
rate from 3 s to the end of the solo window side by side and alone with
their ratio, the CPU time per second alone, the errors, the slowest
`Instance::send`, whether every thread ended and the virtual machine was
freed, and the final status. Then it prints the CPU time per second and
the peak memory of the run side by side, on Linux and Windows. A ratio
well under 1 means the game slows down when others run.

## Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org),
one small change each, in lowercase imperative:

```
feat(jvm): add java.util.Random
fix(gfx): clip drawRegion at the frame's edge
docs: explain the launcher's lifetime
```

Types: `feat`, `fix`, `perf`, `refactor`, `style`, `docs`, `test`,
`build`, `ci`, `chore`. A scope names the crate or area, such as `core`,
`jvm`, `gfx`, `audio`, `midp`, `ui`, `app`, `android`, `xtask`, `compat`
or `site`. The body, wrapped at 72 columns, says why the change is
needed. End it with `Part of #N`, or `Closes #N` when it finishes the
issue. Each commit builds and passes its tests.

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
the heap and the threads, and where its tests go. The MIDP side has its own
pages: [midp.md](docs/architecture/midp.md) (the event thread, frames,
keys, soft keys and the hooks for screens and GameCanvas),
[graphics.md](docs/architecture/graphics.md),
[rms.md](docs/architecture/rms.md), [audio.md](docs/architecture/audio.md),
[media.md](docs/architecture/media.md),
[connections.md](docs/architecture/connections.md) and
[vendors.md](docs/architecture/vendors.md). In short:

1. Read the class's Javadoc, or the JSR for it, and follow its behavior,
   including the exceptions it throws. Where MicroEmulator or J2ME Loader
   differ from the specification because games rely on it, follow them and
   say so in a comment.
2. Put the class in the module of its package and keep the file small.
3. Test it. For `java.*` classes the differential tests run the same Java
   program on a JDK and on FastEmu and compare the output. For MIDP and
   vendor classes, a small MIDlet runs through the launcher (see
   [Test tools](#test-tools)).
4. Run `cargo xtask coverage` and commit the new
   [docs/compat/coverage.md](docs/compat/coverage.md).
5. Add a fragment to `changelog.d/added/`, if people will notice.

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

1. Make sure every issue of the version's milestone is closed, `main`'s
   CI is green, and a dry run of the release workflow on `main` (run it by
   hand; `dry_run` is the default) builds every file and the notes.
2. Run `cargo xtask changelog --release X.Y.Z`. It folds the fragments into
   CHANGELOG.md under `## [X.Y.Z] - date`, adds an empty `## [Unreleased]`
   and deletes the fragments.
3. Set `version` in the root `Cargo.toml` to `X.Y.Z` and run `cargo update
   --workspace`, which writes the new version into `Cargo.lock`. CI builds
   with `--locked`, so the lock must change with it. Commit the three files
   and the deleted fragments as `chore: release vX.Y.Z`. This is the last
   commit before the tag.
4. Tag the merged commit and push the tag. The release workflow builds the
   downloads and fills in [.github/release-notes.md](.github/release-notes.md).

## Code of conduct

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md).
