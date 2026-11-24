# Java ME API inventory

These files list the Java ME APIs that J2ME games call, so that the
people and agents who write FastEmu's Java classes in Rust never have to
guess a signature, a constant or an exception. Every class, constant,
constructor, method and field comes from a primary source (the JSR or
vendor javadoc), and every constant value was checked against compiled
stub classes and the source of the open emulators J2ME Loader,
freej2me-plus and MicroEmulator. Each file names its sources, with links
pinned to the exact commit or release that was read.

## Files

| File | Covers | Classes | Members |
|---|---|---|---|
| [cldc-java-lang.md](cldc-java-lang.md) | `java.lang`, `java.lang.ref` (CLDC 1.1), plus MIDP's `IllegalStateException` | 44 | 338 |
| [cldc-java-util.md](cldc-java-util.md) | `java.util` (CLDC 1.1), plus MIDP's `Timer` and `TimerTask` | 12 | 141 |
| [cldc-java-io.md](cldc-java-io.md) | `java.io` (CLDC 1.1) | 18 | 186 |
| [midp-midlet.md](midp-midlet.md) | `javax.microedition.midlet`, the life cycle, JAD and manifest attributes | 2 | 12 |
| [midp-lcdui.md](midp-lcdui.md) | `javax.microedition.lcdui` | 27 | 452 |
| [midp-lcdui-game.md](midp-lcdui-game.md) | `javax.microedition.lcdui.game` | 5 | 74 |
| [midp-rms.md](midp-rms.md) | `javax.microedition.rms` | 10 | 55 |
| [midp-io.md](midp-io.md) | `javax.microedition.io`: the CLDC connection framework and MIDP 2.0's connections | 19 | 130 |
| [midp-media.md](midp-media.md) | `javax.microedition.media` and its `control` and `protocol` packages: the MIDP 2.0 audio subset and all of JSR-135 | 24 | 161 |
| [midp-pki.md](midp-pki.md) | `javax.microedition.pki` | 2 | 26 |
| [nokia.md](nokia.md) | Nokia UI API 1.1 (`com.nokia.mid.ui`, `com.nokia.mid.sound`) and later additions | 6 | 71 |
| [vendors.md](vendors.md) | Siemens, Samsung, Motorola, Sony Ericsson and LG APIs | 76 | 767 |
| [system-properties.md](system-properties.md) | Every system property key games read, with values | | |
| [jsr-later.md](jsr-later.md) | For later versions: M3G (JSR-184), MascotCapsule V3, FileConnection and PIM (JSR-75), WMA (JSR-120), Bluetooth (JSR-82), Location (JSR-179), SVG (JSR-226) | 121 | 1358 |

## How a file is laid out

Each file starts with what the package is for and the rules from its
spec that an implementer must know, then:

1. **Sources**: every document and code base the file was built from.
2. **Classes**: an index with each class's kind, tier, the version that
   added it, and its member count.
3. **One section per class**: the declaration, its tier and version, a
   link to its javadoc page and to its source in each open emulator that
   has it, a note on behaviour where it matters, and the member tables.
4. **Cross-check**: every place where the stub jars or the emulators'
   source disagree with the javadoc (a missing member, a different
   constant value, a different `static` flag). An emulator that disagrees
   is usually wrong; the javadoc is the spec.

Members are listed under the class that declares them. An inherited
member is not repeated: look it up in the superclass, as the JVM does
when it resolves a reference.

### Columns

| Column | Meaning |
|---|---|
| T | The tier, see below. |
| Field | The field name. |
| Type | The field's JVM descriptor, such as `I` or `Ljava/lang/String;`. |
| Value | The value of a compile-time constant. Integers in decimal, with the hex form for bit flags and masks; `float` and `double` with their exact bits; `char` as the character and its code; `String` in Java syntax. "(not a constant)" for fields that are not compile-time constants, such as `System.out`. |
| Constructor, Method | The Java form, with the javadoc's parameter names and simple type names. |
| Descriptor | The member's name and JVM descriptor exactly as a `CONSTANT_NameAndType` in a class file names it: `drawString(Ljava/lang/String;III)V`, `<init>(Ljava/lang/String;)V`. A game's `invokevirtual` or `getstatic` must resolve to this. |
| Flags | `protected` (every member not marked so is `public`), `static`, `abstract` (not repeated on interface methods, which are all abstract), `final`, and `deprecated` when the javadoc deprecates the member. Fields are `final` unless the column says "not final". |
| Since | The version that added the member. `CLDC 1.1` marks what CLDC 1.0 lacks (found by comparing the CLDC 1.0 and 1.1 javadoc and stub jars); `MIDP 2.0` marks what MIDP 1.0 lacks (from the javadoc's since tags); in [midp-media.md](midp-media.md), `MIDP 2.0` marks the audio subset every MIDP 2.0 phone has and `MMAPI` what only a full JSR-135 phone has; vendor files show the vendor's own version tags. |
| Throws | Each exception the javadoc lists under "Throws", with its condition in the javadoc's words, then any declared exception the javadoc does not describe. Unchecked exceptions are included: FastEmu must throw them in the same cases, because games catch them. |
| Notes | What an implementer needs beyond the signature: behaviour games depend on, traps, and how FastEmu should map the member onto its runtime. |

## Tiers

Every class and member has a tier:

- **Tier 1: most games need it.** A game cannot start or play without it:
  `Canvas`, `Graphics`, `Image`, `Font`, `Display`, `GameCanvas`,
  `Sprite`, MMAPI's `Manager` and `Player`, `RecordStore`, `Thread`,
  `StringBuffer.append`, Nokia's `FullCanvas` and `DirectGraphics`, and
  every exception the VM itself throws (`NullPointerException`,
  `ArrayIndexOutOfBoundsException` and the rest). Implement all of tier
  1 before anything else.
- **Tier 2: many games need it.** Common but not universal: high-level
  screens such as `Form`, `List` and `Alert`, `PlayerListener` and
  `ToneControl`, `HttpConnection`, `Calendar`, Nokia's `DeviceControl`,
  Siemens' game API.
- **Tier 3: rare.** Few games call it, or only for side features:
  `CustomItem`, `PushRegistry`, record store sharing, video, most vendor
  APIs. Implement it last, but never leave a game-reachable member
  unimplemented: return a clear error instead.

A member takes its class's tier unless the file gives it another. No
vendor API is tier 1, since no vendor API is needed by most games; a game
written for one vendor still needs that vendor's classes to start.

Tiers are a judgment, not a measurement: they come from what the spec
says games may rely on, from which members the open emulators implement
fully (they were driven by real games), and from how J2ME games are
commonly written. They set an order of work; which members each FastEmu
version implements is the project's decision.

## Sources and what was checked

- **Spec javadoc.** CLDC 1.0, CLDC 1.1, MIDP 2.0, JSR-135 and JSR-82 from
  Oracle's hosted copies; Nokia, Siemens, Samsung, Motorola, Sony
  Ericsson, LG, MascotCapsule and the other JSRs from
  [J2ME_Docs](https://github.com/nikita36078/J2ME_Docs), a javadoc
  collection kept by J2ME Loader's author. Class pages were parsed for
  declarations, parameter names, throws entries and since tags;
  `constant-values.html` pages for constant values.
- **Stub jars.** MicroEmulator 2.0.4's API jars on Maven Central
  (`cldcapi10`, `cldcapi11`, `midpapi20`, and the JSR-75, JSR-82, JSR-120,
  JSR-135, Nokia UI and Siemens jars), whose class files give exact
  descriptors, flags and `ConstantValue` attributes. The CLDC 1.1 and
  MIDP 2.0 javadoc and stubs agree on every constant and on every member
  except `WeakReference.get` and `clear`, which the javadoc documents as
  overrides and the stub inherits from `Reference`.
- **Emulator source.** [J2ME Loader](https://github.com/nikita36078/J2ME-Loader)
  (Apache-2.0), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus)
  (GPL-3.0) and [MicroEmulator](https://github.com/barteo/microemu)
  (LGPL-2.1 or Apache-2.0), parsed for member lists and constant
  initializers.

Known limits:

- Oracle's `java/lang/StringBuffer.html` is cut off in all three Oracle
  sets, so `StringBuffer` comes from the CLDC 1.1.1 edition of the
  javadoc; its members match the CLDC 1.1 stub exactly.
- Nokia UI API 1.1's `constant-values.html` is empty, so the Nokia
  constant values come from the MicroEmulator Nokia jar and the three
  emulators, which agree.
- LG's javadoc is in Korean; its throws entries keep the exception names
  but not the conditions.
- The javadoc of a few late APIs (JTWI, SATSA, JAX-RPC, JAXP) is not in
  the collection; their system property keys come from BlackBerry's list
  in [system-properties.md](system-properties.md).

The Throws column and some notes quote the specification documents
briefly, so the conditions are exact.
