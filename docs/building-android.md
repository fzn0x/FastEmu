# Building the Android app

The Android app is a Gradle project in `android/` around one native library,
`libfastemu_android.so`, built from `crates/fastemu-android` with
[cargo-ndk](https://github.com/bbqsrc/cargo-ndk). Its UI is GPUI drawn with
Vulkan through `crates/gpui_android`, a trimmed copy of the Zed Android port's
backend (see that crate's README).

The APK carries the library for `arm64-v8a` (phones and tablets) and `x86_64`
(the Android emulator). It needs Android 12 (API 31) or later and a Vulkan
driver.

## What you need

| Tool | Version used | Notes |
| --- | --- | --- |
| Rust | the stable toolchain in `rust-toolchain.toml` (1.99 here) | `rustup target add aarch64-linux-android x86_64-linux-android` |
| cargo-ndk | 4.1.2 | `cargo install cargo-ndk --version 4.1.2 --locked` |
| Android SDK | platform 35, build-tools 35.0.0 | `sdkmanager "platforms;android-35" "build-tools;35.0.0"` |
| Android NDK | 28.2.13676358 (r28c) | `sdkmanager "ndk;28.2.13676358"` |
| JDK | 17 | the Android Gradle Plugin 8.7 needs 17 or later |

Gradle itself comes from the wrapper (`android/gradlew`, Gradle 8.10.2,
checked against its SHA-256), which downloads it on first use.

The Zed Android port documents NDK r27. FastEmu builds with r28 without
changes: the library links, loads and runs on the emulator, and r28 aligns
its segments to 16 KB pages by default, which Android 15 devices with 16 KB
pages require.

## Environment

Point the tools at the SDK, the NDK and the JDK. In Git Bash on Windows:

```sh
export ANDROID_HOME="$LOCALAPPDATA/Android/Sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export JAVA_HOME="C:/Program Files/Microsoft/jdk-17.0.11.9-hotspot"
```

On Linux and macOS, the same three variables with your own paths. Gradle also
reads the SDK's path from `android/local.properties` (`sdk.dir=...`), which
Android Studio writes and git ignores. `cargo` and `cargo ndk` must be on the
`PATH` Gradle runs with.

## Build the APK

```sh
cd android
./gradlew assembleDebug        # or gradlew.bat assembleDebug in cmd and PowerShell
```

The `cargoBuild` task runs before every build. It is the same as running this
from the repository's root:

```sh
cargo ndk -t arm64-v8a -t x86_64 -P 31 \
  -o android/app/src/main/jniLibs \
  build --release -p fastemu-android
```

The library is always built with the release profile: a debug build of GPUI
is several times larger and too slow to judge. When the libraries in
`android/app/src/main/jniLibs` are already current, skip that step with
`./gradlew assembleDebug -Pfastemu.skipCargo`.

A first build takes about five minutes for both ABIs on a 20-thread machine
with six cargo jobs; after a change to FastEmu's own crates, about ten seconds
each. Gradle's first run downloads Gradle and its plugins, about a minute.

`assembleRelease` builds a release APK, shrunk with R8 and, for now, signed
with the debug key like the debug build.

| Output | Path | Size |
| --- | --- | --- |
| Debug APK | `android/app/build/outputs/apk/debug/app-debug.apk` | 33.5 MB from a clean build |
| Release APK | `android/app/build/outputs/apk/release/app-release.apk` | 27.2 MB |
| arm64-v8a library | `android/app/src/main/jniLibs/arm64-v8a/libfastemu_android.so` | 12.3 MB (5.3 MB gzipped) |
| x86_64 library | `android/app/src/main/jniLibs/x86_64/libfastemu_android.so` | 13.9 MB (5.7 MB gzipped) |

Gradle stores the libraries uncompressed so Android can map them straight from
the APK. An arm64-only build (`-t arm64-v8a` and `abiFilters`) halves the APK.

## Run it

```sh
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n io.github.fzn0x.fastemu/.MainActivity
adb logcat -s fastemu fastemu_activity fastemu_files AndroidRuntime
```

The app starts the mock launcher's test pattern ("FastEmu demo"). Its keypad
and a hardware keyboard drive the square: the arrows and 2, 4, 6, 8 move it,
OK and 5 change its colour, and a tap on the screen moves it there. "Open game"
shows Android's document picker for `.jar` and `.jad` files; "Open with
FastEmu" in a file manager works too. Either copies the file into the app's
storage and starts it. Back moves the app behind the home screen with the
game paused; it resumes when the app comes back.

## Check it in a headless emulator

The emulator runs without a window, and the app draws through it. Create a
private AVD from an installed x86_64 system image, so your own AVDs stay as
they are:

```sh
export ANDROID_AVD_HOME="$PWD/build/avd"    # any folder
"$ANDROID_HOME/cmdline-tools/latest/bin/avdmanager" create avd \
  -n fastemu_api35 -k "system-images;android-35;google_apis_playstore;x86_64" -d pixel_6
"$ANDROID_HOME/emulator/emulator" -avd fastemu_api35 -port 5580 \
  -no-window -no-audio -gpu swiftshader_indirect -no-snapshot -no-boot-anim &
adb -s emulator-5580 wait-for-device
until [ "$(adb -s emulator-5580 shell getprop sys.boot_completed | tr -d '\r')" = 1 ]; do sleep 5; done

adb -s emulator-5580 install -r android/app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5580 shell am start -W -n io.github.fzn0x.fastemu/.MainActivity
adb -s emulator-5580 exec-out screencap -p > fastemu.png
adb -s emulator-5580 logcat -d | grep -E "fastemu|FATAL|panic"

adb -s emulator-5580 emu kill               # stops only this emulator
```

On Windows the AVD manager is `avdmanager.bat`. With
`-gpu swiftshader_indirect` the emulator still offers wgpu the host GPU
through its Vulkan driver, which GPUI picks; on this machine that was the
host's NVIDIA GPU. A first launch shows Android's "Viewing full screen"
notice over the top of the app once; tap "Got it" or ignore it.

Input can be driven from adb: `input tap X Y` taps,
`input swipe X Y X Y 1500` holds a key for 1.5 seconds, and
`input keyevent KEYCODE_DPAD_LEFT` presses a hardware key.

## Tests and lints

The app's logic that needs neither Android nor GPUI tests on the host:

```sh
cargo test -p fastemu-android
```

Everything, including `gpui_android`'s input tests, also runs on a connected
device or emulator through cargo-ndk's test runner, which pushes the test
binaries with adb:

```sh
ANDROID_SERIAL=emulator-5580 cargo ndk-test -t x86_64 -P 31 -p gpui_android -p fastemu-android
```

The GPUI code only compiles for Android, so lint it for an Android target as
well as the host:

```sh
cargo clippy -p fastemu-android -p gpui_android --all-targets -- -D warnings
cargo ndk -t arm64-v8a -P 31 clippy -p fastemu-android -p gpui_android --all-targets -- -D warnings
```

## Signing

Both build types are signed with the Android SDK's debug key, which Gradle
creates in `~/.android/debug.keystore`. Publishing needs a release key and a
`signingConfigs` entry that reads it from a file git ignores
(`android/app/signing.properties` and `*.keystore` are already ignored).
