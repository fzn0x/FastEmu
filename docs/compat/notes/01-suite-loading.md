# 1. Suite loading

How the emulators open a `.jar` (and its `.jad`), read the attributes,
pick the MIDlet, choose an encoding, find resources and load classes.

Link bases used below:

- JL = J2ME Loader at `e270ed91e67b`
- FP = freej2me-plus at `aab3350832c0`
- FJ = freej2me at `eb4ac87e9a0b`
- ME = MicroEmulator at `e47b656d983d`
- WIE = wie at `eb5b809d693e`, RJ = RustJava at `df24aaa7a440`

## Broken JAR and ZIP files

Real game JARs are often damaged or built by odd tools. What the
emulators do:

- **Central directory failures.** J2ME Loader first wrapped
  `java.util.zip.ZipFile` and, when opening failed, fell back to
  `ZipInputStream`, which walks the local file headers and ignores the
  central directory ([commit 2b628b40](https://github.com/nikita36078/J2ME-Loader/commit/2b628b40),
  "Improve jar compatibility"). It later moved to zip4j
  ([commit 7da67601](https://github.com/nikita36078/J2ME-Loader/commit/7da67601)),
  which it still uses. [code]
- **Entry names that are not UTF-8.** Java's `ZipFile` decodes names as
  UTF-8 and throws on bad bytes. J2ME Loader switched to ISO-8859-1 so
  every byte maps to one char and nothing throws
  ([commit f3ca8aad](https://github.com/nikita36078/J2ME-Loader/commit/f3ca8aad)).
  Chinese and Russian JAR tools wrote names in GBK or CP1251. [code]
- **Case in entry names.** J2ME Loader's zip wrapper returns the first
  exact match and otherwise the first case-insensitive match, skipping
  entries with empty names
  ([ZipFile.getFileHeader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/ru/woesss/util/zip/ZipFile.java#L32)).
  The same lookup serves the manifest, classes and resources. [code]
- **`.CLASS` in upper case.** The dexer treats any entry whose name ends
  in `.class` without regard to case as a class
  ([commit b2b7354f](https://github.com/nikita36078/J2ME-Loader/commit/b2b7354f)). [code]
- **Bad PNG checksums** inside the JAR are handled at image decode time,
  see [Graphics](04-graphics.md#image-creation-and-decoding).
- **Duplicate entries.** No emulator handles them on purpose. J2ME
  Loader's lookup returns the first exact match in central directory
  order. [code] What real phones did with duplicates is unknown.
  [unverified]
- **Strict readers.** wie reads the whole archive with the `zip` crate
  and fails the launch on the first bad entry
  ([extract_zip](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-backend/src/lib.rs#L71)).
  RustJava's `ZipFile` uses `by_name`, an exact match. [code] These are
  the behaviours FastEmu should avoid.
- **Paths with `!` and non-ASCII characters.** freej2me-plus loads JARs
  through `jar:` URLs, so a `!` in the folder name made Java look inside
  the wrong file ([issue 49](https://github.com/TASEmulators/freej2me-plus/issues/49));
  non-ASCII folder names broke saves on Windows
  ([issue 136](https://github.com/TASEmulators/freej2me-plus/issues/136)). [tracker]
  FastEmu should keep host paths as `Path`/`OsString` and never build
  URL strings from them.

**For FastEmu:** read the central directory leniently; if it is missing
or inconsistent, scan local file headers; keep entry names as raw bytes
and compare them as Latin-1 (byte for byte), with a case-insensitive
fallback; ignore CRC mismatches; accept stored and deflated entries
with or without data descriptors; on duplicates take the first entry.

## Manifest and JAD parsing

### Finding the manifest

- J2ME Loader: `META-INF/MANIFEST.MF` through the case-insensitive
  lookup above ([commit b2cfdecd](https://github.com/nikita36078/J2ME-Loader/commit/b2cfdecd),
  "Ignore manifest filename case"). [code]
- freej2me: tries `META-INF/MANIFEST.MF`, `meta-inf/MANIFEST.MF`,
  `META-INF/manifest.fm`, `meta-inf/manifest.fm`
  ([MIDletLoader.loadManifest](https://github.com/hex007/freej2me/blob/eb4ac87e9a0b212ba701691eb67ee4892b2b1173/src/org/recompile/mobile/MIDletLoader.java#L165)).
  Lower-case `meta-inf` is "very common on Chinese games"
  ([freej2me issue 62](https://github.com/hex007/freej2me/issues/62)). [code] [tracker]
- freej2me-plus: `META-INF/MANIFEST.MF`, then the misspelled
  `META-INF/MANIFEST.FM`, both through a case-insensitive resource search
  ([loadManifest](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L565)). [code]
- wie: `META-INF/MANIFEST.MF` compared without case
  ([J2MEEmulator::from_jar](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-j2me/src/emulator.rs#L47)). [code]

### Line format

The JAR manifest format wraps long lines at 72 bytes: a line that starts
with one space continues the previous value. Nokia's own SDK wrote
`MIDlet-1` values split in the middle of the class name
(`...button.ButtonM` + CR LF + space + `idlet`,
[freej2me issue 62](https://github.com/hex007/freej2me/issues/62)). [tracker]

| Rule | J2ME Loader | freej2me-plus | freej2me | wie |
| --- | --- | --- | --- | --- |
| Line ends | splits on any run of `\r` and `\n` | `readLine` | `readLine` | splits on `\n`, trims `\r` |
| Continuation | line without `:` that starts with a space; one space dropped | line that starts with a space; one space dropped | line that starts with a space; `trim()` drops all spaces | line that starts with a space; one space dropped |
| Continuation that contains `:` | becomes a new key (bug) | joined | joined | joined |
| UTF-8 BOM | stripped | not handled | not handled | not handled |
| Key and value | split at first `:`, both trimmed | same | same | same |
| Empty lines | skipped | skipped | kept | skipped |
| Invalid UTF-8 | replaced by the decoder | replaced | platform charset | the whole line is dropped |

Sources: [JL Descriptor.parse](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/jar/Descriptor.java#L155),
[FP parseDescriptorInto](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L433),
[FJ loadManifest](https://github.com/hex007/freej2me/blob/eb4ac87e9a0b212ba701691eb67ee4892b2b1173/src/org/recompile/mobile/MIDletLoader.java#L165),
[WIE J2MEDescriptor::parse](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-j2me/src/emulator.rs#L154). [code]

A manifest that starts with a BOM made Java's own manifest reader fail
with `invalid header field name: ?Manifest-Version`, and the game then
failed with `ClassNotFoundException`
([freej2me issue 226](https://github.com/hex007/freej2me/issues/226)). [tracker]
MicroEmulator tries `java.util.jar.Manifest` first and falls back to its
own line parser when that throws
([JadProperties.read](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/util/JadProperties.java#L145)). [code]

**For FastEmu:** decode as bytes first; strip a BOM; split on CR, LF or
CR LF; a line that starts with a space continues the previous value
whatever it contains, with exactly one space removed; skip blank lines;
split at the first `:` and trim both sides; keep the first value when a
key repeats inside one file (freej2me-plus logs and keeps the existing
value). Never reject a whole descriptor for one bad line.

### Required attributes and checks

- J2ME Loader requires `MIDlet-Name`, `MIDlet-Vendor` and
  `MIDlet-Version` in every descriptor, and `MIDlet-Jar-URL` plus an
  integer `MIDlet-Jar-Size` in a JAD
  ([verify](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/jar/Descriptor.java#L116)).
  It does not compare the size with the real JAR. [code]
- If the JAD and the manifest disagree on name or vendor, J2ME Loader
  shows the manifest and offers to install the JAR alone, ignoring the
  JAD ([InstallerDialog](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/installer/InstallerDialog.java#L306)). [code]
  JSR 118 says such a JAR "MUST NOT be installed". [spec] Users edit
  names in JADs ([J2ME Loader issue 865](https://github.com/nikita36078/J2ME-Loader/issues/865)),
  so FastEmu should warn, not refuse.
- freej2me-plus requires nothing. With no `MIDlet-1` it scans every class
  for the bytes `startApp` and runs the first class that has them
  ([findMainClassInJar](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L285),
  [commit 6abbdee](https://github.com/TASEmulators/freej2me-plus/commit/6abbdee),
  "Support loading jars without a valid manifest file"). [code]
- wie requires a non-empty name and main class from `MIDlet-1`. [code]

### Finding the JAR of a JAD

- J2ME Loader resolves `MIDlet-Jar-URL` against the JAD's folder; if no
  file is there it tries the JAD's name with `.jar`
  ([checkJarFile](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/installer/AppInstaller.java#L368)).
  `http` and `https` JAD links are downloaded, with redirects followed. [code]
- freej2me-plus ignores `MIDlet-Jar-URL` and always uses the JAD's name
  with `.jar`. Opening a JAR also picks up a `.jad` (or a Korean `.msd`)
  of the same name next to it
  ([MobilePlatform](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MobilePlatform.java#L764)). [code]

**For FastEmu:** try `MIDlet-Jar-URL` as a relative path, then its last
path segment, then the JAD's own name with `.jar`. When the user opens a
JAR, use a same-name JAD next to it if one exists.

### Precedence

JSR 118: "If the MIDlet suite is not trusted the value from the
descriptor file will override the value from the manifest file." For
trusted suites the values must be identical. [spec]

- J2ME Loader: manifest first, then `putAll` of the JAD, so the JAD wins
  ([install](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/installer/AppInstaller.java#L298)). [code]
- freej2me-plus: JAD first; a manifest key is added only if the JAD did
  not set it. [code]
- MicroEmulator: `getAppProperty` asks the JAD, then the manifest
  ([Common.getAppProperty](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/Common.java#L159)). [code]
- wie: when a JAD is given, only the JAD is read; the manifest is
  ignored ([from_jad_jar](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-j2me/src/emulator.rs#L40)). [code]
  Games that read manifest-only attributes break under that rule.

FastEmu's `SuiteInfo.properties` puts the descriptor's entries first,
which gives the same result as long as lookups take the first match.

### Attribute name case

JSR 118, application descriptor: "Attribute names are case-sensitive
and MUST match exactly." [spec] The JAR manifest format compares names
without case, as Java SE's `java.util.jar.Attributes.Name` does.
[unverified here] J2ME Loader and freej2me-plus store attributes in a
`HashMap` and look them up with exact case. [code] MicroEmulator reads
both files through `java.util.jar.Manifest` [code], so its lookups
should ignore case. [unverified]

`SuiteInfo::property` in `fastemu-core` ignores case and its doc comment
says MIDP asks for that. The comment is wrong for the descriptor. In
practice ignoring case is the tolerant choice and harms nothing found
here, so the fix is to the comment, not the behaviour.

### Vendor descriptor formats

- KDDI `.kjx`: a `KJX` header, then the JAD, then the JAR, in one file.
  Both J2ME Loader ([parseKjx](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/installer/AppInstaller.java#L173))
  and freej2me-plus split it. [code]
- DoJa `.jam` (Japanese i-appli): `key=value` lines in Shift_JIS;
  freej2me-plus reads it when the manifest has no `MIDlet-1`. [code]
- SK Telecom `.msd`: freej2me-plus treats it as a JAD in EUC-KR. [code]

These are out of FastEmu's MIDP scope but explain some "broken JAD"
reports.

## MIDlet-n parsing

JSR 118: name, icon and class separated by commas; "The lowest value of
<n> MUST be 1 and consecutive ordinals MUST be used. The first missing
entry terminates the list"; leading and trailing spaces are ignored; the
icon may be omitted. [spec]

| | J2ME Loader | freej2me-plus | MicroEmulator | wie |
| --- | --- | --- | --- | --- |
| Count | until the first gap | 1 to 9, gaps allowed | any `MIDlet-<int>` | `MIDlet-1` only |
| Name | text before the first comma | first field | before the first comma | first field |
| Class | text after the last comma | third field | after the second comma | third field |
| Two fields only | name, class | `Name/icon.png, Class`: splits the name at the first `/` | `StringIndexOutOfBoundsException` escapes | class empty, launch fails |
| Icon fallback | `MIDlet-Icon`, else field 2 of `MIDlet-1` | field 2 of `MIDlet-1` | field 2 | `MIDlet-Icon`, else field 2 |
| Icon leading `/` | all stripped | kept | kept | stripped |

Sources: [JL loadMIDletList](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MicroLoader.java#L177),
[JL getIcon](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/woesss/j2me/jar/Descriptor.java#L221),
[FP loadManifest](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L660),
[ME getMidletEntries](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/util/JadProperties.java#L93),
[WIE from_properties](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-j2me/src/emulator.rs#L193). [code]

freej2me-plus added the two-field repair because some manifests miss the
comma between name and icon. [code comment]

**For FastEmu:** class = text after the last comma, trimmed; name = text
before the first comma, trimmed; icon = the middle part if there are
three or more fields, with leading slashes stripped; stop at the first
missing ordinal, but if `MIDlet-1` is missing and higher ones exist, log
it and use them. If no `MIDlet-n` exists at all, offer the classes that
extend `MIDlet` (a superclass walk over the class files is more reliable
than freej2me-plus's byte search).

## Encodings

Three separate encoding questions come up:

1. **Descriptor text.** J2ME Loader reads JAD files as UTF-8 and the
   manifest with Android's default charset, which is UTF-8. [code]
   freej2me-plus reads both as UTF-8 unless the game runs in EUC-KR mode
   ([parseDescriptorInto](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L440));
   [commit 7515a1c](https://github.com/TASEmulators/freej2me-plus/commit/7515a1c)
   fixed "Manifest reader when manifest.mf contains chinese". [code]
   Names only matter for display and for the save folder, so a lossy
   decode is acceptable; keep the raw bytes for `getAppProperty`
   comparisons. [unverified as a game requirement]
2. **The `microedition.encoding` property and implicit conversions.**
   J2ME Loader rewrites the game's bytecode so every implicit-charset
   call names the configured encoding: `new String(byte[]...)`,
   `String.getBytes()`, `new InputStreamReader(InputStream)`,
   `new OutputStreamWriter(OutputStream)`, `ByteArrayOutputStream.toString()`
   and `new PrintStream(OutputStream)`
   ([AndroidMethodVisitor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/AndroidMethodVisitor.java#L102)).
   The default is `ISO-8859-1`
   ([system.props](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/assets/defaults/system.props)),
   and an unknown charset name falls back to `ISO-8859-1`
   ([applyConfiguration](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MicroLoader.java#L267)).
   The setting is per game ([commit c6728616](https://github.com/nikita36078/J2ME-Loader/commit/c6728616)). [code]
   freej2me-plus sets `microedition.encoding` to the host JVM's
   `file.encoding` and supports only ISO-8859-1, Shift_JIS and EUC-KR;
   it switches to Shift_JIS by itself when a game loads `com.kddi`,
   `com.j_phone` or DoJa classes ([loadClass](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L984)). [code]
3. **`DataInputStream.readUTF`** is modified UTF-8 and independent of
   the setting; most games store text this way. [unverified as a
   measurement]

**For FastEmu:** `InstanceSettings::encoding` is the right knob. The JVM
must use it for every conversion that takes no charset, not only for
`new String(bytes)`. Support at least ISO-8859-1, UTF-8, windows-1251,
GBK/GB2312, Big5, Shift_JIS, EUC-KR and windows-1250/1252, and map an
unknown name to ISO-8859-1 instead of throwing.

## Resource lookup

What `Class.getResourceAsStream(name)` does in each emulator:

| Step | J2ME Loader | freej2me-plus | MicroEmulator |
| --- | --- | --- | --- |
| `\` to `/` | yes ("Siemens file path") | yes | no |
| Collapse `//` | yes | yes | no |
| Name without leading `/` | prefixed with the package of the `Class` it is called on | tried at the root first, then prefixed with the main MIDlet class's package | prefixed with the package of the `Class` it is called on |
| Leading `/` | stripped | stripped (up to two) | stripped |
| Exact match | yes | yes | yes |
| Case-insensitive fallback | yes | yes | no |
| `resource:` prefix | via `Connector` | removed; Siemens streams return 0 at end | no |
| Locale fallback | no | file names containing the locale are retried with 28 other locale codes | no |
| Missing resource | `null` | `null` (logged) | `null` |

Sources: [JL AppClassLoader.getResourceAsStream](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/AppClassLoader.java#L61),
[FP getMIDletResourceAsStream](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L858),
[FP findResourceInJar](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L786),
[ME MIDletResourceLoader.resolveName](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/util/MIDletResourceLoader.java#L83). [code]

Notes:

- freej2me-plus rewrites `Class.getResourceAsStream` calls to a static
  helper that drops the class argument
  ([Mobile.getResourceAsStream](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/Mobile.java#L390)),
  which is why it guesses with the main class's package. freej2me had a
  report that relative names failed outright
  ([freej2me issue 131](https://github.com/hex007/freej2me/issues/131)). [code] [tracker]
- In the default package (where obfuscators put every class) a relative
  name resolves to the root. [code in all three]
- freej2me-plus's locale fallback exists because Bounce Tales and
  Nature Park ask for `xx` as English
  ([supportedLocales](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L112)). [code comment]
- The Siemens stream that returns 0 instead of -1 at the end
  ([SiemensInputStream](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L1117))
  is only used for names that started with `resource:`. [code] Why
  Siemens games need it is not documented. [unverified]
- J2ME Loader logged the original name when a lookup failed
  ([commit b9b16271](https://github.com/nikita36078/J2ME-Loader/commit/b9b16271)),
  which helps users report missing files. [code]

**For FastEmu:** normalize `\` and repeated `/`; resolve relative names
against the package of the `Class` object the method is called on, as
Java does; strip leading slashes; exact match, then case-insensitive
match; log misses with the original
name at `LogLevel::Warn`. Keep freej2me-plus's "try the root first" as a
fallback after the standard rule fails, since it rescues games that used
paths relative to the root.

## Resource streams

MicroEmulator wraps every resource stream because "All existing Java ME
implementations always read all bytes available in one read call"
([MIDletResourceInputStream](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/util/MIDletResourceInputStream.java)). [code]
J2ME Loader and freej2me-plus read the whole entry into a
`ByteArrayInputStream`. [code] Games do `new byte[in.available()]` and a
single `read(buf)`; a stream that returns short reads or a small
`available()` corrupts their data.

**For FastEmu:** return resource streams backed by the whole decompressed
entry: `available()` is the remaining length, `read(byte[], off, len)`
copies `min(len, remaining)`, `mark`/`reset` work, `skip` is exact.

## Class loading quirks

- **Game classes shadow emulator classes.** J2ME Loader loads classes
  from the game's JAR first and only then from the emulator
  ([AppClassLoader.loadClass](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/AppClassLoader.java#L168),
  [commit bdf3c2ac](https://github.com/nikita36078/J2ME-Loader/commit/bdf3c2ac)).
  An earlier attempt to skip JAR classes that the emulator also provides
  was reverted ([commit 3aae52b8](https://github.com/nikita36078/J2ME-Loader/commit/3aae52b8)). [code]
  Games bundle their own copies of small vendor or utility classes.
  freej2me-plus does the opposite for any name under `java.`, `javax.`,
  `com.nokia`, `com.siemens`, `com.motorola` and about 30 other
  prefixes; it notes this breaks a game whose own package starts with
  `com.nokia` ([loadClass](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L984)). [code]
  For FastEmu: `java.*` and `javax.*` always come from the runtime (a
  game must not replace `java.lang.String`); everything else prefers the
  JAR.
- **Class name must match the entry path.** J2ME Loader skips a class
  whose internal name differs from its path
  ([AndroidProducer.instrument](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/AndroidProducer.java)). [code]
  FastEmu loads by name, so it should look up the entry from the name and
  check the class's own name, logging a mismatch instead of failing the
  suite.
- **Obfuscated names.** Obfuscators produce classes and members named
  `a`, `b`, `if`, `do` and names that differ only in case (`a.class` and
  `A.class`). They are legal in class files. On a case-insensitive host
  file system they collide, so never extract a JAR to disk. [unverified
  as observed in these trackers; follows from the zip lookups above]
- **CLDC lacks `Thread.stop()`, `suspend()`, `resume()`, `destroy()`.**
  Games declare methods with those names in `Thread` subclasses. On a
  Java SE runtime they override final methods: Android rejects the class
  with `LinkageError`, so J2ME Loader renames them
  ([MethodRenamer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/MethodRenamer.java#L45));
  freej2me-plus prefixes them with `_`
  ([visitMethod](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletLoader.java#L1324)). [code]
  FastEmu's `java.lang.Thread` must follow CLDC 1.1 and not declare them.
  The same applies to every class: offer the CLDC/MIDP API surface, not a
  Java SE superset that can clash with game methods.
- **Boolean returns.** JVMS says `ireturn` from a `boolean` method keeps
  only the low bit. Dalvik's verifier rejected such code, so J2ME Loader
  inserts `& 1` ([AndroidMethodVisitor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/AndroidMethodVisitor.java#L69));
  it fixed VerifyErrors in SimCity Societies and other games
  ([J2ME Loader issue 194](https://github.com/nikita36078/J2ME-Loader/issues/194)). [code] [tracker]
  An interpreter should narrow `boolean`, `byte`, `char` and `short`
  stores and returns as the JVMS says, and should not reject methods for
  stack-map or type-inference reasons: CLDC 1.1 class files often have no
  `StackMap`, and obfuscated ones carry dead or odd code
  ([J2ME Loader issue 814](https://github.com/nikita36078/J2ME-Loader/issues/814)). [tracker]
- **`startApp` declared in a superclass.** freej2me looked the method up
  with `getDeclaredMethod` on the main class and crashed when the game
  inherited a `protected final startApp` from its own base class
  ([freej2me issue 6](https://github.com/hex007/freej2me/issues/6));
  freej2me-plus now walks the superclasses. [tracker] [code] FastEmu
  should call lifecycle methods by virtual dispatch.
- **Main class constructor.** JSR 118 requires a public no-argument
  constructor. [spec] J2ME Loader and freej2me-plus call
  `setAccessible(true)` so non-public constructors work too. [code]
- **Missing classes.** A reference to an API the emulator lacks must
  fail at first use with `NoClassDefFoundError`, not at load time, so
  games that probe optional APIs inside `try` blocks keep running. See
  [Crash causes](09-crashes-and-settings.md#the-20-most-common-crash-causes).
