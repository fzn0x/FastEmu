# Third-party notices

FastEmu's own code is under the GNU General Public License v3.0 or later
(see [LICENSE](LICENSE)). These files and libraries of other people are
bundled in FastEmu's apps or its website, and keep their own licenses.

Whoever adds a font, an image, a sound, a vendored source file or a
Java class file taken from elsewhere adds its entry here in the same pull
request, with the author, the license and the files that hold it. The
sections below say what goes in each.

## Fonts

Each entry names the font, its copyright line, its license, where it comes
from, which files hold it and what FastEmu changed in it. Fonts are under
the SIL Open Font License, Version 1.1, with its text next to the font
file, or under another license that allows bundling.

No fonts are bundled yet.

## Icons and images

Each entry names the set, its copyright line, its license and the files
that copy from it. The FastEmu logo and the app icons were made for
FastEmu and are under FastEmu's license.

No third-party images are bundled yet.

## Sounds and instruments

Each entry names the sound bank or the samples, their license and the
files that hold them.

No sounds are bundled yet.

## Source code taken from other projects

Code copied into the tree, rather than used as a dependency, with the
project it comes from, its license and the files. Behavior that FastEmu
follows without copying code, such as that of MicroEmulator and
J2ME Loader, is credited in the [README](README.md#credits) and does not
belong here.

No code is copied from other projects yet.

## Libraries

FastEmu's Rust crates link libraries from crates.io under permissive
licenses (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode) and, for a few, the
Mozilla Public License 2.0. Their names, versions and licenses are in
`Cargo.lock`, and each crate's own license file is in its source.
Continuous integration checks the license of every one against
[deny.toml](deny.toml).

The apps are built on [GPUI](https://github.com/zed-industries/zed), the
UI framework of the Zed editor. The `gpui` crate is under the Apache License, Version 2.0:
<https://www.apache.org/licenses/LICENSE-2.0>. The Android app uses
Dylan Murzello's port of Zed to Android for GPUI on Android.

The Android app's Gradle project lists its own libraries in
`android/app/build.gradle.kts`.
