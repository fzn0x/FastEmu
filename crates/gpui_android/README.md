# gpui_android

GPUI's Android backend: a `gpui::Platform` that runs in a GameActivity and
draws with Vulkan through `gpui_wgpu`. FastEmu's Android app
(`crates/fastemu-android`) runs its GPUI views on it.

This is a trimmed copy of `crates/gpui_android` from the
[Zed Android port](https://github.com/Dylanmurzello/zed-android-port) at
revision `746f088ee8d6b8b273354826757dd09e74565854`, Apache-2.0 like the
original (see `NOTICE` and `LICENSE-APACHE`). It builds against `gpui` and
`gpui_wgpu` from the same repository and revision, the GPUI that FastEmu's
desktop app uses.

## Why a copy and not a git dependency

The port's crate serves Zed, an editor with many windows that runs an Ubuntu
userland. As a dependency it pulls in `zdroid_runtime` and `zd_engine_core`
(the userland engine), `ureq`, and Zed's `util`. It calls 34 Java methods
over JNI, on a `com.zdroid` Activity and its helpers, and exports 26 native
methods under `com.zdroid.NativeBridge`. FastEmu would have had to ship
those Kotlin classes and package names for a backend whose features it
doesn't use. The copy keeps the parts a single-window game emulator needs,
about 3,600 of the original's 9,500 lines of Rust, and needs four Java
methods and two native methods from the app.

## What the app provides

The Activity is a `GameActivity` (`androidx.games:games-activity:4.0.0`,
the Java side android-activity 0.6 supports) with these methods, called
from Rust:

| Method | Used for |
| --- | --- |
| `gestureConfiguration(): FloatArray` | touch slop, fling velocity, double-tap and long-press timeouts |
| `keyCharacter(deviceId: Int, keyCode: Int, metaState: Int): Int` | what a hardware key types on its layout |
| `openUrl(url: String)` | `cx.open_url` |
| `setKeepScreenOn(on: Boolean)` | `cx.prevent_idle_sleep` |

and an `io.github.fzn0x.fastemu.NativeBridge` object whose native methods
this crate exports:

| Native method | Used for |
| --- | --- |
| `nativeOnWindowInsetsChanged(left, top, right, bottom: Int)` | the window's safe area, in physical pixels |
| `nativeOpenUrl(url: String)` | files other apps open with FastEmu, delivered to `on_open_urls` |

Path prompts (`cx.prompt_for_paths`) arrive on the channel from
[`path_prompts`], and the app answers them with its own file picker.

## What changed from upstream

Kept with small edits: `dispatcher.rs`, `jni_env.rs`, `events.rs`,
`events/keyboard.rs`, `events/mouse.rs`, `insets.rs`, `open_files.rs`,
`platform.rs`, `touch.rs`, `window.rs` and the crate root. Kept unchanged:
`display.rs`, `keyboard.rs`.

- One window. `open_window` refuses a second window instead of starting an
  `ExtraWindowActivity`, and everything about extra windows is gone
  (`multi_window.rs`, their insets, focus, titles and events).
- No IME bridge (`ime.rs`): no on-screen keyboard, no composition, no dead
  keys. Hardware keys still type what their layout says.
- No trackpad gestures sent over JNI and no glide after a two-finger
  scroll; a trackpad works as a mouse.
- No two-finger gestures in `touch.rs`. Upstream cancelled the first
  finger's touch when a second one landed, which would release a held
  phone key; here each finger is its own touch.
- Removed services: clipboard, credentials (Android Keystore), pointer
  icons, thermal status, Battery Saver, the splash screen, the in-app
  updater, DNS, storage permissions, the documents provider, the Ubuntu
  runtime and its foreground service, askpass, SELinux and pointer
  authentication probes, process hygiene and project import. The matching
  `Platform` methods return neutral answers (no clipboard item, nominal
  thermal state, no credentials).
- A hardware key's release names the key its press named, even when Shift
  or the layout changed in between (`KeysDown` in `events/keyboard.rs`).
  Upstream named the release by the modifiers held at release, so Shift+8
  pressed `*` and released `8`, and the phone's `*` key stayed held.
- The text system loads `/system/fonts` only and falls back to Roboto; the
  port also loaded the runtime's fonts and fell back to Zed's Lilex.
- JNI entry points belong to `io.github.fzn0x.fastemu.NativeBridge`.
- Errors are logged directly instead of through Zed's `util::ResultExt`,
  and `unsafe` blocks carry SAFETY comments.

## Syncing with upstream

Diff the files above against the port's `crates/gpui_android/src` at the
new revision, carry over fixes to the run loop, surface, renderer and
input paths, and move the `gpui` revision in `Cargo.toml` (and the
`[patch.crates-io]` block in the workspace's `Cargo.toml`) to match.
