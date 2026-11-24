# java.lang and java.lang.ref (CLDC 1.1)

The `java.lang` and `java.lang.ref` classes a CLDC 1.1 virtual machine
provides, plus `IllegalStateException`, which MIDP adds. CLDC is a subset
of Java SE 1.3: the classes keep their Java SE names and descriptors, but
many members are missing. A game compiled against a member that is not
listed here fails to link on a real phone, so FastEmu should not add
members beyond this list either.

CLDC 1.0 has no floating point at all: no `float` or `double` in the VM,
no `Float` or `Double` classes, and no members that take or return them.
The Since column says `CLDC 1.1` for each member CLDC 1.0 lacks. Games
written for CLDC 1.0 phones use fixed-point math instead, and still run on
a CLDC 1.1 VM.

Exceptions the VM throws on its own (null dereference, bad array index,
division by zero, failed cast, failed allocation, missing class) are tier
1 whatever the game code says, because the interpreter must create them.

Column meanings and tier rules: [README](README.md).

## Sources

- [CLDC 1.1 javadoc (JSR-139)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- [CLDC 1.0 javadoc (JSR-30)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.0/jsr030/index.html): compared member by member to mark what CLDC 1.0 lacks (Since column).
- MicroEmulator 2.0.4 API stub jars [cldcapi10](https://repo1.maven.org/maven2/org/microemu/cldcapi10/2.0.4/cldcapi10-2.0.4.jar) and [cldcapi11](https://repo1.maven.org/maven2/org/microemu/cldcapi11/2.0.4/cldcapi11-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [MIDP 2.0 javadoc](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/lang/IllegalStateException.html): `IllegalStateException`, which MIDP adds to `java.lang`.
- Oracle's copy of `StringBuffer.html` stops in the middle of the `setLength` entry (the page holds a NUL character), in all three Oracle sets. `StringBuffer` is therefore taken from the [J2ME_Docs copy](https://nikita36078.github.io/J2ME_Docs/docs/cldc-1.1/java/lang/StringBuffer.html) of the JSR-139 javadoc (commit `fbfdfed7009369b6bf7a5cce39b8a13f178c2c69`), which is the CLDC 1.1.1 edition; its `StringBuffer` members match the cldcapi11 stub exactly (see Cross-check).

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [ArithmeticException](#arithmeticexception) | class | 1 | CLDC 1.0 | 2 |
| [ArrayIndexOutOfBoundsException](#arrayindexoutofboundsexception) | class | 1 | CLDC 1.0 | 3 |
| [ArrayStoreException](#arraystoreexception) | class | 2 | CLDC 1.0 | 2 |
| [Boolean](#boolean) | class | 2 | CLDC 1.0 | 7 |
| [Byte](#byte) | class | 3 | CLDC 1.0 | 9 |
| [Character](#character) | class | 2 | CLDC 1.0 | 15 |
| [Class](#class) | class | 1 | CLDC 1.0 | 9 |
| [ClassCastException](#classcastexception) | class | 1 | CLDC 1.0 | 2 |
| [ClassNotFoundException](#classnotfoundexception) | class | 1 | CLDC 1.0 | 2 |
| [Double](#double) | class | 2 | CLDC 1.1 | 24 |
| [Error](#error) | class | 2 | CLDC 1.0 | 2 |
| [Exception](#exception) | class | 1 | CLDC 1.0 | 2 |
| [Float](#float) | class | 2 | CLDC 1.1 | 25 |
| [IllegalAccessException](#illegalaccessexception) | class | 2 | CLDC 1.0 | 2 |
| [IllegalArgumentException](#illegalargumentexception) | class | 1 | CLDC 1.0 | 2 |
| [IllegalMonitorStateException](#illegalmonitorstateexception) | class | 2 | CLDC 1.0 | 2 |
| [IllegalThreadStateException](#illegalthreadstateexception) | class | 3 | CLDC 1.0 | 2 |
| [IndexOutOfBoundsException](#indexoutofboundsexception) | class | 2 | CLDC 1.0 | 2 |
| [InstantiationException](#instantiationexception) | class | 2 | CLDC 1.0 | 2 |
| [Integer](#integer) | class | 1 | CLDC 1.0 | 21 |
| [InterruptedException](#interruptedexception) | class | 1 | CLDC 1.0 | 2 |
| [Long](#long) | class | 2 | CLDC 1.0 | 13 |
| [Math](#math) | class | 1 | CLDC 1.0 | 22 |
| [NegativeArraySizeException](#negativearraysizeexception) | class | 1 | CLDC 1.0 | 2 |
| [NoClassDefFoundError](#noclassdeffounderror) | class | 1 | CLDC 1.1 | 2 |
| [NullPointerException](#nullpointerexception) | class | 1 | CLDC 1.0 | 2 |
| [NumberFormatException](#numberformatexception) | class | 1 | CLDC 1.0 | 2 |
| [Object](#object) | class | 1 | CLDC 1.0 | 10 |
| [OutOfMemoryError](#outofmemoryerror) | class | 1 | CLDC 1.0 | 2 |
| [Runnable](#runnable) | interface | 1 | CLDC 1.0 | 1 |
| [Runtime](#runtime) | class | 1 | CLDC 1.0 | 5 |
| [RuntimeException](#runtimeexception) | class | 1 | CLDC 1.0 | 2 |
| [SecurityException](#securityexception) | class | 2 | CLDC 1.0 | 2 |
| [Short](#short) | class | 3 | CLDC 1.0 | 9 |
| [String](#string) | class | 1 | CLDC 1.0 | 47 |
| [StringBuffer](#stringbuffer) | class | 1 | CLDC 1.0 | 33 |
| [StringIndexOutOfBoundsException](#stringindexoutofboundsexception) | class | 1 | CLDC 1.0 | 3 |
| [System](#system) | class | 1 | CLDC 1.0 | 8 |
| [Thread](#thread) | class | 1 | CLDC 1.0 | 20 |
| [Throwable](#throwable) | class | 1 | CLDC 1.0 | 5 |
| [VirtualMachineError](#virtualmachineerror) | class | 2 | CLDC 1.0 | 2 |
| [IllegalStateException](#illegalstateexception) | class | 1 | MIDP 1.0 (not CLDC) | 2 |
| [Reference](#reference) | class | 3 | CLDC 1.1 | 2 |
| [WeakReference](#weakreference) | class | 3 | CLDC 1.1 | 3 |

## java.lang

### ArithmeticException

`public class java.lang.ArithmeticException extends java.lang.RuntimeException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ArithmeticException.html).

Thrown by the VM for integer division or remainder by zero (`idiv`, `irem`, `ldiv`, `lrem`).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### ArrayIndexOutOfBoundsException

`public class java.lang.ArrayIndexOutOfBoundsException extends java.lang.IndexOutOfBoundsException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ArrayIndexOutOfBoundsException.html).

Thrown by the VM from every array load and store with a bad index.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(int index)` | `<init>(I)V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### ArrayStoreException

`public class java.lang.ArrayStoreException extends java.lang.RuntimeException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ArrayStoreException.html).

Thrown by the VM from `aastore` when the value's class does not fit the array's component type.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Boolean

`public final class java.lang.Boolean extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Boolean.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `TRUE` | `Ljava/lang/Boolean;` | (not a constant) | static | 1.1 |  |
| 2 | `FALSE` | `Ljava/lang/Boolean;` | (not a constant) | static | 1.1 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(boolean value)` | `<init>(Z)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `boolean booleanValue()` | `booleanValue()Z` |  | 1.0 |  |  |
| 2 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 2 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |

### Byte

`public final class java.lang.Byte extends java.lang.Object`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Byte.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `MIN_VALUE` | `B` | `-128` | static | 1.0 |  |
| 3 | `MAX_VALUE` | `B` | `127` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(byte value)` | `<init>(B)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `byte parseByte(String s)` | `parseByte(Ljava/lang/String;)B` | static | 1.0 | `NumberFormatException` If the string does not contain a parsable byte |  |
| 3 | `byte parseByte(String s, int radix)` | `parseByte(Ljava/lang/String;I)B` | static | 1.0 | `NumberFormatException` If the String does not contain a parsable byte |  |
| 3 | `byte byteValue()` | `byteValue()B` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |

### Character

`public final class java.lang.Character extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Character.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `MIN_RADIX` | `I` | `2` | static | 1.0 |  |
| 3 | `MAX_RADIX` | `I` | `36` | static | 1.0 |  |
| 3 | `MIN_VALUE` | `C` | `'\u0000'` (0) | static | 1.0 |  |
| 3 | `MAX_VALUE` | `C` | `'\uffff'` (65535) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(char value)` | `<init>(C)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `char charValue()` | `charValue()C` |  | 1.0 |  |  |
| 2 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 2 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 2 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `boolean isLowerCase(char ch)` | `isLowerCase(C)Z` | static | 1.0 |  |  |
| 2 | `boolean isUpperCase(char ch)` | `isUpperCase(C)Z` | static | 1.0 |  |  |
| 2 | `boolean isDigit(char ch)` | `isDigit(C)Z` | static | 1.0 |  |  |
| 2 | `char toLowerCase(char ch)` | `toLowerCase(C)C` | static | 1.0 |  |  |
| 2 | `char toUpperCase(char ch)` | `toUpperCase(C)C` | static | 1.0 |  |  |
| 2 | `int digit(char ch, int radix)` | `digit(CI)I` | static | 1.0 |  |  |

### Class

`public final class java.lang.Class extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Class.html).

Games probe for vendor APIs with `Class.forName` and catch `ClassNotFoundException`, then pick a code path (Nokia `DirectUtils`, Siemens `ExtendedImage`, MMAPI `Manager`). Which vendor classes FastEmu offers changes what a game does.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 1 | `Class forName(String className)` | `forName(Ljava/lang/String;)Ljava/lang/Class;` | static | 1.0 | `ClassNotFoundException` if the class could not be found; `Error` if the function fails for any other reason | Loads and links the class. Throws `ClassNotFoundException` for names the suite and the platform do not have; games rely on that for feature detection. |
| 2 | `Object newInstance()` | `newInstance()Ljava/lang/Object;` |  | 1.0 | `IllegalAccessException` if the class or initializer is not accessible; `InstantiationException` if an application tries to instantiate an abstract class or an interface, or if the instantiation fails for some other reason | Calls the public no-argument constructor. Used by games that load levels or screens by class name. |
| 3 | `boolean isInstance(Object obj)` | `isInstance(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 3 | `boolean isAssignableFrom(Class cls)` | `isAssignableFrom(Ljava/lang/Class;)Z` |  | 1.0 | `NullPointerException` if the specified Class parameter is null |  |
| 3 | `boolean isInterface()` | `isInterface()Z` |  | 1.0 |  |  |
| 3 | `boolean isArray()` | `isArray()Z` |  | 1.0 |  |  |
| 2 | `String getName()` | `getName()Ljava/lang/String;` |  | 1.0 |  | Returns the binary name with dots, `[I` style names for arrays. |
| 1 | `InputStream getResourceAsStream(String name)` | `getResourceAsStream(Ljava/lang/String;)Ljava/io/InputStream;` |  | 1.0 |  | Reads a file from the MIDlet suite JAR; returns `null` when it is missing. A name starting with `/` is absolute; any other name is relative to the package of this class. MIDP adds: canonicalize `/./` and `/segment/../`, never reach outside the JAR, and never return `.class` files (MIDP 2.0 `java.lang` package notes). |

### ClassCastException

`public class java.lang.ClassCastException extends java.lang.RuntimeException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ClassCastException.html).

Thrown by the VM from `checkcast`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### ClassNotFoundException

`public class java.lang.ClassNotFoundException extends java.lang.Exception`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ClassNotFoundException.html).

Thrown by `Class.forName`; games catch it to detect vendor APIs.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Double

`public final class java.lang.Double extends java.lang.Object`

Tier 2. Since CLDC 1.1. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Double.html).

CLDC 1.1 only. Games for CLDC 1.0 phones never touch it.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `POSITIVE_INFINITY` | `D` | `+Infinity` (bits `0x7ff0000000000000`) | static | 1.1 |  |
| 3 | `NEGATIVE_INFINITY` | `D` | `-Infinity` (bits `0xfff0000000000000`) | static | 1.1 |  |
| 2 | `NaN` | `D` | `NaN` (bits `0x7ff8000000000000`) | static | 1.1 | NaN constants compare unequal to themselves; the `ConstantValue` holds the canonical bits. |
| 3 | `MAX_VALUE` | `D` | `1.7976931348623157E308` (bits `0x7fefffffffffffff`) | static | 1.1 |  |
| 3 | `MIN_VALUE` | `D` | `4.9E-324` (bits `0x0000000000000001`) | static | 1.1 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(double value)` | `<init>(D)V` |  | 1.1 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String toString(double d)` | `toString(D)Ljava/lang/String;` | static | 1.1 |  |  |
| 2 | `Double valueOf(String s)` | `valueOf(Ljava/lang/String;)Ljava/lang/Double;` | static | 1.1 | `NumberFormatException` if the string does not contain a parsable number |  |
| 2 | `double parseDouble(String s)` | `parseDouble(Ljava/lang/String;)D` | static | 1.1 | `NumberFormatException` if the string does not contain a parsable double |  |
| 2 | `boolean isNaN(double v)` | `isNaN(D)Z` | static | 1.1 |  |  |
| 2 | `boolean isInfinite(double v)` | `isInfinite(D)Z` | static | 1.1 |  |  |
| 2 | `boolean isNaN()` | `isNaN()Z` |  | 1.1 |  |  |
| 2 | `boolean isInfinite()` | `isInfinite()Z` |  | 1.1 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.1 |  |  |
| 3 | `byte byteValue()` | `byteValue()B` |  | 1.1 |  |  |
| 3 | `short shortValue()` | `shortValue()S` |  | 1.1 |  |  |
| 2 | `int intValue()` | `intValue()I` |  | 1.1 |  |  |
| 3 | `long longValue()` | `longValue()J` |  | 1.1 |  |  |
| 3 | `float floatValue()` | `floatValue()F` |  | 1.1 |  |  |
| 2 | `double doubleValue()` | `doubleValue()D` |  | 1.1 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | 1.1 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.1 |  |  |
| 3 | `long doubleToLongBits(double value)` | `doubleToLongBits(D)J` | static | 1.1 |  |  |
| 3 | `double longBitsToDouble(long bits)` | `longBitsToDouble(J)D` | static | 1.1 |  |  |

### Error

`public class java.lang.Error extends java.lang.Throwable`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Error.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Exception

`public class java.lang.Exception extends java.lang.Throwable`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Exception.html).

The common base games catch around everything (`catch (Exception e)`).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Float

`public final class java.lang.Float extends java.lang.Object`

Tier 2. Since CLDC 1.1. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Float.html).

CLDC 1.1 only.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `POSITIVE_INFINITY` | `F` | `+Infinity` (bits `0x7f800000`) | static | 1.1 |  |
| 3 | `NEGATIVE_INFINITY` | `F` | `-Infinity` (bits `0xff800000`) | static | 1.1 |  |
| 3 | `NaN` | `F` | `NaN` (bits `0x7fc00000`) | static | 1.1 |  |
| 3 | `MAX_VALUE` | `F` | `3.4028234663852886E38f` (bits `0x7f7fffff`) | static | 1.1 |  |
| 3 | `MIN_VALUE` | `F` | `1.401298464324817E-45f` (bits `0x00000001`) | static | 1.1 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(float value)` | `<init>(F)V` |  | 1.1 |  |  |
| 3 | `(double value)` | `<init>(D)V` |  | 1.1 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String toString(float f)` | `toString(F)Ljava/lang/String;` | static | 1.1 |  |  |
| 2 | `Float valueOf(String s)` | `valueOf(Ljava/lang/String;)Ljava/lang/Float;` | static | 1.1 | `NumberFormatException` if the string does not contain a parsable number |  |
| 2 | `float parseFloat(String s)` | `parseFloat(Ljava/lang/String;)F` | static | 1.1 | `NumberFormatException` if the string does not contain a parsable float |  |
| 2 | `boolean isNaN(float v)` | `isNaN(F)Z` | static | 1.1 |  |  |
| 2 | `boolean isInfinite(float v)` | `isInfinite(F)Z` | static | 1.1 |  |  |
| 2 | `boolean isNaN()` | `isNaN()Z` |  | 1.1 |  |  |
| 2 | `boolean isInfinite()` | `isInfinite()Z` |  | 1.1 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.1 |  |  |
| 3 | `byte byteValue()` | `byteValue()B` |  | 1.1 |  |  |
| 3 | `short shortValue()` | `shortValue()S` |  | 1.1 |  |  |
| 2 | `int intValue()` | `intValue()I` |  | 1.1 |  |  |
| 3 | `long longValue()` | `longValue()J` |  | 1.1 |  |  |
| 2 | `float floatValue()` | `floatValue()F` |  | 1.1 |  |  |
| 3 | `double doubleValue()` | `doubleValue()D` |  | 1.1 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | 1.1 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.1 |  |  |
| 3 | `int floatToIntBits(float value)` | `floatToIntBits(F)I` | static | 1.1 |  |  |
| 3 | `float intBitsToFloat(int bits)` | `intBitsToFloat(I)F` | static | 1.1 |  |  |

### IllegalAccessException

`public class java.lang.IllegalAccessException extends java.lang.Exception`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/IllegalAccessException.html).

Thrown by `Class.newInstance` when the constructor is not public.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IllegalArgumentException

`public class java.lang.IllegalArgumentException extends java.lang.RuntimeException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/IllegalArgumentException.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IllegalMonitorStateException

`public class java.lang.IllegalMonitorStateException extends java.lang.RuntimeException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/IllegalMonitorStateException.html).

Thrown by `wait`, `notify` and `notifyAll` when the caller does not own the monitor, and by `monitorexit` without a matching enter.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IllegalThreadStateException

`public class java.lang.IllegalThreadStateException extends java.lang.IllegalArgumentException`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/IllegalThreadStateException.html).

Thrown by `Thread.start` on a thread that was already started.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  | 1.0 |  |  |
| 3 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IndexOutOfBoundsException

`public class java.lang.IndexOutOfBoundsException extends java.lang.RuntimeException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/IndexOutOfBoundsException.html).

Base of the array and string index exceptions; thrown directly by `Vector` and stream methods.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### InstantiationException

`public class java.lang.InstantiationException extends java.lang.Exception`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/InstantiationException.html).

Thrown by `Class.newInstance` for interfaces and abstract classes.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Integer

`public final class java.lang.Integer extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Integer.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `MIN_VALUE` | `I` | `-2147483648` (`0x80000000`) | static | 1.0 |  |
| 2 | `MAX_VALUE` | `I` | `2147483647` (`0x7fffffff`) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(int value)` | `<init>(I)V` |  | 1.0 |  | Boxed keys and values for `Hashtable` and `Vector`. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String toString(int i, int radix)` | `toString(II)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String toHexString(int i)` | `toHexString(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 3 | `String toOctalString(int i)` | `toOctalString(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 3 | `String toBinaryString(int i)` | `toBinaryString(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 1 | `String toString(int i)` | `toString(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 1 | `int parseInt(String s, int radix)` | `parseInt(Ljava/lang/String;I)I` | static | 1.0 | `NumberFormatException` if the string does not contain a parsable integer | Throws `NumberFormatException` for a null, empty or malformed string. Games parse JAD properties and level files with it. |
| 1 | `int parseInt(String s)` | `parseInt(Ljava/lang/String;)I` | static | 1.0 | `NumberFormatException` if the string does not contain a parsable integer | Throws `NumberFormatException` for a null, empty or malformed string. Games parse JAD properties and level files with it. |
| 2 | `Integer valueOf(String s, int radix)` | `valueOf(Ljava/lang/String;I)Ljava/lang/Integer;` | static | 1.0 | `NumberFormatException` if the String cannot be parsed as an int |  |
| 2 | `Integer valueOf(String s)` | `valueOf(Ljava/lang/String;)Ljava/lang/Integer;` | static | 1.0 | `NumberFormatException` if the string cannot be parsed as an integer |  |
| 3 | `byte byteValue()` | `byteValue()B` |  | 1.0 |  |  |
| 3 | `short shortValue()` | `shortValue()S` |  | 1.0 |  |  |
| 1 | `int intValue()` | `intValue()I` |  | 1.0 |  |  |
| 2 | `long longValue()` | `longValue()J` |  | 1.0 |  |  |
| 3 | `float floatValue()` | `floatValue()F` |  | 1.1 |  |  |
| 3 | `double doubleValue()` | `doubleValue()D` |  | 1.1 |  |  |
| 2 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 1 | `int hashCode()` | `hashCode()I` |  | 1.0 |  | Returns the value itself. |
| 1 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  | Two `Integer`s are equal when their values are; `Hashtable` lookups depend on it. |

### InterruptedException

`public class java.lang.InterruptedException extends java.lang.Exception`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/InterruptedException.html).

Declared by `Thread.sleep`, `Thread.join` and `Object.wait`; nearly every game loop catches it.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Long

`public final class java.lang.Long extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Long.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `MIN_VALUE` | `J` | `-9223372036854775808L` (`0x8000000000000000L`) | static | 1.0 |  |
| 2 | `MAX_VALUE` | `J` | `9223372036854775807L` (`0x7fffffffffffffffL`) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(long value)` | `<init>(J)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String toString(long i, int radix)` | `toString(JI)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String toString(long i)` | `toString(J)Ljava/lang/String;` | static | 1.0 |  |  |
| 3 | `long parseLong(String s, int radix)` | `parseLong(Ljava/lang/String;I)J` | static | 1.0 | `NumberFormatException` if the string does not contain a parsable integer |  |
| 2 | `long parseLong(String s)` | `parseLong(Ljava/lang/String;)J` | static | 1.0 | `NumberFormatException` if the string does not contain a parsable long |  |
| 2 | `long longValue()` | `longValue()J` |  | 1.0 |  |  |
| 3 | `float floatValue()` | `floatValue()F` |  | 1.1 |  |  |
| 3 | `double doubleValue()` | `doubleValue()D` |  | 1.1 |  |  |
| 2 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 2 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |

### Math

`public final class java.lang.Math extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Math.html).

CLDC 1.1 has no `asin`, `acos`, `atan`, `atan2`, `exp`, `log`, `pow`, `round` or `random`; CLDC 1.1.1 later added `asin`, `acos`, `atan` and `atan2` only. Games that need them ship their own, usually as fixed-point tables.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `E` | `D` | `2.718281828459045` (bits `0x4005bf0a8b145769`) | static | 1.1 |  |
| 2 | `PI` | `D` | `3.141592653589793` (bits `0x400921fb54442d18`) | static | 1.1 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `double sin(double a)` | `sin(D)D` | static | 1.1 |  | Argument in radians. Results must match Java SE's `StrictMath` closely enough that games that build lookup tables at start-up get the same tables. |
| 2 | `double cos(double a)` | `cos(D)D` | static | 1.1 |  |  |
| 3 | `double tan(double a)` | `tan(D)D` | static | 1.1 |  |  |
| 3 | `double toRadians(double angdeg)` | `toRadians(D)D` | static | 1.1 |  |  |
| 3 | `double toDegrees(double angrad)` | `toDegrees(D)D` | static | 1.1 |  |  |
| 2 | `double sqrt(double a)` | `sqrt(D)D` | static | 1.1 |  |  |
| 2 | `double ceil(double a)` | `ceil(D)D` | static | 1.1 |  |  |
| 2 | `double floor(double a)` | `floor(D)D` | static | 1.1 |  |  |
| 1 | `int abs(int a)` | `abs(I)I` | static | 1.0 |  |  |
| 2 | `long abs(long a)` | `abs(J)J` | static | 1.0 |  |  |
| 2 | `float abs(float a)` | `abs(F)F` | static | 1.1 |  |  |
| 2 | `double abs(double a)` | `abs(D)D` | static | 1.1 |  |  |
| 1 | `int max(int a, int b)` | `max(II)I` | static | 1.0 |  |  |
| 2 | `long max(long a, long b)` | `max(JJ)J` | static | 1.0 |  |  |
| 2 | `float max(float a, float b)` | `max(FF)F` | static | 1.1 |  |  |
| 2 | `double max(double a, double b)` | `max(DD)D` | static | 1.1 |  |  |
| 1 | `int min(int a, int b)` | `min(II)I` | static | 1.0 |  |  |
| 2 | `long min(long a, long b)` | `min(JJ)J` | static | 1.0 |  |  |
| 2 | `float min(float a, float b)` | `min(FF)F` | static | 1.1 |  |  |
| 2 | `double min(double a, double b)` | `min(DD)D` | static | 1.1 |  |  |

### NegativeArraySizeException

`public class java.lang.NegativeArraySizeException extends java.lang.RuntimeException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/NegativeArraySizeException.html).

Thrown by the VM from `newarray`, `anewarray` and `multianewarray` with a negative size.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### NoClassDefFoundError

`public class java.lang.NoClassDefFoundError extends java.lang.Error`

Tier 1. Since CLDC 1.1. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/NoClassDefFoundError.html).

CLDC 1.1 only. Thrown by the VM when a class a game references (often a vendor API) is missing at link time. Games that probe with `Class.forName` avoid it; games that reference the class directly crash with it.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.1 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.1 |  |  |

### NullPointerException

`public class java.lang.NullPointerException extends java.lang.RuntimeException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/NullPointerException.html).

Thrown by the VM for every use of a null reference.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### NumberFormatException

`public class java.lang.NumberFormatException extends java.lang.IllegalArgumentException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/NumberFormatException.html).

Thrown by the `parse` methods; games catch it when reading JAD and saved data.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Object

`public class java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Object.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Class getClass()` | `getClass()Ljava/lang/Class;` | final | 1.0 |  |  |
| 1 | `int hashCode()` | `hashCode()I` |  | 1.0 |  | Identity hash by default. Games use objects as `Hashtable` keys. |
| 1 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 1 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `void notify()` | `notify()V` | final | 1.0 | `IllegalMonitorStateException` if the current thread is not the owner of this object's monitor |  |
| 2 | `void notifyAll()` | `notifyAll()V` | final | 1.0 | `IllegalMonitorStateException` if the current thread is not the owner of this object's monitor |  |
| 2 | `void wait(long timeout)` | `wait(J)V` | final | 1.0 | `IllegalArgumentException` if the value of timeout is negative; `IllegalMonitorStateException` if the current thread is not the owner of the object's monitor; `InterruptedException` if another thread has interrupted the current thread. The interrupted status of the current thread is cleared when this exception is thrown | Waits at most the given milliseconds; 0 means forever. The caller must own the monitor. |
| 3 | `void wait(long timeout, int nanos)` | `wait(JI)V` | final | 1.0 | `IllegalArgumentException` if the value of timeout is negative or the value of nanos is not in the range 0-999999; `IllegalMonitorStateException` if the current thread is not the owner of this object's monitor; `InterruptedException` if another thread has interrupted the current thread. The interrupted status of the current thread is cleared when this exception is thrown |  |
| 2 | `void wait()` | `wait()V` | final | 1.0 | `IllegalMonitorStateException` if the current thread is not the owner of the object's monitor; `InterruptedException` if another thread has interrupted the current thread. The interrupted status of the current thread is cleared when this exception is thrown |  |

### OutOfMemoryError

`public class java.lang.OutOfMemoryError extends java.lang.VirtualMachineError`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/OutOfMemoryError.html).

Thrown by the VM when the heap is exhausted. Some games catch it and drop caches; the heap size a profile reports (`Runtime.totalMemory`) should match the size FastEmu really allows.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Runnable

`public interface java.lang.Runnable`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Runnable.html).

Implemented by nearly every game's main class or canvas for its game loop thread.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void run()` | `run()V` |  | 1.0 |  |  |

### Runtime

`public class java.lang.Runtime extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Runtime.html).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `Runtime getRuntime()` | `getRuntime()Ljava/lang/Runtime;` | static | 1.0 |  |  |
| 3 | `void exit(int status)` | `exit(I)V` |  | 1.0 |  | A MIDlet must call `MIDlet.notifyDestroyed` instead: MIDP says `Runtime.exit` MUST throw `SecurityException` when a MIDlet calls it. |
| 1 | `long freeMemory()` | `freeMemory()J` |  | 1.0 |  | Games check it to pick asset quality or to decide when to drop images. Report numbers consistent with the profile's heap. |
| 1 | `long totalMemory()` | `totalMemory()J` |  | 1.0 |  | The heap size of the phone; FastEmu reports the device profile's `heap_bytes`. |
| 2 | `void gc()` | `gc()V` |  | 1.0 |  |  |

### RuntimeException

`public class java.lang.RuntimeException extends java.lang.Exception`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/RuntimeException.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### SecurityException

`public class java.lang.SecurityException extends java.lang.RuntimeException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/SecurityException.html).

Thrown by `System.exit` and `Runtime.exit` in a MIDlet, and by permission checks (HTTP, push, files).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Short

`public final class java.lang.Short extends java.lang.Object`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Short.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `MIN_VALUE` | `S` | `-32768` (`0xffff8000`) | static | 1.0 |  |
| 3 | `MAX_VALUE` | `S` | `32767` (`0x7fff`) | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(short value)` | `<init>(S)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `short parseShort(String s)` | `parseShort(Ljava/lang/String;)S` | static | 1.0 | `NumberFormatException` If the string does not contain a parsable short |  |
| 3 | `short parseShort(String s, int radix)` | `parseShort(Ljava/lang/String;I)S` | static | 1.0 | `NumberFormatException` If the String does not contain a parsable short |  |
| 3 | `short shortValue()` | `shortValue()S` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |

### String

`public final class java.lang.String extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/String.html).

`String` constants in class files are interned by the VM; `==` between two literals with the same text is true and games rely on it.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String value)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `(char[] value)` | `<init>([C)V` |  | 1.0 | `NullPointerException` if value is null |  |
| 2 | `(char[] value, int offset, int count)` | `<init>([CII)V` |  | 1.0 | `IndexOutOfBoundsException` if the offset and count arguments index characters outside the bounds of the value array; `NullPointerException` if value is null |  |
| 2 | `(byte[] bytes, int off, int len, String enc)` | `<init>([BIILjava/lang/String;)V` |  | 1.0 | `UnsupportedEncodingException` If the named encoding is not supported | Decodes with the named encoding; throws `UnsupportedEncodingException` for names the phone does not know. Games mostly pass `"UTF-8"`; FastEmu should know at least `UTF-8`, `ISO-8859-1` and `US-ASCII` and the value of `microedition.encoding`. |
| 2 | `(byte[] bytes, String enc)` | `<init>([BLjava/lang/String;)V` |  | 1.0 | `UnsupportedEncodingException` If the named encoding is not supported | Decodes with the named encoding. Games read UTF-8 text files with it. |
| 2 | `(byte[] bytes, int off, int len)` | `<init>([BII)V` |  | 1.0 |  | Decodes with the default encoding (`microedition.encoding`). |
| 2 | `(byte[] bytes)` | `<init>([B)V` |  | 1.0 |  | Decodes with the default encoding (`microedition.encoding`). |
| 3 | `(StringBuffer buffer)` | `<init>(Ljava/lang/StringBuffer;)V` |  | 1.0 | `NullPointerException` If buffer is null |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int length()` | `length()I` |  | 1.0 |  |  |
| 1 | `char charAt(int index)` | `charAt(I)C` |  | 1.0 | `IndexOutOfBoundsException` if the index argument is negative or not less than the length of this string |  |
| 2 | `void getChars(int srcBegin, int srcEnd, char[] dst, int dstBegin)` | `getChars(II[CI)V` |  | 1.0 | `IndexOutOfBoundsException` If any of the following is true: srcBegin is negative. srcBegin is greater than srcEnd srcEnd is greater than the length of this string dstBegin is negative dstBegin+(srcEnd-srcBegin) is larger than dst.length; `NullPointerException` if dst is null |  |
| 2 | `byte[] getBytes(String enc)` | `getBytes(Ljava/lang/String;)[B` |  | 1.0 | `UnsupportedEncodingException` If the named encoding is not supported |  |
| 2 | `byte[] getBytes()` | `getBytes()[B` |  | 1.0 |  | Encodes with the default encoding (`microedition.encoding`). |
| 1 | `boolean equals(Object anObject)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 2 | `boolean equalsIgnoreCase(String anotherString)` | `equalsIgnoreCase(Ljava/lang/String;)Z` |  | 1.1 |  |  |
| 2 | `int compareTo(String anotherString)` | `compareTo(Ljava/lang/String;)I` |  | 1.0 | `NullPointerException` if anotherString is null |  |
| 3 | `boolean regionMatches(boolean ignoreCase, int toffset, String other, int ooffset, int len)` | `regionMatches(ZILjava/lang/String;II)Z` |  | 1.0 |  |  |
| 2 | `boolean startsWith(String prefix, int toffset)` | `startsWith(Ljava/lang/String;I)Z` |  | 1.0 | `NullPointerException` if prefix is null |  |
| 1 | `boolean startsWith(String prefix)` | `startsWith(Ljava/lang/String;)Z` |  | 1.0 | `NullPointerException` if prefix is null |  |
| 2 | `boolean endsWith(String suffix)` | `endsWith(Ljava/lang/String;)Z` |  | 1.0 | `NullPointerException` if suffix is null |  |
| 1 | `int hashCode()` | `hashCode()I` |  | 1.0 |  | `s[0]*31^(n-1) + ... + s[n-1]` with `int` overflow, as in Java SE. Games hash with it and save the results, so the formula must be exact. |
| 1 | `int indexOf(int ch)` | `indexOf(I)I` |  | 1.0 |  |  |
| 1 | `int indexOf(int ch, int fromIndex)` | `indexOf(II)I` |  | 1.0 |  |  |
| 2 | `int lastIndexOf(int ch)` | `lastIndexOf(I)I` |  | 1.0 |  |  |
| 2 | `int lastIndexOf(int ch, int fromIndex)` | `lastIndexOf(II)I` |  | 1.0 |  |  |
| 1 | `int indexOf(String str)` | `indexOf(Ljava/lang/String;)I` |  | 1.0 | `NullPointerException` if str is null |  |
| 1 | `int indexOf(String str, int fromIndex)` | `indexOf(Ljava/lang/String;I)I` |  | 1.0 | `NullPointerException` if str is null |  |
| 1 | `String substring(int beginIndex)` | `substring(I)Ljava/lang/String;` |  | 1.0 | `IndexOutOfBoundsException` if beginIndex is negative or larger than the length of this String object |  |
| 1 | `String substring(int beginIndex, int endIndex)` | `substring(II)Ljava/lang/String;` |  | 1.0 | `IndexOutOfBoundsException` if the beginIndex is negative, or endIndex is larger than the length of this String object, or beginIndex is larger than endIndex |  |
| 2 | `String concat(String str)` | `concat(Ljava/lang/String;)Ljava/lang/String;` |  | 1.0 | `NullPointerException` if str is null |  |
| 2 | `String replace(char oldChar, char newChar)` | `replace(CC)Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `String toLowerCase()` | `toLowerCase()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `String toUpperCase()` | `toUpperCase()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `String trim()` | `trim()Ljava/lang/String;` |  | 1.0 |  |  |
| 1 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `char[] toCharArray()` | `toCharArray()[C` |  | 1.0 |  |  |
| 1 | `String valueOf(Object obj)` | `valueOf(Ljava/lang/Object;)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String valueOf(char[] data)` | `valueOf([C)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String valueOf(char[] data, int offset, int count)` | `valueOf([CII)Ljava/lang/String;` | static | 1.0 | `NullPointerException` if data is null; `IndexOutOfBoundsException` if offset is negative, or count is negative, or offset+count is larger than data.length |  |
| 1 | `String valueOf(boolean b)` | `valueOf(Z)Ljava/lang/String;` | static | 1.0 |  |  |
| 1 | `String valueOf(char c)` | `valueOf(C)Ljava/lang/String;` | static | 1.0 |  |  |
| 1 | `String valueOf(int i)` | `valueOf(I)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String valueOf(long l)` | `valueOf(J)Ljava/lang/String;` | static | 1.0 |  |  |
| 2 | `String valueOf(float f)` | `valueOf(F)Ljava/lang/String;` | static | 1.1 |  |  |
| 2 | `String valueOf(double d)` | `valueOf(D)Ljava/lang/String;` | static | 1.1 |  |  |
| 3 | `String intern()` | `intern()Ljava/lang/String;` |  | 1.1 |  |  |

### StringBuffer

`public final class java.lang.StringBuffer extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc-1.1](https://nikita36078.github.io/J2ME_Docs/docs/cldc-1.1/java/lang/StringBuffer.html).

`javac` compiles string concatenation (`"Score: " + n`) to `new StringBuffer()`, `append` calls and `toString()`, so the `append` overloads run in every game. CLDC has no `StringBuilder`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(int length)` | `<init>(I)V` |  | 1.0 | `NegativeArraySizeException` if the length argument is less than 0 |  |
| 1 | `(String str)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int length()` | `length()I` |  | 1.0 |  |  |
| 3 | `int capacity()` | `capacity()I` |  | 1.0 |  |  |
| 3 | `void ensureCapacity(int minimumCapacity)` | `ensureCapacity(I)V` |  | 1.0 |  |  |
| 2 | `void setLength(int newLength)` | `setLength(I)V` |  | 1.0 | `IndexOutOfBoundsException` if the newLength argument is negative |  |
| 1 | `char charAt(int index)` | `charAt(I)C` |  | 1.0 | `IndexOutOfBoundsException` if index is negative or greater than or equal to length() |  |
| 3 | `void getChars(int srcBegin, int srcEnd, char[] dst, int dstBegin)` | `getChars(II[CI)V` |  | 1.0 | `NullPointerException` if dst is null; `IndexOutOfBoundsException` if any of the following is true: srcBegin is negative dstBegin is negative the srcBegin argument is greater than the srcEnd argument. srcEnd is greater than this.length(), the current length of this string buffer. dstBegin+srcEnd-srcBegin is greater than dst.length |  |
| 2 | `void setCharAt(int index, char ch)` | `setCharAt(IC)V` |  | 1.0 | `IndexOutOfBoundsException` if index is negative or greater than or equal to length() |  |
| 1 | `StringBuffer append(Object obj)` | `append(Ljava/lang/Object;)Ljava/lang/StringBuffer;` |  | 1.0 |  | Appends `String.valueOf(obj)`, which is `"null"` for null. |
| 1 | `StringBuffer append(String str)` | `append(Ljava/lang/String;)Ljava/lang/StringBuffer;` |  | 1.0 |  | Appends `"null"` for a null argument. |
| 2 | `StringBuffer append(char[] str)` | `append([C)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 2 | `StringBuffer append(char[] str, int offset, int len)` | `append([CII)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 1 | `StringBuffer append(boolean b)` | `append(Z)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 1 | `StringBuffer append(char c)` | `append(C)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 1 | `StringBuffer append(int i)` | `append(I)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 1 | `StringBuffer append(long l)` | `append(J)Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 2 | `StringBuffer append(float f)` | `append(F)Ljava/lang/StringBuffer;` |  | 1.1 |  |  |
| 2 | `StringBuffer append(double d)` | `append(D)Ljava/lang/StringBuffer;` |  | 1.1 |  |  |
| 2 | `StringBuffer delete(int start, int end)` | `delete(II)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if start is negative, greater than length(), or greater than end |  |
| 2 | `StringBuffer deleteCharAt(int index)` | `deleteCharAt(I)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the index is negative or greater than or equal to length() |  |
| 3 | `StringBuffer insert(int offset, Object obj)` | `insert(ILjava/lang/Object;)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 2 | `StringBuffer insert(int offset, String str)` | `insert(ILjava/lang/String;)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer insert(int offset, char[] str)` | `insert(I[C)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer insert(int offset, boolean b)` | `insert(IZ)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 2 | `StringBuffer insert(int offset, char c)` | `insert(IC)Ljava/lang/StringBuffer;` |  | 1.0 | `IndexOutOfBoundsException` if the offset is invalid |  |
| 2 | `StringBuffer insert(int offset, int i)` | `insert(II)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer insert(int offset, long l)` | `insert(IJ)Ljava/lang/StringBuffer;` |  | 1.0 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer insert(int offset, float f)` | `insert(IF)Ljava/lang/StringBuffer;` |  | 1.1 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer insert(int offset, double d)` | `insert(ID)Ljava/lang/StringBuffer;` |  | 1.1 | `StringIndexOutOfBoundsException` if the offset is invalid |  |
| 3 | `StringBuffer reverse()` | `reverse()Ljava/lang/StringBuffer;` |  | 1.0 |  |  |
| 1 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |

### StringIndexOutOfBoundsException

`public class java.lang.StringIndexOutOfBoundsException extends java.lang.IndexOutOfBoundsException`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/StringIndexOutOfBoundsException.html).

Thrown by `String` and `StringBuffer` index checks.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 1 | `(int index)` | `<init>(I)V` |  | 1.0 |  |  |

### System

`public final class java.lang.System extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/System.html).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `out` | `Ljava/io/PrintStream;` | (not a constant) | static | 1.0 | A `PrintStream`; on phones the output went nowhere or to a debug log. FastEmu sends it to the instance log. |
| 3 | `err` | `Ljava/io/PrintStream;` | (not a constant) | static | 1.0 | Same as `out`. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `long currentTimeMillis()` | `currentTimeMillis()J` | static | 1.0 |  | Milliseconds since 1970-01-01 UTC. Games time frames with it, so it must move smoothly and never go backwards. |
| 1 | `void arraycopy(Object src, int srcOffset, Object dst, int dstOffset, int length)` | `arraycopy(Ljava/lang/Object;ILjava/lang/Object;II)V` | static | 1.0 | `IndexOutOfBoundsException` if copying would cause access of data outside array bounds; `ArrayStoreException` if an element in the src array could not be stored into the dest array because of a type mismatch; `NullPointerException` if either src or dst is null | Handles overlapping ranges in the same array as if through a temporary copy. Throws `ArrayStoreException` for incompatible arrays and `IndexOutOfBoundsException` for bad ranges, without copying anything. |
| 3 | `int identityHashCode(Object x)` | `identityHashCode(Ljava/lang/Object;)I` | static | 1.0 |  |  |
| 1 | `String getProperty(String key)` | `getProperty(Ljava/lang/String;)Ljava/lang/String;` | static | 1.0 | `NullPointerException` if key is null; `IllegalArgumentException` if key is empty | Returns `null` for keys the phone does not define. Every key games read is in [system-properties.md](system-properties.md). |
| 3 | `void exit(int status)` | `exit(I)V` | static | 1.0 |  | MIDP says `System.exit` MUST throw `SecurityException` when a MIDlet calls it; games that call it expect to stop, so FastEmu may treat it as `notifyDestroyed` but must log it. |
| 1 | `void gc()` | `gc()V` | static | 1.0 |  | Games call it often, sometimes every frame. It must be cheap when there is little garbage. |

### Thread

`public class java.lang.Thread extends java.lang.Object implements java.lang.Runnable`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Thread.html).

Games run their main loop in their own thread and draw from it; many also start helper threads for loading or sound.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `MIN_PRIORITY` | `I` | `1` | static | 1.0 |  |
| 2 | `NORM_PRIORITY` | `I` | `5` | static | 1.0 |  |
| 2 | `MAX_PRIORITY` | `I` | `10` | static | 1.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 3 | `(String name)` | `<init>(Ljava/lang/String;)V` |  | 1.1 |  |  |
| 1 | `(Runnable target)` | `<init>(Ljava/lang/Runnable;)V` |  | 1.0 |  |  |
| 3 | `(Runnable target, String name)` | `<init>(Ljava/lang/Runnable;Ljava/lang/String;)V` |  | 1.1 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `Thread currentThread()` | `currentThread()Ljava/lang/Thread;` | static | 1.0 |  |  |
| 1 | `void yield()` | `yield()V` | static | 1.0 |  | Some games busy-wait with it between frames. |
| 1 | `void sleep(long millis)` | `sleep(J)V` | static | 1.0 | `InterruptedException` if another thread has interrupted the current thread. The interrupted status of the current thread is cleared when this exception is thrown | Games call it every frame to cap the frame rate; it must give the CPU to other threads (key events, repaints) and honor the time. |
| 1 | `void start()` | `start()V` |  | 1.0 | `IllegalThreadStateException` if the thread was already started |  |
| 2 | `void run()` | `run()V` |  | 1.0 |  |  |
| 2 | `void interrupt()` | `interrupt()V` |  | 1.1 |  | CLDC 1.1 only. Wakes a thread in `sleep`, `wait` or `join` with `InterruptedException`. |
| 2 | `boolean isAlive()` | `isAlive()Z` | final | 1.0 |  |  |
| 2 | `void setPriority(int newPriority)` | `setPriority(I)V` | final | 1.0 | `IllegalArgumentException` If the priority is not in the range MIN_PRIORITY to MAX_PRIORITY | Games raise their loop thread's priority; FastEmu can accept and ignore it. |
| 3 | `int getPriority()` | `getPriority()I` | final | 1.0 |  |  |
| 3 | `String getName()` | `getName()Ljava/lang/String;` | final | 1.1 |  |  |
| 3 | `int activeCount()` | `activeCount()I` | static | 1.0 |  |  |
| 2 | `void join()` | `join()V` | final | 1.0 | `InterruptedException` if another thread has interrupted the current thread. The interrupted status of the current thread is cleared when this exception is thrown |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |

### Throwable

`public class java.lang.Throwable extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/Throwable.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |
| 1 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `String getMessage()` | `getMessage()Ljava/lang/String;` |  | 1.0 |  |  |
| 2 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |
| 1 | `void printStackTrace()` | `printStackTrace()V` |  | 1.0 |  | Prints to `System.err`. Called in most `catch` blocks. |

### VirtualMachineError

`public abstract class java.lang.VirtualMachineError extends java.lang.Error`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/VirtualMachineError.html).

Base of `OutOfMemoryError`. CLDC has no `StackOverflowError`, so FastEmu must report stack overflow with a class that exists, such as `VirtualMachineError`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### IllegalStateException

`public class java.lang.IllegalStateException extends java.lang.RuntimeException`

Tier 1. Since MIDP 1.0 (not CLDC). Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/lang/IllegalStateException.html).

Not in CLDC: MIDP adds it. Thrown by `Timer.schedule` on a cancelled timer or task, by LCDUI when an `Item` is already owned, and by MMAPI for calls in the wrong player state.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | MIDP 1.0 (not CLDC) |  |  |
| 1 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | MIDP 1.0 (not CLDC) |  |  |

## java.lang.ref

### Reference

`public abstract class java.lang.ref.Reference extends java.lang.Object`

Tier 3. Since CLDC 1.1. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ref/Reference.html).

CLDC 1.1 only.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Object get()` | `get()Ljava/lang/Object;` |  | 1.1 |  |  |
| 3 | `void clear()` | `clear()V` |  | 1.1 |  |  |

### WeakReference

`public class java.lang.ref.WeakReference extends java.lang.ref.Reference`

Tier 3. Since CLDC 1.1. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/lang/ref/WeakReference.html).

CLDC 1.1 only. The CLDC 1.1 javadoc documents `get` and `clear` on `WeakReference` itself; the cldcapi11 stub inherits them from `Reference`. Resolution finds them either way.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(Object referent)` | `<init>(Ljava/lang/Object;)V` |  | 1.1 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Object get()` | `get()Ljava/lang/Object;` |  | 1.1 |  |  |
| 3 | `void clear()` | `clear()V` |  | 1.1 |  |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. What differs is listed here; a class not listed matched everywhere it appears.

- `java.lang.ref.WeakReference`
  - cldcapi11.jar does not declare `clear()V`, `get()Ljava/lang/Object;`
