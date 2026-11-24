# 10. Licenses

What FastEmu (GPL-3.0-or-later) may reuse from each project, and the
attribution it needs. This is an engineering summary, not legal advice;
license texts were read in the repositories at the commits named in the
[index](../emulator-notes.md#sources).

## Ground rules

- Facts about behaviour (which key code a phone sends, which exception
  a method throws, which game needs what) are not protected. These notes
  describe behaviour and may be used freely.
- Translating code from Java to Rust, or reusing its structure closely,
  makes a derivative work: the source license applies to the result.
- GPL-3.0-or-later can take in code under GPL-3.0(-or-later),
  LGPL-2.1-or-later, Apache-2.0, MIT, BSD and MPL-2.0 (MPL through its
  secondary-license clause, unless a file says "Incompatible With
  Secondary Licenses"). It cannot take in EPL-1.0 code or GPL-2.0-only
  code.

## Per project

| Project | License | May FastEmu reuse code? | Must keep |
| --- | --- | --- | --- |
| J2ME Loader | Apache-2.0 for the project; per-file exceptions below | Yes, the Apache-2.0 and MicroEmulator-derived Java code | copyright lines, the Apache-2.0 text, a note of changes in each modified file |
| freej2me-plus | GPL-3.0-or-later; bundled parts MIT, BSD-3, LGPL-2.1, Apache-2.0 | Yes | GPL notices, "This file is part of FreeJ2ME" style attribution, the original copyright lines |
| freej2me | GPL-3.0-or-later; ASM is BSD-3 | Yes | as above |
| MicroEmulator | each file under LGPL-2.1-or-later **or** Apache-2.0, the user's choice | Yes, under either | the file's copyright lines and the chosen license's notice |
| wie | MIT | Yes | the MIT copyright and permission notice |
| RustJava | MIT | Yes | the MIT copyright and permission notice |

### J2ME Loader in detail

- Project license: [Apache-2.0](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/LICENSE).
  Java files carry Apache-2.0 headers naming Kulikov Dmitriy, Nikita
  Shakarun, Yury Kharchenko and others. [code]
- Files from MicroEmulator (for example the `org.microemu.android.asm`
  rewriting classes and `com.nokia.mid.ui.FullCanvas`) keep
  MicroEmulator's LGPL-2.1+ / Apache-2.0 header. [code]
- `javax.microedition.shell.custom.Timer` is Apache Harmony code under
  Apache-2.0, ASF header. [code]
- The bundled `dx` tool is AOSP code, Apache-2.0; Sonivox EAS is
  Apache-2.0 (Sonic Network). [code]
- **The M3G native code under `app/src/main/cpp/m3g` is Nokia's Symbian
  code under EPL-1.0** ([header](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/cpp/m3g/src)).
  EPL-1.0 is not compatible with the GPL; FastEmu must not copy or
  translate it. [code]
- TinySoundFont is MIT (listed in the app's licenses page; the source
  folder is a submodule that was empty in the clone). [code]
- The app's [licenses page](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/assets/licenses.html)
  also credits "FreeJ2ME M3D(O) implementation, Copyright (C) 2018 David
  Richardson" without naming a license; the current
  `com.nokia.mid.m3d` files carry Apache-2.0 headers. Treat any code
  traced back to FreeJ2ME as GPL-3.0-or-later. [code]
- No `NOTICE` file was found in the tree, so Apache-2.0 section 4(d)
  adds nothing beyond the license text and the copyright lines. [code]

Apache-2.0 code may be included in a GPL-3.0 work. Keep each file's
copyright lines, include the Apache-2.0 text in the distribution, and
mark files that were changed (section 4(b)).

### freej2me-plus and freej2me in detail

- [LICENSE](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/LICENSE):
  "FreeJ2ME Copyright (c) 2017 David Richardson [Recompile]", GPL-3.0 or
  any later version; the same file holds the ObjectWeb ASM BSD-3 text.
  Java files carry the "This file is part of FreeJ2ME" GPL header. [code]
- Bundled pieces with their own licenses, from its README and headers:
  JLayer (LGPL-2.1), ASM (BSD-3), the libretro API (MIT), Roman Lahin's
  MascotME Mascot Capsule renderer (MIT), and a few Vodafone classes
  under Apache-2.0. [code]
- Several freej2me-plus classes say they are "ported from J2ME-Loader"
  (for example the Vodafone key state code and the catch-block logging);
  those carry J2ME Loader's Apache-2.0 obligations as well. [code]

Code taken from either project goes into FastEmu under GPL-3.0-or-later
with the original copyright lines kept.

### MicroEmulator in detail

[COPYING](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/COPYING):
"each file may be used under either the LGPL or the" Apache License
(the text says "MPL" in one sentence, which looks like a slip; the
headers say Apache-2.0). Most files have the dual header naming Bartek
Teodorczyk and others. A few files, such as `HttpsConnection.java` and
`MutableImage.java`, have no license header; they fall under the
project statement. [code] Either choice works for FastEmu; Apache-2.0 is
simpler to document.

### wie and RustJava in detail

Both are MIT, "Copyright 2020 Inseok Lee"
([wie LICENSE](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/LICENSE),
[RustJava LICENSE](https://github.com/dlunch/RustJava/blob/df24aaa7a440815b12a96e2d7845fec2399cfbf3/LICENSE)). [code]
Rust code can be copied directly; keep the copyright and permission
notice with it. wie's SMAF support uses the separate `smaf` and
`smaf-player` crates; check their licenses before depending on them.
[unverified]

## How FastEmu should attribute

1. In each Rust file that contains translated or copied code, a header
   comment naming the source project, the source file, the commit, the
   copyright holders and the license, for example:
   `// Derived from J2ME Loader, KeyMapper.java at e270ed91e67b,`
   `// Copyright 2019-2023 Yury Kharchenko, Apache-2.0.`
2. A third-party notices file at the repository root (for example
   `THIRD-PARTY-NOTICES.md` or a `LICENSES/` folder) with the full texts
   of Apache-2.0, MIT, BSD-3 and LGPL-2.1 for every project code came
   from, and the copyright lines.
3. The app's About screen links to that file, as J2ME Loader's
   licenses page does.
4. Dependencies added through Cargo are attributed by their crate
   licenses; `cargo about` or `cargo deny` can check them in CI.

## What not to copy

- J2ME Loader's EPL-1.0 M3G code (`app/src/main/cpp/m3g`).
- Anything from KEmulator; its license is not stated in the repository
  checked ([shinovon/KEmulator](https://github.com/shinovon/KEmulator)
  has no license in the GitHub API). [code]
- Game files, sound banks and fonts from any of the repositories unless
  their own licenses allow it (J2ME Loader ships Roboto, Apache-2.0).
