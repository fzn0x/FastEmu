# Third-party notices

FastEmu's own code is under the GNU General Public License v3.0 or later
(see [LICENSE](LICENSE)). These files and libraries of other people are
bundled in FastEmu's apps or its website, and keep their own licenses.

Whoever adds a font, an image, a game frame, a sound, a vendored source
file or a Java class file taken from elsewhere adds its entry here in the
same pull request, with the author, the license and the files that hold
it. The sections below say what goes in each.

## Fonts

Each entry names the font, its copyright line, its license, where it comes
from, which files hold it and what FastEmu changed in it. Fonts are under
the SIL Open Font License, Version 1.1, with its text next to the font
file, or under another license that allows bundling.

**DejaVu LGC fonts, version 2.37.** `crates/fastemu-gfx/fonts/`:
`DejaVuLGCSans.ttf`, `DejaVuLGCSans-Bold.ttf`, `DejaVuLGCSansMono.ttf` and
`DejaVuLGCSansMono-Bold.ttf`, unmodified, from the DejaVu fonts team's
release `dejavu-lgc-fonts-ttf-2.37.zip`. They draw the text of MIDP games.
Copyright (c) 2003 Bitstream, Inc., with glyphs from the Arev fonts,
copyright (c) 2006 Tavmjong Bah; the DejaVu changes are in the public
domain. Bitstream Vera Fonts license and Arev Fonts license, in full in
[License texts](#dejavu-fonts-license).

## Icons and images

Each entry names the set, its copyright line, its license and the files
that copy from it. The FastEmu logo and the app icons were made for
FastEmu and are under FastEmu's license.

**Lucide icons, version 1.72.0** (`lucide-static`). `crates/fastemu-ui/assets/icons/*.svg`,
unmodified, drawn in the desktop and Android apps. Copyright (c) Lucide
Icons and Contributors, ISC License; icons derived from Feather are
copyright (c) Cole Bemis, MIT License. Both in full in
[License texts](#lucide-icons-license).

The wordmark in the logo is set in [Inter](https://github.com/rsms/inter)
(SIL Open Font License 1.1) and outlined to paths; no font file is
bundled for it.

## Game frames

Each entry names the game, its author, its license and the files that
show it. These are pictures of free games, recorded in FastEmu with its
headless runner (`fastemu run --record`). FastEmu bundles none of the
games themselves.

| Game | Author | License |
| --- | --- | --- |
| [Stick Fight](https://github.com/Dahmalahi/STICK-FIGHT-OFFICIEL-REPO) | Dash Animation V2 | MIT |
| [Shooter](https://github.com/RehanSaeed/Shooter) | Muhammad Rehan Saeed | MIT |
| [BombMan](https://github.com/yaricom/bombman-RL-AI-J2ME) | Iaroslav Omelianenko | MIT |
| Traffic Dash, Tower Guard, Star Defender and Block Fall, from [J2ME 100 Games](https://github.com/agneay/j2me-100-games) | Agneay B Nair and the J2ME Game Archive contributors | MIT |

`site/src/games/*.webp` hold the frames of the website's first two
cards, and `docs/images/games.png`, the picture at the top of the README,
shows Stick Fight, Shooter, BombMan, Traffic Dash, Tower Guard and Star
Defender. The MIT License texts, with each game's copyright line, are in
[site/src/games/LICENSES.txt](site/src/games/LICENSES.txt), which the
website publishes beside the frames.

## Sounds and instruments

Each entry names the sound bank or the samples, their license and the
files that hold them.

No sounds are bundled yet.

## Source code taken from other projects

Code copied into the tree, rather than used as a dependency, with the
project it comes from, its license and the files. Behavior that FastEmu
follows without copying code, such as that of MicroEmulator and
J2ME Loader, is credited in the [README](README.md#credits) and does not
belong here. Files that copy no code, but name another project's classes
so that a tool compiles against them, are listed too.

**gpui_android.** `crates/gpui_android/` is a modified copy of
`crates/gpui_android` from the [Zed Android port](https://github.com/Dylanmurzello/zed-android-port)
by Dylan Murzello, revision 746f088ee8d6b8b273354826757dd09e74565854,
which builds on GPUI by Zed Industries. Copyright 2022 - 2025 Zed
Industries, Inc. and the Zed Android port contributors; modifications
copyright the FastEmu contributors. Apache License, Version 2.0, in full
in [License texts](#apache-license-20). What FastEmu changed is in
`crates/gpui_android/README.md` and at the top of each changed file; its
`NOTICE` is in the same folder.

**MicroEmulator's class names, for the benchmark.**
`tools/xtask/bench/microemu-api/` holds 26 signature-only Java files.
23 name public classes and members of MicroEmulator 2.0.4
(`org.microemu`), and 3 name the MIDP classes its jar holds
(`javax.microedition`). They let MeBench, the benchmark's wrapper for
MicroEmulator, compile where MicroEmulator is not installed, such as in
CI. They hold no MicroEmulator code: only the names and types MeBench
uses, a few constant values, and methods that only throw. They are not
in FastEmu's apps, and the bench compiles MeBench again against the
real `microemulator.jar` before it measures. MicroEmulator is by Bartek
Teodorczyk and its contributors, under the GNU Lesser General Public
License, version 2.1 or any newer version, or the Apache License,
Version 2.0, at the user's choice.

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

Two libraries carry notices that must travel with FastEmu:

- **amr-nb** (Apache License 2.0) decodes AMR sound. Its notice:

  ```text
  amr-nb: a Rust port of the AMR-NB codec from opencore-amr 0.1.6.

  This product includes software derived from opencore-amr
  (https://sourceforge.net/projects/opencore-amr/):
    Copyright (C) 1998-2009 PacketVideo
    Licensed under the Apache License, Version 2.0.

  Portions are derived from the following 3GPP standard:
    3GPP TS 26.073, ANSI-C code for the Adaptive Multi-Rate (AMR) speech codec
    (C) 2004, 3GPP Organizational Partners (ARIB, ATIS, CCSA, ETSI, TTA, TTC)
    As stated in the opencore-amr sources: "Permission to distribute, modify and use this
    file under the standard license terms listed above has been obtained from the copyright
    holder."
  ```

- **webpki-roots** bundles Mozilla's list of trusted certificate
  authorities for HTTPS connections, under the Community Data License
  Agreement Permissive 2.0 (<https://cdla.dev/permissive-2-0/>).

## License texts

### DejaVu fonts license

```text
Fonts are (c) Bitstream (see below). DejaVu changes are in public domain.
Glyphs imported from Arev fonts are (c) Tavmjong Bah (see below)


Bitstream Vera Fonts Copyright
------------------------------

Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved. Bitstream Vera is
a trademark of Bitstream, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of the fonts accompanying this license ("Fonts") and associated
documentation files (the "Font Software"), to reproduce and distribute the
Font Software, including without limitation the rights to use, copy, merge,
publish, distribute, and/or sell copies of the Font Software, and to permit
persons to whom the Font Software is furnished to do so, subject to the
following conditions:

The above copyright and trademark notices and this permission notice shall
be included in all copies of one or more of the Font Software typefaces.

The Font Software may be modified, altered, or added to, and in particular
the designs of glyphs or characters in the Fonts may be modified and
additional glyphs or characters may be added to the Fonts, only if the fonts
are renamed to names not containing either the words "Bitstream" or the word
"Vera".

This License becomes null and void to the extent applicable to Fonts or Font
Software that has been modified and is distributed under the "Bitstream
Vera" names.

The Font Software may be sold as part of a larger software package but no
copy of one or more of the Font Software typefaces may be sold by itself.

THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT,
TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL BITSTREAM OR THE GNOME
FOUNDATION BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING
ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL DAMAGES,
WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF
THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM OTHER DEALINGS IN THE
FONT SOFTWARE.

Except as contained in this notice, the names of Gnome, the Gnome
Foundation, and Bitstream Inc., shall not be used in advertising or
otherwise to promote the sale, use or other dealings in this Font Software
without prior written authorization from the Gnome Foundation or Bitstream
Inc., respectively. For further information, contact: fonts at gnome dot
org.

Arev Fonts Copyright
------------------------------

Copyright (c) 2006 by Tavmjong Bah. All Rights Reserved.

Permission is hereby granted, free of charge, to any person obtaining
a copy of the fonts accompanying this license ("Fonts") and
associated documentation files (the "Font Software"), to reproduce
and distribute the modifications to the Bitstream Vera Font Software,
including without limitation the rights to use, copy, merge, publish,
distribute, and/or sell copies of the Font Software, and to permit
persons to whom the Font Software is furnished to do so, subject to
the following conditions:

The above copyright and trademark notices and this permission notice
shall be included in all copies of one or more of the Font Software
typefaces.

The Font Software may be modified, altered, or added to, and in
particular the designs of glyphs or characters in the Fonts may be
modified and additional glyphs or characters may be added to the
Fonts, only if the fonts are renamed to names not containing either
the words "Tavmjong Bah" or the word "Arev".

This License becomes null and void to the extent applicable to Fonts
or Font Software that has been modified and is distributed under the 
"Tavmjong Bah Arev" names.

The Font Software may be sold as part of a larger software package but
no copy of one or more of the Font Software typefaces may be sold by
itself.

THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL
TAVMJONG BAH BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.

Except as contained in this notice, the name of Tavmjong Bah shall not
be used in advertising or otherwise to promote the sale, use or other
dealings in this Font Software without prior written authorization
from Tavmjong Bah. For further information, contact: tavmjong @ free
. fr.

TeX Gyre DJV Math
-----------------
Fonts are (c) Bitstream (see below). DejaVu changes are in public domain.

Math extensions done by B. Jackowski, P. Strzelczyk and P. Pianowski
(on behalf of TeX users groups) are in public domain.

Letters imported from Euler Fraktur from AMSfonts are (c) American
Mathematical Society (see below).
Bitstream Vera Fonts Copyright
Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved. Bitstream Vera
is a trademark of Bitstream, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of the fonts accompanying this license (“Fonts”) and associated
documentation
files (the “Font Software”), to reproduce and distribute the Font Software,
including without limitation the rights to use, copy, merge, publish,
distribute,
and/or sell copies of the Font Software, and to permit persons  to whom
the Font Software is furnished to do so, subject to the following
conditions:

The above copyright and trademark notices and this permission notice
shall be
included in all copies of one or more of the Font Software typefaces.

The Font Software may be modified, altered, or added to, and in particular
the designs of glyphs or characters in the Fonts may be modified and
additional
glyphs or characters may be added to the Fonts, only if the fonts are
renamed
to names not containing either the words “Bitstream” or the word “Vera”.

This License becomes null and void to the extent applicable to Fonts or
Font Software
that has been modified and is distributed under the “Bitstream Vera”
names.

The Font Software may be sold as part of a larger software package but
no copy
of one or more of the Font Software typefaces may be sold by itself.

THE FONT SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT,
TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL BITSTREAM OR THE GNOME
FOUNDATION
BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING ANY GENERAL,
SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL DAMAGES, WHETHER IN AN
ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF THE USE OR
INABILITY TO USE
THE FONT SOFTWARE OR FROM OTHER DEALINGS IN THE FONT SOFTWARE.
Except as contained in this notice, the names of GNOME, the GNOME
Foundation,
and Bitstream Inc., shall not be used in advertising or otherwise to promote
the sale, use or other dealings in this Font Software without prior written
authorization from the GNOME Foundation or Bitstream Inc., respectively.
For further information, contact: fonts at gnome dot org.

AMSFonts (v. 2.2) copyright

The PostScript Type 1 implementation of the AMSFonts produced by and
previously distributed by Blue Sky Research and Y&Y, Inc. are now freely
available for general use. This has been accomplished through the
cooperation
of a consortium of scientific publishers with Blue Sky Research and Y&Y.
Members of this consortium include:

Elsevier Science IBM Corporation Society for Industrial and Applied
Mathematics (SIAM) Springer-Verlag American Mathematical Society (AMS)

In order to assure the authenticity of these fonts, copyright will be
held by
the American Mathematical Society. This is not meant to restrict in any way
the legitimate use of the fonts, such as (but not limited to) electronic
distribution of documents containing these fonts, inclusion of these fonts
into other public domain or commercial font collections or computer
applications, use of the outline data to create derivative fonts and/or
faces, etc. However, the AMS does require that the AMS copyright notice be
removed from any derivative versions of the fonts which have been altered in
any way. In addition, to ensure the fidelity of TeX documents using Computer
Modern fonts, Professor Donald Knuth, creator of the Computer Modern faces,
has requested that any alterations which yield different font metrics be
given a different name.

$Id$
```

### Lucide icons license

```text
ISC License

Copyright (c) 2026 Lucide Icons and Contributors

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

---

The following Lucide icons are derived from the Feather project:

airplay, alert-circle, alert-octagon, alert-triangle, aperture, arrow-down-circle, arrow-down-left, arrow-down-right, arrow-down, arrow-left-circle, arrow-left, arrow-right-circle, arrow-right, arrow-up-circle, arrow-up-left, arrow-up-right, arrow-up, at-sign, calendar, cast, check, chevron-down, chevron-left, chevron-right, chevron-up, chevrons-down, chevrons-left, chevrons-right, chevrons-up, circle, clipboard, clock, code, columns, command, compass, corner-down-left, corner-down-right, corner-left-down, corner-left-up, corner-right-down, corner-right-up, corner-up-left, corner-up-right, crosshair, database, divide-circle, divide-square, dollar-sign, download, external-link, feather, frown, hash, headphones, help-circle, info, italic, key, layout, life-buoy, link-2, link, loader, lock, log-in, log-out, maximize, meh, minimize, minimize-2, minus-circle, minus-square, minus, monitor, moon, more-horizontal, more-vertical, move, music, navigation-2, navigation, octagon, pause-circle, percent, plus-circle, plus-square, plus, power, radio, rss, search, server, share, shopping-bag, sidebar, smartphone, smile, square, table-2, tablet, target, terminal, trash-2, trash, triangle, tv, type, upload, x-circle, x-octagon, x-square, x, zoom-in, zoom-out

The MIT License (MIT) (for the icons listed above)

Copyright (c) 2013-present Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

### Apache License 2.0

```text
Copyright 2022 - 2025 Zed Industries, Inc.


   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at


       http://www.apache.org/licenses/LICENSE-2.0


   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.




Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/


   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION


   1. Definitions.


      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.


      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.


      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.


      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.


      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.


      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.


      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).


      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.


      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."


      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.


   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.


   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.


   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:


      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and


      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and


      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and


      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.


      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.


   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.


   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.


   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.


   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.


   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.


   END OF TERMS AND CONDITIONS
```

