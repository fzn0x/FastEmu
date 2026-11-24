# javax.microedition.pki (MIDP 2.0)

`javax.microedition.pki`, new in MIDP 2.0: the certificate a secure
connection presents (`SecurityInfo.getServerCertificate()` in
[midp-io.md](midp-io.md)) and the exception for a failed certificate
check. Games almost never touch it; it matters only if FastEmu opens
`https://` or `ssl://` connections. When it does, the host's TLS stack
does the checking, and FastEmu only has to fill in a `Certificate` and,
on failure, throw `CertificateException` with the closest reason code.

Column meanings and tier rules: [README](README.md).

## Sources

- [MIDP 2.0 javadoc (JSR-118)](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [Certificate](#certificate) | interface | 3 | MIDP 2.0 | 8 |
| [CertificateException](#certificateexception) | class | 3 | MIDP 2.0 | 18 |

## javax.microedition.pki

### Certificate

`public interface javax.microedition.pki.Certificate`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/pki/Certificate.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/pki/Certificate.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/pki/Certificate.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/pki/Certificate.java).

Read-only view of an X.509 certificate. Dates are milliseconds since 1970 like `System.currentTimeMillis()`.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `String getSubject()` | `getSubject()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getIssuer()` | `getIssuer()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getType()` | `getType()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getVersion()` | `getVersion()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `String getSigAlgName()` | `getSigAlgName()Ljava/lang/String;` |  | 2.0 |  |  |
| 3 | `long getNotBefore()` | `getNotBefore()J` |  | 2.0 |  |  |
| 3 | `long getNotAfter()` | `getNotAfter()J` |  | 2.0 |  |  |
| 3 | `String getSerialNumber()` | `getSerialNumber()Ljava/lang/String;` |  | 2.0 |  |  |

### CertificateException

`public class javax.microedition.pki.CertificateException extends java.io.IOException`

Tier 3. Since MIDP 2.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/pki/CertificateException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/pki/CertificateException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/pki/CertificateException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/pki/CertificateException.java).

A subclass of `IOException`. The reason codes are `byte` constants.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `BAD_EXTENSIONS` | `B` | `1` | static | 2.0 |  |
| 3 | `CERTIFICATE_CHAIN_TOO_LONG` | `B` | `2` | static | 2.0 |  |
| 3 | `EXPIRED` | `B` | `3` | static | 2.0 |  |
| 3 | `UNAUTHORIZED_INTERMEDIATE_CA` | `B` | `4` | static | 2.0 |  |
| 3 | `MISSING_SIGNATURE` | `B` | `5` | static | 2.0 |  |
| 3 | `NOT_YET_VALID` | `B` | `6` | static | 2.0 |  |
| 3 | `SITENAME_MISMATCH` | `B` | `7` | static | 2.0 |  |
| 3 | `UNRECOGNIZED_ISSUER` | `B` | `8` | static | 2.0 |  |
| 3 | `UNSUPPORTED_SIGALG` | `B` | `9` | static | 2.0 |  |
| 3 | `INAPPROPRIATE_KEY_USAGE` | `B` | `10` | static | 2.0 |  |
| 3 | `BROKEN_CHAIN` | `B` | `11` | static | 2.0 |  |
| 3 | `ROOT_CA_EXPIRED` | `B` | `12` | static | 2.0 |  |
| 3 | `UNSUPPORTED_PUBLIC_KEY_TYPE` | `B` | `13` | static | 2.0 |  |
| 3 | `VERIFICATION_FAILED` | `B` | `14` | static | 2.0 |  |

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `(Certificate certificate, byte status)` | `<init>(Ljavax/microedition/pki/Certificate;B)V` |  | 2.0 |  |  |
| 3 | `(String message, Certificate certificate, byte status)` | `<init>(Ljava/lang/String;Ljavax/microedition/pki/Certificate;B)V` |  | 2.0 |  |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `Certificate getCertificate()` | `getCertificate()Ljavax/microedition/pki/Certificate;` |  | 2.0 |  |  |
| 3 | `byte getReason()` | `getReason()B` |  | 2.0 |  |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
