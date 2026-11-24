# java.util (CLDC 1.1, plus MIDP's Timer)

The `java.util` classes of CLDC 1.1, plus `Timer` and `TimerTask`, which
MIDP adds. CLDC has no collections framework: no `List`, `Map`,
`ArrayList`, `HashMap` or `Iterator`. Games use `Vector`, `Hashtable` and
`Enumeration`, and arrays.

Column meanings and tier rules: [README](README.md).

## Sources

- [CLDC 1.1 javadoc (JSR-139)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- [CLDC 1.0 javadoc (JSR-30)](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.0/jsr030/index.html): compared member by member to mark what CLDC 1.0 lacks (Since column).
- MicroEmulator 2.0.4 API stub jars [cldcapi10](https://repo1.maven.org/maven2/org/microemu/cldcapi10/2.0.4/cldcapi10-2.0.4.jar) and [cldcapi11](https://repo1.maven.org/maven2/org/microemu/cldcapi11/2.0.4/cldcapi11-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [MIDP 2.0 javadoc](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/util/Timer.html): `Timer` and `TimerTask`, which MIDP adds to `java.util`.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [Calendar](#calendar) | class | 2 | CLDC 1.0 | 51 |
| [Date](#date) | class | 2 | CLDC 1.0 | 7 |
| [EmptyStackException](#emptystackexception) | class | 3 | CLDC 1.0 | 1 |
| [Enumeration](#enumeration) | interface | 1 | CLDC 1.0 | 2 |
| [Hashtable](#hashtable) | class | 1 | CLDC 1.0 | 14 |
| [NoSuchElementException](#nosuchelementexception) | class | 2 | CLDC 1.0 | 2 |
| [Random](#random) | class | 1 | CLDC 1.0 | 9 |
| [Stack](#stack) | class | 2 | CLDC 1.0 | 6 |
| [TimeZone](#timezone) | class | 3 | CLDC 1.0 | 8 |
| [Vector](#vector) | class | 1 | CLDC 1.0 | 29 |
| [Timer](#timer) | class | 2 | MIDP 1.0 (not CLDC) | 8 |
| [TimerTask](#timertask) | class | 2 | MIDP 1.0 (not CLDC) | 4 |

## java.util

### Calendar

`public abstract class java.util.Calendar extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Calendar.html).

Games use it for clocks, daily bonuses and save dates. Field numbers and month and weekday values are the Java SE ones: `MONTH` counts from `JANUARY` = 0, `DAY_OF_WEEK` from `SUNDAY` = 1.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `YEAR` | `I` | `1` | static | 1.0 |  |
| 2 | `MONTH` | `I` | `2` | static | 1.0 |  |
| 2 | `DATE` | `I` | `5` | static | 1.0 |  |
| 2 | `DAY_OF_MONTH` | `I` | `5` | static | 1.0 |  |
| 2 | `DAY_OF_WEEK` | `I` | `7` | static | 1.0 |  |
| 3 | `AM_PM` | `I` | `9` | static | 1.0 |  |
| 3 | `HOUR` | `I` | `10` | static | 1.0 |  |
| 2 | `HOUR_OF_DAY` | `I` | `11` | static | 1.0 |  |
| 2 | `MINUTE` | `I` | `12` | static | 1.0 |  |
| 2 | `SECOND` | `I` | `13` | static | 1.0 |  |
| 3 | `MILLISECOND` | `I` | `14` | static | 1.0 |  |
| 2 | `SUNDAY` | `I` | `1` | static | 1.0 |  |
| 2 | `MONDAY` | `I` | `2` | static | 1.0 |  |
| 2 | `TUESDAY` | `I` | `3` | static | 1.0 |  |
| 2 | `WEDNESDAY` | `I` | `4` | static | 1.0 |  |
| 2 | `THURSDAY` | `I` | `5` | static | 1.0 |  |
| 2 | `FRIDAY` | `I` | `6` | static | 1.0 |  |
| 2 | `SATURDAY` | `I` | `7` | static | 1.0 |  |
| 2 | `JANUARY` | `I` | `0` | static | 1.0 |  |
| 2 | `FEBRUARY` | `I` | `1` | static | 1.0 |  |
| 2 | `MARCH` | `I` | `2` | static | 1.0 |  |
| 2 | `APRIL` | `I` | `3` | static | 1.0 |  |
| 2 | `MAY` | `I` | `4` | static | 1.0 |  |
| 2 | `JUNE` | `I` | `5` | static | 1.0 |  |
| 2 | `JULY` | `I` | `6` | static | 1.0 |  |
| 2 | `AUGUST` | `I` | `7` | static | 1.0 |  |
| 2 | `SEPTEMBER` | `I` | `8` | static | 1.0 |  |
| 2 | `OCTOBER` | `I` | `9` | static | 1.0 |  |
| 2 | `NOVEMBER` | `I` | `10` | static | 1.0 |  |
| 2 | `DECEMBER` | `I` | `11` | static | 1.0 |  |
| 3 | `AM` | `I` | `0` | static | 1.0 |  |
| 3 | `PM` | `I` | `1` | static | 1.0 |  |
| 3 | `fields` | `[I` | (not a constant) | protected not final | 1.1 |  |
| 3 | `isSet` | `[Z` | (not a constant) | protected not final | 1.1 |  |
| 3 | `time` | `J` | (not a constant) | protected not final | 1.1 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` | protected | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `Date getTime()` | `getTime()Ljava/util/Date;` | final | 1.0 |  |  |
| 2 | `void setTime(Date date)` | `setTime(Ljava/util/Date;)V` | final | 1.0 |  |  |
| 2 | `Calendar getInstance()` | `getInstance()Ljava/util/Calendar;` | static | 1.0 |  |  |
| 3 | `Calendar getInstance(TimeZone zone)` | `getInstance(Ljava/util/TimeZone;)Ljava/util/Calendar;` | static | 1.0 |  |  |
| 3 | `long getTimeInMillis()` | `getTimeInMillis()J` | protected | 1.0 |  | `protected` in CLDC, unlike Java SE 1.4 and later where it is public. A game cannot call it. |
| 3 | `void setTimeInMillis(long millis)` | `setTimeInMillis(J)V` | protected | 1.0 |  | `protected` in CLDC. |
| 2 | `int get(int field)` | `get(I)I` | final | 1.0 | `ArrayIndexOutOfBoundsException` if the parameter is not one of the above |  |
| 2 | `void set(int field, int value)` | `set(II)V` | final | 1.0 | `ArrayIndexOutOfBoundsException` if an illegal field parameter is received |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 3 | `boolean before(Object when)` | `before(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 3 | `boolean after(Object when)` | `after(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 3 | `void setTimeZone(TimeZone value)` | `setTimeZone(Ljava/util/TimeZone;)V` |  | 1.0 |  |  |
| 3 | `TimeZone getTimeZone()` | `getTimeZone()Ljava/util/TimeZone;` |  | 1.0 |  |  |
| 3 | `void computeFields()` | `computeFields()V` | protected abstract | 1.1 |  |  |
| 3 | `void computeTime()` | `computeTime()V` | protected abstract | 1.1 |  |  |

### Date

`public class java.util.Date extends java.lang.Object`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Date.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  | The current time, from `System.currentTimeMillis()`. |
| 2 | `(long date)` | `<init>(J)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `long getTime()` | `getTime()J` |  | 1.0 |  |  |
| 2 | `void setTime(long time)` | `setTime(J)V` |  | 1.0 |  |  |
| 3 | `boolean equals(Object obj)` | `equals(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 3 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.1 |  |  |

### EmptyStackException

`public class java.util.EmptyStackException extends java.lang.RuntimeException`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/EmptyStackException.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  | 1.0 |  |  |

### Enumeration

`public interface java.util.Enumeration`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Enumeration.html).

What `Vector.elements()`, `Hashtable.keys()` and `Hashtable.elements()` return.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `boolean hasMoreElements()` | `hasMoreElements()Z` |  | 1.0 |  |  |
| 1 | `Object nextElement()` | `nextElement()Ljava/lang/Object;` |  | 1.0 | `NoSuchElementException` if no more elements exist |  |

### Hashtable

`public class java.util.Hashtable extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Hashtable.html).

Neither key nor value may be null. Iteration order of `keys()` and `elements()` is unspecified; J2ME Loader and freej2me-plus run games on their host's JDK-derived `java.util.Hashtable`, so following the JDK's bucket order avoids surprises in games that save or draw in key order.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(int initialCapacity)` | `<init>(I)V` |  | 1.0 | `IllegalArgumentException` if the initial capacity is less than zero |  |
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int size()` | `size()I` |  | 1.0 |  |  |
| 1 | `boolean isEmpty()` | `isEmpty()Z` |  | 1.0 |  |  |
| 1 | `Enumeration keys()` | `keys()Ljava/util/Enumeration;` |  | 1.0 |  |  |
| 1 | `Enumeration elements()` | `elements()Ljava/util/Enumeration;` |  | 1.0 |  |  |
| 2 | `boolean contains(Object value)` | `contains(Ljava/lang/Object;)Z` |  | 1.0 | `NullPointerException` if the value is null | Searches values, not keys. |
| 1 | `boolean containsKey(Object key)` | `containsKey(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 1 | `Object get(Object key)` | `get(Ljava/lang/Object;)Ljava/lang/Object;` |  | 1.0 |  |  |
| 3 | `void rehash()` | `rehash()V` | protected | 1.0 |  |  |
| 1 | `Object put(Object key, Object value)` | `put(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;` |  | 1.0 | `NullPointerException` if the key or value is null |  |
| 1 | `Object remove(Object key)` | `remove(Ljava/lang/Object;)Ljava/lang/Object;` |  | 1.0 |  |  |
| 1 | `void clear()` | `clear()V` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |

### NoSuchElementException

`public class java.util.NoSuchElementException extends java.lang.RuntimeException`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/NoSuchElementException.html).

Thrown by `Enumeration.nextElement` past the end and by `Vector.firstElement` and `lastElement` on an empty vector.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |
| 2 | `(String s)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |

### Random

`public class java.util.Random extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Random.html).

The javadoc fixes the algorithm: a 48-bit linear congruential generator with multiplier `0x5DEECE66DL` and addend `0xBL`, as in Java SE. Games seeded with a constant get the same sequence on every phone and build levels from it, so FastEmu must use exactly this generator.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `()` | `<init>()V` |  | 1.0 |  | Seeds from `System.currentTimeMillis()`. |
| 1 | `(long seed)` | `<init>(J)V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void setSeed(long seed)` | `setSeed(J)V` |  | 1.0 |  | Scrambles the seed with `0x5DEECE66DL` and keeps 48 bits. |
| 3 | `int next(int bits)` | `next(I)I` | protected | 1.0 |  | `protected`; the core of the generator. |
| 1 | `int nextInt()` | `nextInt()I` |  | 1.0 |  |  |
| 1 | `int nextInt(int n)` | `nextInt(I)I` |  | 1.1 | `IllegalArgumentException` n is not positive | CLDC 1.1 only. Rejection sampling as in Java SE; CLDC 1.0 games use `Math.abs(nextInt()) % n` or `nextInt() >>> 1` instead. |
| 2 | `long nextLong()` | `nextLong()J` |  | 1.0 |  |  |
| 2 | `float nextFloat()` | `nextFloat()F` |  | 1.1 |  |  |
| 2 | `double nextDouble()` | `nextDouble()D` |  | 1.1 |  |  |

### Stack

`public class java.util.Stack extends java.util.Vector`

Tier 2. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Stack.html).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `Object push(Object item)` | `push(Ljava/lang/Object;)Ljava/lang/Object;` |  | 1.0 |  |  |
| 2 | `Object pop()` | `pop()Ljava/lang/Object;` |  | 1.0 | `EmptyStackException` if this stack is empty |  |
| 2 | `Object peek()` | `peek()Ljava/lang/Object;` |  | 1.0 | `EmptyStackException` if this stack is empty |  |
| 2 | `boolean empty()` | `empty()Z` |  | 1.0 |  |  |
| 3 | `int search(Object o)` | `search(Ljava/lang/Object;)I` |  | 1.0 |  |  |

### TimeZone

`public abstract class java.util.TimeZone extends java.lang.Object`

Tier 3. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/TimeZone.html).

Only `"GMT"` is required. `getTimeZone` returns GMT for IDs it does not understand.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int getOffset(int era, int year, int month, int day, int dayOfWeek, int millis)` | `getOffset(IIIIII)I` | abstract | 1.0 | `IllegalArgumentException` the era, month, day, dayOfWeek, or millis parameters are out of range |  |
| 2 | `int getRawOffset()` | `getRawOffset()I` | abstract | 1.0 |  |  |
| 3 | `boolean useDaylightTime()` | `useDaylightTime()Z` | abstract | 1.0 |  |  |
| 3 | `String getID()` | `getID()Ljava/lang/String;` |  | 1.0 |  |  |
| 3 | `TimeZone getTimeZone(String ID)` | `getTimeZone(Ljava/lang/String;)Ljava/util/TimeZone;` | static | 1.0 |  |  |
| 2 | `TimeZone getDefault()` | `getDefault()Ljava/util/TimeZone;` | static | 1.0 |  |  |
| 3 | `String[] getAvailableIDs()` | `getAvailableIDs()[Ljava/lang/String;` | static | 1.0 |  |  |

### Vector

`public class java.util.Vector extends java.lang.Object`

Tier 1. Since CLDC 1.0. Spec: [cldc11](https://docs.oracle.com/javame/config/cldc/ref-impl/cldc1.1/jsr139/java/util/Vector.html).

The general-purpose list of CLDC. It accepts null elements.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `elementData` | `[Ljava/lang/Object;` | (not a constant) | protected not final | 1.0 | `protected` field; subclasses can read it. |
| 3 | `elementCount` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |
| 3 | `capacityIncrement` | `I` | (not a constant) | protected not final | 1.0 | `protected` field. |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(int initialCapacity, int capacityIncrement)` | `<init>(II)V` |  | 1.0 | `IllegalArgumentException` if the specified initial capacity is negative |  |
| 1 | `(int initialCapacity)` | `<init>(I)V` |  | 1.0 |  |  |
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void copyInto(Object[] anArray)` | `copyInto([Ljava/lang/Object;)V` |  | 1.0 |  |  |
| 3 | `void trimToSize()` | `trimToSize()V` |  | 1.0 |  |  |
| 3 | `void ensureCapacity(int minCapacity)` | `ensureCapacity(I)V` |  | 1.0 |  |  |
| 2 | `void setSize(int newSize)` | `setSize(I)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if new size is negative |  |
| 3 | `int capacity()` | `capacity()I` |  | 1.0 |  |  |
| 1 | `int size()` | `size()I` |  | 1.0 |  |  |
| 1 | `boolean isEmpty()` | `isEmpty()Z` |  | 1.0 |  |  |
| 1 | `Enumeration elements()` | `elements()Ljava/util/Enumeration;` |  | 1.0 |  |  |
| 1 | `boolean contains(Object elem)` | `contains(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 1 | `int indexOf(Object elem)` | `indexOf(Ljava/lang/Object;)I` |  | 1.0 |  |  |
| 2 | `int indexOf(Object elem, int index)` | `indexOf(Ljava/lang/Object;I)I` |  | 1.0 |  |  |
| 2 | `int lastIndexOf(Object elem)` | `lastIndexOf(Ljava/lang/Object;)I` |  | 1.0 |  |  |
| 2 | `int lastIndexOf(Object elem, int index)` | `lastIndexOf(Ljava/lang/Object;I)I` |  | 1.0 | `IndexOutOfBoundsException` if index is greater than or equal to the current size of this vector |  |
| 1 | `Object elementAt(int index)` | `elementAt(I)Ljava/lang/Object;` |  | 1.0 | `ArrayIndexOutOfBoundsException` if an invalid index was given |  |
| 2 | `Object firstElement()` | `firstElement()Ljava/lang/Object;` |  | 1.0 | `NoSuchElementException` if this vector has no components |  |
| 2 | `Object lastElement()` | `lastElement()Ljava/lang/Object;` |  | 1.0 | `NoSuchElementException` if this vector is empty |  |
| 1 | `void setElementAt(Object obj, int index)` | `setElementAt(Ljava/lang/Object;I)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if the index was invalid |  |
| 1 | `void removeElementAt(int index)` | `removeElementAt(I)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if the index was invalid |  |
| 1 | `void insertElementAt(Object obj, int index)` | `insertElementAt(Ljava/lang/Object;I)V` |  | 1.0 | `ArrayIndexOutOfBoundsException` if the index was invalid |  |
| 1 | `void addElement(Object obj)` | `addElement(Ljava/lang/Object;)V` |  | 1.0 |  |  |
| 1 | `boolean removeElement(Object obj)` | `removeElement(Ljava/lang/Object;)Z` |  | 1.0 |  |  |
| 1 | `void removeAllElements()` | `removeAllElements()V` |  | 1.0 |  |  |
| 3 | `String toString()` | `toString()Ljava/lang/String;` |  | 1.0 |  |  |

### Timer

`public class java.util.Timer extends java.lang.Object`

Tier 2. Since MIDP 1.0 (not CLDC). Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/util/Timer.html).

Not in CLDC: MIDP adds it. Each `Timer` runs its tasks one after another on its own background thread. Some games drive their main loop with a repeating task.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` |  | MIDP 1.0 (not CLDC) |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void schedule(TimerTask task, long delay)` | `schedule(Ljava/util/TimerTask;J)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if delay is negative, or delay + System.currentTimeMillis() is negative; `IllegalStateException` if task was already scheduled or cancelled, or timer was cancelled |  |
| 3 | `void schedule(TimerTask task, Date time)` | `schedule(Ljava/util/TimerTask;Ljava/util/Date;)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if time.getTime() is negative; `IllegalStateException` if task was already scheduled or cancelled, timer was cancelled, or timer thread terminated |  |
| 2 | `void schedule(TimerTask task, long delay, long period)` | `schedule(Ljava/util/TimerTask;JJ)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if delay is negative, or delay + System.currentTimeMillis() is negative; `IllegalStateException` if task was already scheduled or cancelled, timer was cancelled, or timer thread terminated | Fixed delay: each run is scheduled from the actual start of the previous one, so a slow run delays the rest. |
| 3 | `void schedule(TimerTask task, Date firstTime, long period)` | `schedule(Ljava/util/TimerTask;Ljava/util/Date;J)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if time.getTime() is negative; `IllegalStateException` if task was already scheduled or cancelled, timer was cancelled, or timer thread terminated |  |
| 2 | `void scheduleAtFixedRate(TimerTask task, long delay, long period)` | `scheduleAtFixedRate(Ljava/util/TimerTask;JJ)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if delay is negative, or delay + System.currentTimeMillis() is negative; `IllegalStateException` if task was already scheduled or cancelled, timer was cancelled, or timer thread terminated | Fixed rate: each run is scheduled from the first run's scheduled time, so late runs are followed by catch-up runs in quick succession. |
| 3 | `void scheduleAtFixedRate(TimerTask task, Date firstTime, long period)` | `scheduleAtFixedRate(Ljava/util/TimerTask;Ljava/util/Date;J)V` |  | MIDP 1.0 (not CLDC) | `IllegalArgumentException` if time.getTime() is negative; `IllegalStateException` if task was already scheduled or cancelled, timer was cancelled, or timer thread terminated |  |
| 2 | `void cancel()` | `cancel()V` |  | MIDP 1.0 (not CLDC) |  | Ends the timer; further `schedule` calls throw `IllegalStateException`. A running task finishes. |

### TimerTask

`public abstract class java.util.TimerTask extends java.lang.Object implements java.lang.Runnable`

Tier 2. Since MIDP 1.0 (not CLDC). Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/java/util/TimerTask.html).

Not in CLDC: MIDP adds it. A task may be scheduled only once; scheduling it again throws `IllegalStateException`.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `()` | `<init>()V` | protected | MIDP 1.0 (not CLDC) |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void run()` | `run()V` | abstract | MIDP 1.0 (not CLDC) |  |  |
| 2 | `boolean cancel()` | `cancel()Z` |  | MIDP 1.0 (not CLDC) |  |  |
| 3 | `long scheduledExecutionTime()` | `scheduledExecutionTime()J` |  | MIDP 1.0 (not CLDC) |  |  |

## Cross-check

Every member above comes from the javadoc named first under Sources. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
