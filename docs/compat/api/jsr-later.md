# Optional APIs for later versions: M3G, Mascot Capsule, files, messaging and more

Optional packages that some games need and that FastEmu leaves for later
versions. This file is shorter than the others: for M3G, MascotCapsule,
FileConnection and WMA it lists every member with its descriptor and
constant values but without throws entries or notes; for PIM, Bluetooth,
Location and SVG it lists only the classes, with links to their javadoc.
The rules of [README](README.md) still apply, and the javadoc linked from
each class holds the semantics.

A game tests for each package before using it, through its
`*.version` system property ([system-properties.md](system-properties.md))
or `Class.forName`. Until FastEmu implements a package, it must leave the
property `null` and the classes absent, so those games take their
fallback path.

Column meanings and tier rules: [README](README.md).

## Sources

- [J2ME_Docs](https://github.com/nikita36078/J2ME_Docs/tree/fbfdfed7009369b6bf7a5cce39b8a13f178c2c69/docs) at commit `fbfdfed7009369b6bf7a5cce39b8a13f178c2c69`: a collection of vendor and JSR javadoc sets kept by the J2ME Loader author, browsable at [nikita36078.github.io/J2ME_Docs](https://nikita36078.github.io/J2ME_Docs/). The javadoc sets used here: [jsr184](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/overview-summary.html) (M3G 1.1), [Mascot_Capsule_API](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/index.html) (MascotCapsule micro3D V3), [jsr75](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/overview-summary.html) and [jsr75-pim](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/overview-summary.html), [jsr120](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/overview-summary.html), [jsr179-1_1-mrel-javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/overview-summary.html) and [jsr226](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/overview-summary.html).
- [JSR-82 javadoc](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/index.html), hosted by Oracle.
- MicroEmulator 2.0.4 jars [microemu-jsr-75](https://repo1.maven.org/maven2/org/microemu/microemu-jsr-75/2.0.4/microemu-jsr-75-2.0.4.jar), [microemu-jsr-82](https://repo1.maven.org/maven2/org/microemu/microemu-jsr-82/2.0.4/microemu-jsr-82-2.0.4.jar) and [microemu-jsr-120](https://repo1.maven.org/maven2/org/microemu/microemu-jsr-120/2.0.4/microemu-jsr-120-2.0.4.jar) (Maven Central), read for descriptors and constant values.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Members |
|---|---|---|---|
| [AnimationController](#animationcontroller) | class | 3 | 11 |
| [AnimationTrack](#animationtrack) | class | 3 | 26 |
| [Appearance](#appearance) | class | 2 | 13 |
| [Background](#background) | class | 2 | 19 |
| [Camera](#camera) | class | 2 | 9 |
| [CompositingMode](#compositingmode) | class | 2 | 21 |
| [Fog](#fog) | class | 3 | 12 |
| [javax.microedition.m3g.Graphics3D](#javaxmicroeditionm3ggraphics3d) | class | 2 | 32 |
| [Group](#group) | class | 2 | 7 |
| [Image2D](#image2d) | class | 2 | 14 |
| [IndexBuffer](#indexbuffer) | class | 2 | 2 |
| [KeyframeSequence](#keyframesequence) | class | 3 | 20 |
| [javax.microedition.m3g.Light](#javaxmicroeditionm3glight) | class | 2 | 19 |
| [Loader](#loader) | class | 2 | 2 |
| [Material](#material) | class | 2 | 11 |
| [Mesh](#mesh) | class | 2 | 7 |
| [MorphingMesh](#morphingmesh) | class | 3 | 6 |
| [javax.microedition.m3g.Node](#javaxmicroeditionm3gnode) | class | 2 | 19 |
| [Object3D](#object3d) | class | 2 | 12 |
| [PolygonMode](#polygonmode) | class | 2 | 20 |
| [RayIntersection](#rayintersection) | class | 3 | 10 |
| [SkinnedMesh](#skinnedmesh) | class | 3 | 6 |
| [Sprite3D](#sprite3d) | class | 2 | 11 |
| [Texture2D](#texture2d) | class | 2 | 23 |
| [Transform](#transform) | class | 2 | 15 |
| [Transformable](#transformable) | class | 2 | 13 |
| [TriangleStripArray](#trianglestriparray) | class | 2 | 2 |
| [VertexArray](#vertexarray) | class | 2 | 8 |
| [VertexBuffer](#vertexbuffer) | class | 2 | 12 |
| [World](#world) | class | 2 | 5 |
| [ActionTable](#actiontable) | class | 2 | 7 |
| [AffineTrans](#affinetrans) | class | 2 | 42 |
| [Effect3D](#effect3d) | class | 2 | 26 |
| [Figure](#figure) | class | 2 | 11 |
| [FigureLayout](#figurelayout) | class | 2 | 18 |
| [com.mascotcapsule.micro3d.v3.Graphics3D](#commascotcapsulemicro3dv3graphics3d) | class | 2 | 57 |
| [com.mascotcapsule.micro3d.v3.Light](#commascotcapsulemicro3dv3light) | class | 2 | 14 |
| [Texture](#texture) | class | 2 | 3 |
| [Util3D](#util3d) | class | 2 | 3 |
| [Vector3D](#vector3d) | class | 2 | 19 |
| [ConnectionClosedException](#connectionclosedexception) | class | 3 | 2 |
| [FileConnection](#fileconnection) | interface | 3 | 31 |
| [FileSystemListener](#filesystemlistener) | interface | 3 | 3 |
| [FileSystemRegistry](#filesystemregistry) | class | 3 | 3 |
| [IllegalModeException](#illegalmodeexception) | class | 3 | 2 |
| [BinaryMessage](#binarymessage) | interface | 3 | 2 |
| [Message](#message) | interface | 3 | 3 |
| [MessageConnection](#messageconnection) | interface | 3 | 8 |
| [MessageListener](#messagelistener) | interface | 3 | 1 |
| [TextMessage](#textmessage) | interface | 3 | 2 |
| `Contact` | interface | 3 | 45 |
| `ContactList` | interface | 3 | 3 |
| `javax.microedition.pim.Event` | interface | 3 | 14 |
| `EventList` | interface | 3 | 8 |
| `FieldEmptyException` | class | 3 | 4 |
| `FieldFullException` | class | 3 | 4 |
| `PIM` | class | 3 | 14 |
| `PIMException` | class | 3 | 11 |
| `PIMItem` | interface | 3 | 38 |
| `PIMList` | interface | 3 | 25 |
| `RepeatRule` | class | 3 | 53 |
| `ToDo` | interface | 3 | 12 |
| `ToDoList` | interface | 3 | 4 |
| `UnsupportedFieldException` | class | 3 | 4 |
| `BluetoothConnectionException` | class | 3 | 9 |
| `BluetoothStateException` | class | 3 | 2 |
| `DataElement` | class | 3 | 29 |
| `DeviceClass` | class | 3 | 4 |
| `DiscoveryAgent` | class | 3 | 11 |
| `DiscoveryListener` | interface | 3 | 12 |
| `L2CAPConnection` | interface | 3 | 7 |
| `L2CAPConnectionNotifier` | interface | 3 | 1 |
| `LocalDevice` | class | 3 | 11 |
| `RemoteDevice` | class | 3 | 13 |
| `ServiceRecord` | interface | 3 | 10 |
| `ServiceRegistrationException` | class | 3 | 2 |
| `UUID` | class | 3 | 5 |
| `Authenticator` | interface | 3 | 2 |
| `ClientSession` | interface | 3 | 10 |
| `HeaderSet` | interface | 3 | 17 |
| `Operation` | interface | 3 | 4 |
| `PasswordAuthentication` | class | 3 | 3 |
| `ResponseCodes` | class | 3 | 37 |
| `ServerRequestHandler` | class | 3 | 11 |
| `SessionNotifier` | interface | 3 | 2 |
| `AddressInfo` | class | 3 | 20 |
| `Coordinates` | class | 3 | 13 |
| `Criteria` | class | 3 | 21 |
| `Landmark` | class | 3 | 9 |
| `LandmarkException` | class | 3 | 2 |
| `LandmarkStore` | class | 3 | 14 |
| `Location` | class | 3 | 19 |
| `LocationException` | class | 3 | 2 |
| `LocationListener` | interface | 3 | 2 |
| `LocationProvider` | class | 3 | 12 |
| `Orientation` | class | 3 | 6 |
| `ProximityListener` | interface | 3 | 2 |
| `QualifiedCoordinates` | class | 3 | 5 |
| `ExternalResourceHandler` | interface | 3 | 1 |
| `SVGAnimator` | class | 3 | 12 |
| `SVGEventListener` | interface | 3 | 7 |
| `SVGImage` | class | 3 | 12 |
| `ScalableGraphics` | class | 3 | 8 |
| `ScalableImage` | class | 3 | 8 |
| `DOMException` | class | 3 | 12 |
| `Document` | interface | 3 | 3 |
| `Element` | interface | 3 | 0 |
| `org.w3c.dom.Node` | interface | 3 | 6 |
| `org.w3c.dom.events.Event` | interface | 3 | 2 |
| `EventListener` | interface | 3 | 1 |
| `EventTarget` | interface | 3 | 2 |
| `SVGAnimationElement` | interface | 3 | 2 |
| `SVGElement` | interface | 3 | 18 |
| `SVGException` | class | 3 | 4 |
| `SVGLocatableElement` | interface | 3 | 3 |
| `SVGMatrix` | interface | 3 | 6 |
| `SVGPath` | interface | 3 | 13 |
| `SVGPoint` | interface | 3 | 4 |
| `SVGRGBColor` | interface | 3 | 3 |
| `SVGRect` | interface | 3 | 8 |
| `SVGSVGElement` | interface | 3 | 11 |

## JSR-184: Mobile 3D Graphics (M3G)

M3G (JSR-184) is the standard 3D API of MIDP 2.0 phones: a retained-mode
scene graph with an immediate mode beside it. Its `.m3g` file format is defined in the
javadoc's [file format page](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/file-format.html).
J2ME Loader implements it through a native library.

### AnimationController

`public class javax.microedition.m3g.AnimationController extends javax.microedition.m3g.Object3D`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/AnimationController.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `()` | `<init>()V` |  |
| 3 | `void setActiveInterval(int start, int end)` | `setActiveInterval(II)V` |  |
| 3 | `int getActiveIntervalStart()` | `getActiveIntervalStart()I` |  |
| 3 | `int getActiveIntervalEnd()` | `getActiveIntervalEnd()I` |  |
| 3 | `void setSpeed(float speed, int worldTime)` | `setSpeed(FI)V` |  |
| 3 | `float getSpeed()` | `getSpeed()F` |  |
| 3 | `void setPosition(float sequenceTime, int worldTime)` | `setPosition(FI)V` |  |
| 3 | `float getPosition(int worldTime)` | `getPosition(I)F` |  |
| 3 | `int getRefWorldTime()` | `getRefWorldTime()I` |  |
| 3 | `void setWeight(float weight)` | `setWeight(F)V` |  |
| 3 | `float getWeight()` | `getWeight()F` |  |

### AnimationTrack

`public class javax.microedition.m3g.AnimationTrack extends javax.microedition.m3g.Object3D`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/AnimationTrack.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `ALPHA` = `256` (`0x100`) | `I` | static |
| 3 | `AMBIENT_COLOR` = `257` | `I` | static |
| 3 | `COLOR` = `258` | `I` | static |
| 3 | `CROP` = `259` | `I` | static |
| 3 | `DENSITY` = `260` | `I` | static |
| 3 | `DIFFUSE_COLOR` = `261` | `I` | static |
| 3 | `EMISSIVE_COLOR` = `262` | `I` | static |
| 3 | `FAR_DISTANCE` = `263` | `I` | static |
| 3 | `FIELD_OF_VIEW` = `264` | `I` | static |
| 3 | `INTENSITY` = `265` | `I` | static |
| 3 | `MORPH_WEIGHTS` = `266` | `I` | static |
| 3 | `NEAR_DISTANCE` = `267` | `I` | static |
| 3 | `ORIENTATION` = `268` | `I` | static |
| 3 | `PICKABILITY` = `269` | `I` | static |
| 3 | `SCALE` = `270` | `I` | static |
| 3 | `SHININESS` = `271` | `I` | static |
| 3 | `SPECULAR_COLOR` = `272` | `I` | static |
| 3 | `SPOT_ANGLE` = `273` | `I` | static |
| 3 | `SPOT_EXPONENT` = `274` | `I` | static |
| 3 | `TRANSLATION` = `275` | `I` | static |
| 3 | `VISIBILITY` = `276` | `I` | static |
| 3 | `(KeyframeSequence sequence, int property)` | `<init>(Ljavax/microedition/m3g/KeyframeSequence;I)V` |  |
| 3 | `void setController(AnimationController controller)` | `setController(Ljavax/microedition/m3g/AnimationController;)V` |  |
| 3 | `AnimationController getController()` | `getController()Ljavax/microedition/m3g/AnimationController;` |  |
| 3 | `KeyframeSequence getKeyframeSequence()` | `getKeyframeSequence()Ljavax/microedition/m3g/KeyframeSequence;` |  |
| 3 | `int getTargetProperty()` | `getTargetProperty()I` |  |

### Appearance

`public class javax.microedition.m3g.Appearance extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Appearance.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `void setLayer(int layer)` | `setLayer(I)V` |  |
| 2 | `int getLayer()` | `getLayer()I` |  |
| 2 | `void setFog(Fog fog)` | `setFog(Ljavax/microedition/m3g/Fog;)V` |  |
| 2 | `Fog getFog()` | `getFog()Ljavax/microedition/m3g/Fog;` |  |
| 2 | `void setPolygonMode(PolygonMode polygonMode)` | `setPolygonMode(Ljavax/microedition/m3g/PolygonMode;)V` |  |
| 2 | `PolygonMode getPolygonMode()` | `getPolygonMode()Ljavax/microedition/m3g/PolygonMode;` |  |
| 2 | `void setCompositingMode(CompositingMode compositingMode)` | `setCompositingMode(Ljavax/microedition/m3g/CompositingMode;)V` |  |
| 2 | `CompositingMode getCompositingMode()` | `getCompositingMode()Ljavax/microedition/m3g/CompositingMode;` |  |
| 2 | `void setTexture(int index, Texture2D texture)` | `setTexture(ILjavax/microedition/m3g/Texture2D;)V` |  |
| 2 | `Texture2D getTexture(int index)` | `getTexture(I)Ljavax/microedition/m3g/Texture2D;` |  |
| 2 | `void setMaterial(Material material)` | `setMaterial(Ljavax/microedition/m3g/Material;)V` |  |
| 2 | `Material getMaterial()` | `getMaterial()Ljavax/microedition/m3g/Material;` |  |

### Background

`public class javax.microedition.m3g.Background extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Background.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `BORDER` = `32` | `I` | static |
| 2 | `REPEAT` = `33` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setColorClearEnable(boolean enable)` | `setColorClearEnable(Z)V` |  |
| 2 | `boolean isColorClearEnabled()` | `isColorClearEnabled()Z` |  |
| 2 | `void setDepthClearEnable(boolean enable)` | `setDepthClearEnable(Z)V` |  |
| 2 | `boolean isDepthClearEnabled()` | `isDepthClearEnabled()Z` |  |
| 2 | `void setColor(int ARGB)` | `setColor(I)V` |  |
| 2 | `int getColor()` | `getColor()I` |  |
| 2 | `void setImage(Image2D image)` | `setImage(Ljavax/microedition/m3g/Image2D;)V` |  |
| 2 | `Image2D getImage()` | `getImage()Ljavax/microedition/m3g/Image2D;` |  |
| 2 | `void setImageMode(int modeX, int modeY)` | `setImageMode(II)V` |  |
| 2 | `int getImageModeX()` | `getImageModeX()I` |  |
| 2 | `int getImageModeY()` | `getImageModeY()I` |  |
| 2 | `void setCrop(int cropX, int cropY, int width, int height)` | `setCrop(IIII)V` |  |
| 2 | `int getCropX()` | `getCropX()I` |  |
| 2 | `int getCropY()` | `getCropY()I` |  |
| 2 | `int getCropWidth()` | `getCropWidth()I` |  |
| 2 | `int getCropHeight()` | `getCropHeight()I` |  |

### Camera

`public class javax.microedition.m3g.Camera extends javax.microedition.m3g.Node`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Camera.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `GENERIC` = `48` | `I` | static |
| 2 | `PARALLEL` = `49` | `I` | static |
| 2 | `PERSPECTIVE` = `50` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setParallel(float fovy, float aspectRatio, float near, float far)` | `setParallel(FFFF)V` |  |
| 2 | `void setPerspective(float fovy, float aspectRatio, float near, float far)` | `setPerspective(FFFF)V` |  |
| 2 | `void setGeneric(Transform transform)` | `setGeneric(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `int getProjection(Transform transform)` | `getProjection(Ljavax/microedition/m3g/Transform;)I` |  |
| 2 | `int getProjection(float[] params)` | `getProjection([F)I` |  |

### CompositingMode

`public class javax.microedition.m3g.CompositingMode extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/CompositingMode.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `ALPHA` = `64` | `I` | static |
| 2 | `ALPHA_ADD` = `65` | `I` | static |
| 2 | `MODULATE` = `66` | `I` | static |
| 2 | `MODULATE_X2` = `67` | `I` | static |
| 2 | `REPLACE` = `68` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setBlending(int mode)` | `setBlending(I)V` |  |
| 2 | `int getBlending()` | `getBlending()I` |  |
| 2 | `void setAlphaThreshold(float threshold)` | `setAlphaThreshold(F)V` |  |
| 2 | `float getAlphaThreshold()` | `getAlphaThreshold()F` |  |
| 2 | `void setAlphaWriteEnable(boolean enable)` | `setAlphaWriteEnable(Z)V` |  |
| 2 | `boolean isAlphaWriteEnabled()` | `isAlphaWriteEnabled()Z` |  |
| 2 | `void setColorWriteEnable(boolean enable)` | `setColorWriteEnable(Z)V` |  |
| 2 | `boolean isColorWriteEnabled()` | `isColorWriteEnabled()Z` |  |
| 2 | `void setDepthWriteEnable(boolean enable)` | `setDepthWriteEnable(Z)V` |  |
| 2 | `boolean isDepthWriteEnabled()` | `isDepthWriteEnabled()Z` |  |
| 2 | `void setDepthTestEnable(boolean enable)` | `setDepthTestEnable(Z)V` |  |
| 2 | `boolean isDepthTestEnabled()` | `isDepthTestEnabled()Z` |  |
| 2 | `void setDepthOffset(float factor, float units)` | `setDepthOffset(FF)V` |  |
| 2 | `float getDepthOffsetFactor()` | `getDepthOffsetFactor()F` |  |
| 2 | `float getDepthOffsetUnits()` | `getDepthOffsetUnits()F` |  |

### Fog

`public class javax.microedition.m3g.Fog extends javax.microedition.m3g.Object3D`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Fog.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `EXPONENTIAL` = `80` | `I` | static |
| 3 | `LINEAR` = `81` | `I` | static |
| 3 | `()` | `<init>()V` |  |
| 3 | `void setMode(int mode)` | `setMode(I)V` |  |
| 3 | `int getMode()` | `getMode()I` |  |
| 3 | `void setLinear(float near, float far)` | `setLinear(FF)V` |  |
| 3 | `float getNearDistance()` | `getNearDistance()F` |  |
| 3 | `float getFarDistance()` | `getFarDistance()F` |  |
| 3 | `void setDensity(float density)` | `setDensity(F)V` |  |
| 3 | `float getDensity()` | `getDensity()F` |  |
| 3 | `void setColor(int RGB)` | `setColor(I)V` |  |
| 3 | `int getColor()` | `getColor()I` |  |

### javax.microedition.m3g.Graphics3D

`public class javax.microedition.m3g.Graphics3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Graphics3D.html).
The renderer: `getInstance()`, `bindTarget(g)` on a MIDP `Graphics`, `render(world)` or `render(vertices, indices, appearance, transform)`, `releaseTarget()`.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `ANTIALIAS` = `2` | `I` | static |
| 2 | `DITHER` = `4` | `I` | static |
| 2 | `TRUE_COLOR` = `8` | `I` | static |
| 2 | `OVERWRITE` = `16` | `I` | static |
| 2 | `Graphics3D getInstance()` | `getInstance()Ljavax/microedition/m3g/Graphics3D;` | static final |
| 2 | `void bindTarget(Object target)` | `bindTarget(Ljava/lang/Object;)V` |  |
| 2 | `void bindTarget(Object target, boolean depthBuffer, int hints)` | `bindTarget(Ljava/lang/Object;ZI)V` |  |
| 2 | `void releaseTarget()` | `releaseTarget()V` |  |
| 2 | `Object getTarget()` | `getTarget()Ljava/lang/Object;` |  |
| 2 | `int getHints()` | `getHints()I` |  |
| 2 | `boolean isDepthBufferEnabled()` | `isDepthBufferEnabled()Z` |  |
| 2 | `void setViewport(int x, int y, int width, int height)` | `setViewport(IIII)V` |  |
| 2 | `int getViewportX()` | `getViewportX()I` |  |
| 2 | `int getViewportY()` | `getViewportY()I` |  |
| 2 | `int getViewportWidth()` | `getViewportWidth()I` |  |
| 2 | `int getViewportHeight()` | `getViewportHeight()I` |  |
| 2 | `void setDepthRange(float near, float far)` | `setDepthRange(FF)V` |  |
| 2 | `float getDepthRangeNear()` | `getDepthRangeNear()F` |  |
| 2 | `float getDepthRangeFar()` | `getDepthRangeFar()F` |  |
| 2 | `void clear(Background background)` | `clear(Ljavax/microedition/m3g/Background;)V` |  |
| 2 | `void render(World world)` | `render(Ljavax/microedition/m3g/World;)V` |  |
| 2 | `void render(Node node, Transform transform)` | `render(Ljavax/microedition/m3g/Node;Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void render(VertexBuffer vertices, IndexBuffer triangles, Appearance appearance, Transform transform, int scope)` | `render(Ljavax/microedition/m3g/VertexBuffer;Ljavax/microedition/m3g/IndexBuffer;Ljavax/microedition/m3g/Appearance;Ljavax/microedition/m3g/Transform;I)V` |  |
| 2 | `void render(VertexBuffer vertices, IndexBuffer triangles, Appearance appearance, Transform transform)` | `render(Ljavax/microedition/m3g/VertexBuffer;Ljavax/microedition/m3g/IndexBuffer;Ljavax/microedition/m3g/Appearance;Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void setCamera(Camera camera, Transform transform)` | `setCamera(Ljavax/microedition/m3g/Camera;Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `Camera getCamera(Transform transform)` | `getCamera(Ljavax/microedition/m3g/Transform;)Ljavax/microedition/m3g/Camera;` |  |
| 2 | `int addLight(Light light, Transform transform)` | `addLight(Ljavax/microedition/m3g/Light;Ljavax/microedition/m3g/Transform;)I` |  |
| 2 | `void setLight(int index, Light light, Transform transform)` | `setLight(ILjavax/microedition/m3g/Light;Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void resetLights()` | `resetLights()V` |  |
| 2 | `int getLightCount()` | `getLightCount()I` |  |
| 2 | `Light getLight(int index, Transform transform)` | `getLight(ILjavax/microedition/m3g/Transform;)Ljavax/microedition/m3g/Light;` |  |
| 2 | `Hashtable getProperties()` | `getProperties()Ljava/util/Hashtable;` | static final |

### Group

`public class javax.microedition.m3g.Group extends javax.microedition.m3g.Node`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Group.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `void addChild(Node child)` | `addChild(Ljavax/microedition/m3g/Node;)V` |  |
| 2 | `void removeChild(Node child)` | `removeChild(Ljavax/microedition/m3g/Node;)V` |  |
| 2 | `int getChildCount()` | `getChildCount()I` |  |
| 2 | `Node getChild(int index)` | `getChild(I)Ljavax/microedition/m3g/Node;` |  |
| 2 | `boolean pick(int scope, float ox, float oy, float oz, float dx, float dy, float dz, RayIntersection ri)` | `pick(IFFFFFFLjavax/microedition/m3g/RayIntersection;)Z` |  |
| 2 | `boolean pick(int scope, float x, float y, Camera camera, RayIntersection ri)` | `pick(IFFLjavax/microedition/m3g/Camera;Ljavax/microedition/m3g/RayIntersection;)Z` |  |

### Image2D

`public class javax.microedition.m3g.Image2D extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Image2D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `ALPHA` = `96` | `I` | static |
| 2 | `LUMINANCE` = `97` | `I` | static |
| 2 | `LUMINANCE_ALPHA` = `98` | `I` | static |
| 2 | `RGB` = `99` | `I` | static |
| 2 | `RGBA` = `100` | `I` | static |
| 2 | `(int format, Object image)` | `<init>(ILjava/lang/Object;)V` |  |
| 2 | `(int format, int width, int height, byte[] image)` | `<init>(III[B)V` |  |
| 2 | `(int format, int width, int height, byte[] image, byte[] palette)` | `<init>(III[B[B)V` |  |
| 2 | `(int format, int width, int height)` | `<init>(III)V` |  |
| 2 | `void set(int x, int y, int width, int height, byte[] image)` | `set(IIII[B)V` |  |
| 2 | `boolean isMutable()` | `isMutable()Z` |  |
| 2 | `int getFormat()` | `getFormat()I` |  |
| 2 | `int getWidth()` | `getWidth()I` |  |
| 2 | `int getHeight()` | `getHeight()I` |  |

### IndexBuffer

`public abstract class javax.microedition.m3g.IndexBuffer extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/IndexBuffer.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `int getIndexCount()` | `getIndexCount()I` |  |
| 2 | `void getIndices(int[] indices)` | `getIndices([I)V` |  |

### KeyframeSequence

`public class javax.microedition.m3g.KeyframeSequence extends javax.microedition.m3g.Object3D`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/KeyframeSequence.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `LINEAR` = `176` | `I` | static |
| 3 | `SLERP` = `177` | `I` | static |
| 3 | `SPLINE` = `178` | `I` | static |
| 3 | `SQUAD` = `179` | `I` | static |
| 3 | `STEP` = `180` | `I` | static |
| 3 | `CONSTANT` = `192` | `I` | static |
| 3 | `LOOP` = `193` | `I` | static |
| 3 | `(int numKeyframes, int numComponents, int interpolation)` | `<init>(III)V` |  |
| 3 | `int getComponentCount()` | `getComponentCount()I` |  |
| 3 | `int getKeyframeCount()` | `getKeyframeCount()I` |  |
| 3 | `int getInterpolationType()` | `getInterpolationType()I` |  |
| 3 | `void setKeyframe(int index, int time, float[] value)` | `setKeyframe(II[F)V` |  |
| 3 | `int getKeyframe(int index, float[] value)` | `getKeyframe(I[F)I` |  |
| 3 | `void setValidRange(int first, int last)` | `setValidRange(II)V` |  |
| 3 | `int getValidRangeFirst()` | `getValidRangeFirst()I` |  |
| 3 | `int getValidRangeLast()` | `getValidRangeLast()I` |  |
| 3 | `void setDuration(int duration)` | `setDuration(I)V` |  |
| 3 | `int getDuration()` | `getDuration()I` |  |
| 3 | `void setRepeatMode(int mode)` | `setRepeatMode(I)V` |  |
| 3 | `int getRepeatMode()` | `getRepeatMode()I` |  |

### javax.microedition.m3g.Light

`public class javax.microedition.m3g.Light extends javax.microedition.m3g.Node`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Light.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `AMBIENT` = `128` | `I` | static |
| 2 | `DIRECTIONAL` = `129` | `I` | static |
| 2 | `OMNI` = `130` | `I` | static |
| 2 | `SPOT` = `131` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setMode(int mode)` | `setMode(I)V` |  |
| 2 | `int getMode()` | `getMode()I` |  |
| 2 | `void setIntensity(float intensity)` | `setIntensity(F)V` |  |
| 2 | `float getIntensity()` | `getIntensity()F` |  |
| 2 | `void setColor(int RGB)` | `setColor(I)V` |  |
| 2 | `int getColor()` | `getColor()I` |  |
| 2 | `void setSpotAngle(float angle)` | `setSpotAngle(F)V` |  |
| 2 | `float getSpotAngle()` | `getSpotAngle()F` |  |
| 2 | `void setSpotExponent(float exponent)` | `setSpotExponent(F)V` |  |
| 2 | `float getSpotExponent()` | `getSpotExponent()F` |  |
| 2 | `void setAttenuation(float constant, float linear, float quadratic)` | `setAttenuation(FFF)V` |  |
| 2 | `float getConstantAttenuation()` | `getConstantAttenuation()F` |  |
| 2 | `float getLinearAttenuation()` | `getLinearAttenuation()F` |  |
| 2 | `float getQuadraticAttenuation()` | `getQuadraticAttenuation()F` |  |

### Loader

`public class javax.microedition.m3g.Loader`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Loader.html).
Loads `.m3g` files (and PNG as `Image2D`) from the JAR or a URL; most M3G games build their scenes this way.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `Object3D[] load(String name)` | `load(Ljava/lang/String;)[Ljavax/microedition/m3g/Object3D;` | static |
| 2 | `Object3D[] load(byte[] data, int offset)` | `load([BI)[Ljavax/microedition/m3g/Object3D;` | static |

### Material

`public class javax.microedition.m3g.Material extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Material.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `AMBIENT` = `1024` (`0x400`) | `I` | static |
| 2 | `DIFFUSE` = `2048` (`0x800`) | `I` | static |
| 2 | `EMISSIVE` = `4096` (`0x1000`) | `I` | static |
| 2 | `SPECULAR` = `8192` (`0x2000`) | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setColor(int target, int ARGB)` | `setColor(II)V` |  |
| 2 | `int getColor(int target)` | `getColor(I)I` |  |
| 2 | `void setShininess(float shininess)` | `setShininess(F)V` |  |
| 2 | `float getShininess()` | `getShininess()F` |  |
| 2 | `void setVertexColorTrackingEnable(boolean enable)` | `setVertexColorTrackingEnable(Z)V` |  |
| 2 | `boolean isVertexColorTrackingEnabled()` | `isVertexColorTrackingEnabled()Z` |  |

### Mesh

`public class javax.microedition.m3g.Mesh extends javax.microedition.m3g.Node`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Mesh.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(VertexBuffer vertices, IndexBuffer submesh, Appearance appearance)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;Ljavax/microedition/m3g/IndexBuffer;Ljavax/microedition/m3g/Appearance;)V` |  |
| 2 | `(VertexBuffer vertices, IndexBuffer[] submeshes, Appearance[] appearances)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;[Ljavax/microedition/m3g/IndexBuffer;[Ljavax/microedition/m3g/Appearance;)V` |  |
| 2 | `void setAppearance(int index, Appearance appearance)` | `setAppearance(ILjavax/microedition/m3g/Appearance;)V` |  |
| 2 | `Appearance getAppearance(int index)` | `getAppearance(I)Ljavax/microedition/m3g/Appearance;` |  |
| 2 | `IndexBuffer getIndexBuffer(int index)` | `getIndexBuffer(I)Ljavax/microedition/m3g/IndexBuffer;` |  |
| 2 | `VertexBuffer getVertexBuffer()` | `getVertexBuffer()Ljavax/microedition/m3g/VertexBuffer;` |  |
| 2 | `int getSubmeshCount()` | `getSubmeshCount()I` |  |

### MorphingMesh

`public class javax.microedition.m3g.MorphingMesh extends javax.microedition.m3g.Mesh`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/MorphingMesh.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `(VertexBuffer base, VertexBuffer[] targets, IndexBuffer submesh, Appearance appearance)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;[Ljavax/microedition/m3g/VertexBuffer;Ljavax/microedition/m3g/IndexBuffer;Ljavax/microedition/m3g/Appearance;)V` |  |
| 3 | `(VertexBuffer base, VertexBuffer[] targets, IndexBuffer[] submeshes, Appearance[] appearances)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;[Ljavax/microedition/m3g/VertexBuffer;[Ljavax/microedition/m3g/IndexBuffer;[Ljavax/microedition/m3g/Appearance;)V` |  |
| 3 | `VertexBuffer getMorphTarget(int index)` | `getMorphTarget(I)Ljavax/microedition/m3g/VertexBuffer;` |  |
| 3 | `int getMorphTargetCount()` | `getMorphTargetCount()I` |  |
| 3 | `void setWeights(float[] weights)` | `setWeights([F)V` |  |
| 3 | `void getWeights(float[] weights)` | `getWeights([F)V` |  |

### javax.microedition.m3g.Node

`public abstract class javax.microedition.m3g.Node extends javax.microedition.m3g.Transformable`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Node.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `NONE` = `144` | `I` | static |
| 2 | `ORIGIN` = `145` | `I` | static |
| 2 | `X_AXIS` = `146` | `I` | static |
| 2 | `Y_AXIS` = `147` | `I` | static |
| 2 | `Z_AXIS` = `148` | `I` | static |
| 2 | `void setRenderingEnable(boolean enable)` | `setRenderingEnable(Z)V` |  |
| 2 | `void setPickingEnable(boolean enable)` | `setPickingEnable(Z)V` |  |
| 2 | `void setScope(int scope)` | `setScope(I)V` |  |
| 2 | `void setAlphaFactor(float alphaFactor)` | `setAlphaFactor(F)V` |  |
| 2 | `boolean isRenderingEnabled()` | `isRenderingEnabled()Z` |  |
| 2 | `boolean isPickingEnabled()` | `isPickingEnabled()Z` |  |
| 2 | `int getScope()` | `getScope()I` |  |
| 2 | `float getAlphaFactor()` | `getAlphaFactor()F` |  |
| 2 | `Node getParent()` | `getParent()Ljavax/microedition/m3g/Node;` |  |
| 2 | `boolean getTransformTo(Node target, Transform transform)` | `getTransformTo(Ljavax/microedition/m3g/Node;Ljavax/microedition/m3g/Transform;)Z` |  |
| 2 | `void align(Node reference)` | `align(Ljavax/microedition/m3g/Node;)V` | final |
| 2 | `void setAlignment(Node zRef, int zTarget, Node yRef, int yTarget)` | `setAlignment(Ljavax/microedition/m3g/Node;ILjavax/microedition/m3g/Node;I)V` |  |
| 2 | `int getAlignmentTarget(int axis)` | `getAlignmentTarget(I)I` |  |
| 2 | `Node getAlignmentReference(int axis)` | `getAlignmentReference(I)Ljavax/microedition/m3g/Node;` |  |

### Object3D

`public abstract class javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Object3D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `int animate(int time)` | `animate(I)I` | final |
| 2 | `Object3D duplicate()` | `duplicate()Ljavax/microedition/m3g/Object3D;` | final |
| 2 | `Object3D find(int userID)` | `find(I)Ljavax/microedition/m3g/Object3D;` |  |
| 2 | `int getReferences(Object3D[] references)` | `getReferences([Ljavax/microedition/m3g/Object3D;)I` |  |
| 2 | `void setUserID(int userID)` | `setUserID(I)V` |  |
| 2 | `int getUserID()` | `getUserID()I` |  |
| 2 | `void setUserObject(Object userObject)` | `setUserObject(Ljava/lang/Object;)V` |  |
| 2 | `Object getUserObject()` | `getUserObject()Ljava/lang/Object;` |  |
| 2 | `void addAnimationTrack(AnimationTrack animationTrack)` | `addAnimationTrack(Ljavax/microedition/m3g/AnimationTrack;)V` |  |
| 2 | `AnimationTrack getAnimationTrack(int index)` | `getAnimationTrack(I)Ljavax/microedition/m3g/AnimationTrack;` |  |
| 2 | `void removeAnimationTrack(AnimationTrack animationTrack)` | `removeAnimationTrack(Ljavax/microedition/m3g/AnimationTrack;)V` |  |
| 2 | `int getAnimationTrackCount()` | `getAnimationTrackCount()I` |  |

### PolygonMode

`public class javax.microedition.m3g.PolygonMode extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/PolygonMode.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `CULL_BACK` = `160` | `I` | static |
| 2 | `CULL_FRONT` = `161` | `I` | static |
| 2 | `CULL_NONE` = `162` | `I` | static |
| 2 | `SHADE_FLAT` = `164` | `I` | static |
| 2 | `SHADE_SMOOTH` = `165` | `I` | static |
| 2 | `WINDING_CCW` = `168` | `I` | static |
| 2 | `WINDING_CW` = `169` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void setCulling(int mode)` | `setCulling(I)V` |  |
| 2 | `int getCulling()` | `getCulling()I` |  |
| 2 | `void setWinding(int mode)` | `setWinding(I)V` |  |
| 2 | `int getWinding()` | `getWinding()I` |  |
| 2 | `void setShading(int mode)` | `setShading(I)V` |  |
| 2 | `int getShading()` | `getShading()I` |  |
| 2 | `void setTwoSidedLightingEnable(boolean enable)` | `setTwoSidedLightingEnable(Z)V` |  |
| 2 | `boolean isTwoSidedLightingEnabled()` | `isTwoSidedLightingEnabled()Z` |  |
| 2 | `void setLocalCameraLightingEnable(boolean enable)` | `setLocalCameraLightingEnable(Z)V` |  |
| 2 | `boolean isLocalCameraLightingEnabled()` | `isLocalCameraLightingEnabled()Z` |  |
| 2 | `void setPerspectiveCorrectionEnable(boolean enable)` | `setPerspectiveCorrectionEnable(Z)V` |  |
| 2 | `boolean isPerspectiveCorrectionEnabled()` | `isPerspectiveCorrectionEnabled()Z` |  |

### RayIntersection

`public class javax.microedition.m3g.RayIntersection`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/RayIntersection.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `()` | `<init>()V` |  |
| 3 | `Node getIntersected()` | `getIntersected()Ljavax/microedition/m3g/Node;` |  |
| 3 | `void getRay(float[] ray)` | `getRay([F)V` |  |
| 3 | `float getDistance()` | `getDistance()F` |  |
| 3 | `int getSubmeshIndex()` | `getSubmeshIndex()I` |  |
| 3 | `float getTextureS(int index)` | `getTextureS(I)F` |  |
| 3 | `float getTextureT(int index)` | `getTextureT(I)F` |  |
| 3 | `float getNormalX()` | `getNormalX()F` |  |
| 3 | `float getNormalY()` | `getNormalY()F` |  |
| 3 | `float getNormalZ()` | `getNormalZ()F` |  |

### SkinnedMesh

`public class javax.microedition.m3g.SkinnedMesh extends javax.microedition.m3g.Mesh`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/SkinnedMesh.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `(VertexBuffer vertices, IndexBuffer submesh, Appearance appearance, Group skeleton)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;Ljavax/microedition/m3g/IndexBuffer;Ljavax/microedition/m3g/Appearance;Ljavax/microedition/m3g/Group;)V` |  |
| 3 | `(VertexBuffer vertices, IndexBuffer[] submeshes, Appearance[] appearances, Group skeleton)` | `<init>(Ljavax/microedition/m3g/VertexBuffer;[Ljavax/microedition/m3g/IndexBuffer;[Ljavax/microedition/m3g/Appearance;Ljavax/microedition/m3g/Group;)V` |  |
| 3 | `Group getSkeleton()` | `getSkeleton()Ljavax/microedition/m3g/Group;` |  |
| 3 | `void addTransform(Node bone, int weight, int firstVertex, int numVertices)` | `addTransform(Ljavax/microedition/m3g/Node;III)V` |  |
| 3 | `void getBoneTransform(Node bone, Transform transform)` | `getBoneTransform(Ljavax/microedition/m3g/Node;Ljavax/microedition/m3g/Transform;)V` |  |
| 3 | `int getBoneVertices(Node bone, int[] indices, float[] weights)` | `getBoneVertices(Ljavax/microedition/m3g/Node;[I[F)I` |  |

### Sprite3D

`public class javax.microedition.m3g.Sprite3D extends javax.microedition.m3g.Node`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Sprite3D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(boolean scaled, Image2D image, Appearance appearance)` | `<init>(ZLjavax/microedition/m3g/Image2D;Ljavax/microedition/m3g/Appearance;)V` |  |
| 2 | `boolean isScaled()` | `isScaled()Z` |  |
| 2 | `void setAppearance(Appearance appearance)` | `setAppearance(Ljavax/microedition/m3g/Appearance;)V` |  |
| 2 | `Appearance getAppearance()` | `getAppearance()Ljavax/microedition/m3g/Appearance;` |  |
| 2 | `void setImage(Image2D image)` | `setImage(Ljavax/microedition/m3g/Image2D;)V` |  |
| 2 | `Image2D getImage()` | `getImage()Ljavax/microedition/m3g/Image2D;` |  |
| 2 | `void setCrop(int cropX, int cropY, int width, int height)` | `setCrop(IIII)V` |  |
| 2 | `int getCropX()` | `getCropX()I` |  |
| 2 | `int getCropY()` | `getCropY()I` |  |
| 2 | `int getCropWidth()` | `getCropWidth()I` |  |
| 2 | `int getCropHeight()` | `getCropHeight()I` |  |

### Texture2D

`public class javax.microedition.m3g.Texture2D extends javax.microedition.m3g.Transformable`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Texture2D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `FILTER_BASE_LEVEL` = `208` | `I` | static |
| 2 | `FILTER_LINEAR` = `209` | `I` | static |
| 2 | `FILTER_NEAREST` = `210` | `I` | static |
| 2 | `FUNC_ADD` = `224` | `I` | static |
| 2 | `FUNC_BLEND` = `225` | `I` | static |
| 2 | `FUNC_DECAL` = `226` | `I` | static |
| 2 | `FUNC_MODULATE` = `227` | `I` | static |
| 2 | `FUNC_REPLACE` = `228` | `I` | static |
| 2 | `WRAP_CLAMP` = `240` | `I` | static |
| 2 | `WRAP_REPEAT` = `241` | `I` | static |
| 2 | `(Image2D image)` | `<init>(Ljavax/microedition/m3g/Image2D;)V` |  |
| 2 | `void setImage(Image2D image)` | `setImage(Ljavax/microedition/m3g/Image2D;)V` |  |
| 2 | `Image2D getImage()` | `getImage()Ljavax/microedition/m3g/Image2D;` |  |
| 2 | `void setFiltering(int levelFilter, int imageFilter)` | `setFiltering(II)V` |  |
| 2 | `int getLevelFilter()` | `getLevelFilter()I` |  |
| 2 | `int getImageFilter()` | `getImageFilter()I` |  |
| 2 | `void setWrapping(int wrapS, int wrapT)` | `setWrapping(II)V` |  |
| 2 | `int getWrappingS()` | `getWrappingS()I` |  |
| 2 | `int getWrappingT()` | `getWrappingT()I` |  |
| 2 | `void setBlending(int func)` | `setBlending(I)V` |  |
| 2 | `int getBlending()` | `getBlending()I` |  |
| 2 | `void setBlendColor(int RGB)` | `setBlendColor(I)V` |  |
| 2 | `int getBlendColor()` | `getBlendColor()I` |  |

### Transform

`public class javax.microedition.m3g.Transform`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Transform.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `(Transform transform)` | `<init>(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void setIdentity()` | `setIdentity()V` |  |
| 2 | `void set(Transform transform)` | `set(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void set(float[] matrix)` | `set([F)V` |  |
| 2 | `void get(float[] matrix)` | `get([F)V` |  |
| 2 | `void invert()` | `invert()V` |  |
| 2 | `void transpose()` | `transpose()V` |  |
| 2 | `void postMultiply(Transform transform)` | `postMultiply(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void postScale(float sx, float sy, float sz)` | `postScale(FFF)V` |  |
| 2 | `void postRotate(float angle, float ax, float ay, float az)` | `postRotate(FFFF)V` |  |
| 2 | `void postRotateQuat(float qx, float qy, float qz, float qw)` | `postRotateQuat(FFFF)V` |  |
| 2 | `void postTranslate(float tx, float ty, float tz)` | `postTranslate(FFF)V` |  |
| 2 | `void transform(VertexArray in, float[] out, boolean W)` | `transform(Ljavax/microedition/m3g/VertexArray;[FZ)V` |  |
| 2 | `void transform(float[] vectors)` | `transform([F)V` |  |

### Transformable

`public abstract class javax.microedition.m3g.Transformable extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/Transformable.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `void setOrientation(float angle, float ax, float ay, float az)` | `setOrientation(FFFF)V` |  |
| 2 | `void preRotate(float angle, float ax, float ay, float az)` | `preRotate(FFFF)V` |  |
| 2 | `void postRotate(float angle, float ax, float ay, float az)` | `postRotate(FFFF)V` |  |
| 2 | `void getOrientation(float[] angleAxis)` | `getOrientation([F)V` |  |
| 2 | `void setScale(float sx, float sy, float sz)` | `setScale(FFF)V` |  |
| 2 | `void scale(float sx, float sy, float sz)` | `scale(FFF)V` |  |
| 2 | `void getScale(float[] xyz)` | `getScale([F)V` |  |
| 2 | `void setTranslation(float tx, float ty, float tz)` | `setTranslation(FFF)V` |  |
| 2 | `void translate(float tx, float ty, float tz)` | `translate(FFF)V` |  |
| 2 | `void getTranslation(float[] xyz)` | `getTranslation([F)V` |  |
| 2 | `void setTransform(Transform transform)` | `setTransform(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void getTransform(Transform transform)` | `getTransform(Ljavax/microedition/m3g/Transform;)V` |  |
| 2 | `void getCompositeTransform(Transform transform)` | `getCompositeTransform(Ljavax/microedition/m3g/Transform;)V` |  |

### TriangleStripArray

`public class javax.microedition.m3g.TriangleStripArray extends javax.microedition.m3g.IndexBuffer`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/TriangleStripArray.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(int firstIndex, int[] stripLengths)` | `<init>(I[I)V` |  |
| 2 | `(int[] indices, int[] stripLengths)` | `<init>([I[I)V` |  |

### VertexArray

`public class javax.microedition.m3g.VertexArray extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/VertexArray.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(int numVertices, int numComponents, int componentSize)` | `<init>(III)V` |  |
| 2 | `void set(int firstVertex, int numVertices, short[] values)` | `set(II[S)V` |  |
| 2 | `void set(int firstVertex, int numVertices, byte[] values)` | `set(II[B)V` |  |
| 2 | `int getVertexCount()` | `getVertexCount()I` |  |
| 2 | `int getComponentCount()` | `getComponentCount()I` |  |
| 2 | `int getComponentType()` | `getComponentType()I` |  |
| 2 | `void get(int firstVertex, int numVertices, short[] values)` | `get(II[S)V` |  |
| 2 | `void get(int firstVertex, int numVertices, byte[] values)` | `get(II[B)V` |  |

### VertexBuffer

`public class javax.microedition.m3g.VertexBuffer extends javax.microedition.m3g.Object3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/VertexBuffer.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `int getVertexCount()` | `getVertexCount()I` |  |
| 2 | `void setPositions(VertexArray positions, float scale, float[] bias)` | `setPositions(Ljavax/microedition/m3g/VertexArray;F[F)V` |  |
| 2 | `void setTexCoords(int index, VertexArray texCoords, float scale, float[] bias)` | `setTexCoords(ILjavax/microedition/m3g/VertexArray;F[F)V` |  |
| 2 | `void setNormals(VertexArray normals)` | `setNormals(Ljavax/microedition/m3g/VertexArray;)V` |  |
| 2 | `void setColors(VertexArray colors)` | `setColors(Ljavax/microedition/m3g/VertexArray;)V` |  |
| 2 | `VertexArray getPositions(float[] scaleBias)` | `getPositions([F)Ljavax/microedition/m3g/VertexArray;` |  |
| 2 | `VertexArray getTexCoords(int index, float[] scaleBias)` | `getTexCoords(I[F)Ljavax/microedition/m3g/VertexArray;` |  |
| 2 | `VertexArray getNormals()` | `getNormals()Ljavax/microedition/m3g/VertexArray;` |  |
| 2 | `VertexArray getColors()` | `getColors()Ljavax/microedition/m3g/VertexArray;` |  |
| 2 | `void setDefaultColor(int ARGB)` | `setDefaultColor(I)V` |  |
| 2 | `int getDefaultColor()` | `getDefaultColor()I` |  |

### World

`public class javax.microedition.m3g.World extends javax.microedition.m3g.Group`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr184/javax/microedition/m3g/World.html).
The root of a scene graph loaded from a `.m3g` file.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `void setBackground(Background background)` | `setBackground(Ljavax/microedition/m3g/Background;)V` |  |
| 2 | `Background getBackground()` | `getBackground()Ljavax/microedition/m3g/Background;` |  |
| 2 | `void setActiveCamera(Camera camera)` | `setActiveCamera(Ljavax/microedition/m3g/Camera;)V` |  |
| 2 | `Camera getActiveCamera()` | `getActiveCamera()Ljavax/microedition/m3g/Camera;` |  |

## MascotCapsule micro3D V3

MascotCapsule micro3D V3 (HI Corporation) is the other 3D engine of the
era, built into Sony Ericsson phones and others. J2ME Loader implements it
on OpenGL ES 2.0. Close relatives of the API appear under vendor package
names (`com.motorola.graphics.j3d`, `com.jblend.graphics.j3d`,
`com.vodafone.v10.graphics.j3d`); J2ME Loader declares those as
interfaces and implements them with its one `Graphics` class.

### ActionTable

`public class com.mascotcapsule.micro3d.v3.ActionTable`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/ActionTable.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(byte[] b)` | `<init>([B)V` |  |
| 2 | `(String name)` | `<init>(Ljava/lang/String;)V` |  |
| 2 | `void dispose()` | `dispose()V` | final |
| 2 | `int getNumAction()` | `getNumAction()I` | final deprecated |
| 2 | `int getNumActions()` | `getNumActions()I` | final |
| 2 | `int getNumFrame(int idx)` | `getNumFrame(I)I` | final deprecated |
| 2 | `int getNumFrames(int idx)` | `getNumFrames(I)I` | final |

### AffineTrans

`public class com.mascotcapsule.micro3d.v3.AffineTrans`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/AffineTrans.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `m00` = (not a constant) | `I` | not final |
| 2 | `m01` = (not a constant) | `I` | not final |
| 2 | `m02` = (not a constant) | `I` | not final |
| 2 | `m03` = (not a constant) | `I` | not final |
| 2 | `m10` = (not a constant) | `I` | not final |
| 2 | `m11` = (not a constant) | `I` | not final |
| 2 | `m12` = (not a constant) | `I` | not final |
| 2 | `m13` = (not a constant) | `I` | not final |
| 2 | `m20` = (not a constant) | `I` | not final |
| 2 | `m21` = (not a constant) | `I` | not final |
| 2 | `m22` = (not a constant) | `I` | not final |
| 2 | `m23` = (not a constant) | `I` | not final |
| 2 | `()` | `<init>()V` |  |
| 2 | `(int m00, int m01, int m02, int m03, int m10, int m11, int m12, int m13, int m20, int m21, int m22, int m23)` | `<init>(IIIIIIIIIIII)V` |  |
| 2 | `(AffineTrans a)` | `<init>(Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` |  |
| 2 | `(int[][] a)` | `<init>([[I)V` |  |
| 2 | `(int[] a)` | `<init>([I)V` |  |
| 2 | `(int[] a, int offset)` | `<init>([II)V` |  |
| 2 | `void setRotationX(int r)` | `setRotationX(I)V` | final deprecated |
| 2 | `void setRotationY(int r)` | `setRotationY(I)V` | final deprecated |
| 2 | `void setRotationZ(int r)` | `setRotationZ(I)V` | final deprecated |
| 2 | `void setIdentity()` | `setIdentity()V` | final |
| 2 | `void get(int[] a)` | `get([I)V` | final |
| 2 | `void get(int[] a, int offset)` | `get([II)V` | final |
| 2 | `void set(int[] a, int offset)` | `set([II)V` | final |
| 2 | `void set(int m00, int m01, int m02, int m03, int m10, int m11, int m12, int m13, int m20, int m21, int m22, int m23)` | `set(IIIIIIIIIIII)V` | final |
| 2 | `void set(AffineTrans a)` | `set(Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final |
| 2 | `void set(int[][] a)` | `set([[I)V` | final |
| 2 | `void set(int[] a)` | `set([I)V` | final |
| 2 | `Vector3D transPoint(Vector3D v)` | `transPoint(Lcom/mascotcapsule/micro3d/v3/Vector3D;)Lcom/mascotcapsule/micro3d/v3/Vector3D;` | final deprecated |
| 2 | `Vector3D transform(Vector3D v)` | `transform(Lcom/mascotcapsule/micro3d/v3/Vector3D;)Lcom/mascotcapsule/micro3d/v3/Vector3D;` | final |
| 2 | `void rotationX(int r)` | `rotationX(I)V` | final |
| 2 | `void rotationY(int r)` | `rotationY(I)V` | final |
| 2 | `void rotationZ(int r)` | `rotationZ(I)V` | final |
| 2 | `void multiply(AffineTrans a)` | `multiply(Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final deprecated |
| 2 | `void mul(AffineTrans a)` | `mul(Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final |
| 2 | `void multiply(AffineTrans a1, AffineTrans a2)` | `multiply(Lcom/mascotcapsule/micro3d/v3/AffineTrans;Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final deprecated |
| 2 | `void mul(AffineTrans a1, AffineTrans a2)` | `mul(Lcom/mascotcapsule/micro3d/v3/AffineTrans;Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final |
| 2 | `void rotationV(Vector3D v, int r)` | `rotationV(Lcom/mascotcapsule/micro3d/v3/Vector3D;I)V` | final deprecated |
| 2 | `void setRotation(Vector3D v, int r)` | `setRotation(Lcom/mascotcapsule/micro3d/v3/Vector3D;I)V` | final |
| 2 | `void setViewTrans(Vector3D pos, Vector3D look, Vector3D up)` | `setViewTrans(Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final deprecated |
| 2 | `void lookAt(Vector3D pos, Vector3D look, Vector3D up)` | `lookAt(Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final |

### Effect3D

`public class com.mascotcapsule.micro3d.v3.Effect3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Effect3D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `NORMAL_SHADING` = `0` | `I` | static |
| 2 | `TOON_SHADING` = `1` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `(Light light, int shading, boolean isEnableTrans, Texture tex)` | `<init>(Lcom/mascotcapsule/micro3d/v3/Light;IZLcom/mascotcapsule/micro3d/v3/Texture;)V` |  |
| 2 | `Light getLight()` | `getLight()Lcom/mascotcapsule/micro3d/v3/Light;` | final |
| 2 | `void setLight(Light light)` | `setLight(Lcom/mascotcapsule/micro3d/v3/Light;)V` | final |
| 2 | `int getShading()` | `getShading()I` | final deprecated |
| 2 | `int getShadingType()` | `getShadingType()I` | final |
| 2 | `void setShading(int shading)` | `setShading(I)V` | final deprecated |
| 2 | `void setShadingType(int shading)` | `setShadingType(I)V` | final |
| 2 | `int getThreshold()` | `getThreshold()I` | final deprecated |
| 2 | `int getToonThreshold()` | `getToonThreshold()I` | final |
| 2 | `int getThresholdHigh()` | `getThresholdHigh()I` | final deprecated |
| 2 | `int getToonHigh()` | `getToonHigh()I` | final |
| 2 | `int getThresholdLow()` | `getThresholdLow()I` | final deprecated |
| 2 | `int getToonLow()` | `getToonLow()I` | final |
| 2 | `void setThreshold(int threshold, int high, int low)` | `setThreshold(III)V` | final deprecated |
| 2 | `void setToonParams(int threshold, int high, int low)` | `setToonParams(III)V` | final |
| 2 | `boolean isSemiTransparentEnabled()` | `isSemiTransparentEnabled()Z` | final deprecated |
| 2 | `boolean isTransparency()` | `isTransparency()Z` | final |
| 2 | `void setSemiTransparentEnabled(boolean isEnable)` | `setSemiTransparentEnabled(Z)V` | final deprecated |
| 2 | `void setTransparency(boolean isEnable)` | `setTransparency(Z)V` | final |
| 2 | `Texture getSphereMap()` | `getSphereMap()Lcom/mascotcapsule/micro3d/v3/Texture;` | final deprecated |
| 2 | `Texture getSphereTexture()` | `getSphereTexture()Lcom/mascotcapsule/micro3d/v3/Texture;` | final |
| 2 | `void setSphereMap(Texture tex)` | `setSphereMap(Lcom/mascotcapsule/micro3d/v3/Texture;)V` | final deprecated |
| 2 | `void setSphereTexture(Texture tex)` | `setSphereTexture(Lcom/mascotcapsule/micro3d/v3/Texture;)V` | final |

### Figure

`public class com.mascotcapsule.micro3d.v3.Figure`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Figure.html).
A model loaded from an `.mbac` file or byte array; `ActionTable` (`.mtra`) animates it.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(byte[] b)` | `<init>([B)V` |  |
| 2 | `(String name)` | `<init>(Ljava/lang/String;)V` |  |
| 2 | `void dispose()` | `dispose()V` | final |
| 2 | `void setPosture(ActionTable act, int action, int frame)` | `setPosture(Lcom/mascotcapsule/micro3d/v3/ActionTable;II)V` | final |
| 2 | `Texture getTexture()` | `getTexture()Lcom/mascotcapsule/micro3d/v3/Texture;` | final |
| 2 | `void setTexture(Texture t)` | `setTexture(Lcom/mascotcapsule/micro3d/v3/Texture;)V` | final |
| 2 | `void setTexture(Texture[] t)` | `setTexture([Lcom/mascotcapsule/micro3d/v3/Texture;)V` | final |
| 2 | `int getNumTextures()` | `getNumTextures()I` | final |
| 2 | `void selectTexture(int idx)` | `selectTexture(I)V` | final |
| 2 | `int getNumPattern()` | `getNumPattern()I` | final |
| 2 | `void setPattern(int idx)` | `setPattern(I)V` | final |

### FigureLayout

`public class com.mascotcapsule.micro3d.v3.FigureLayout`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/FigureLayout.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `(AffineTrans trans, int sx, int sy, int cx, int cy)` | `<init>(Lcom/mascotcapsule/micro3d/v3/AffineTrans;IIII)V` |  |
| 2 | `AffineTrans getAffineTrans()` | `getAffineTrans()Lcom/mascotcapsule/micro3d/v3/AffineTrans;` | final |
| 2 | `void setAffineTrans(AffineTrans trans)` | `setAffineTrans(Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final |
| 2 | `void setAffineTransArray(AffineTrans[] trans)` | `setAffineTransArray([Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final deprecated |
| 2 | `void setAffineTrans(AffineTrans[] trans)` | `setAffineTrans([Lcom/mascotcapsule/micro3d/v3/AffineTrans;)V` | final |
| 2 | `void selectAffineTrans(int idx)` | `selectAffineTrans(I)V` | final |
| 2 | `int getScaleX()` | `getScaleX()I` | final |
| 2 | `int getScaleY()` | `getScaleY()I` | final |
| 2 | `void setScale(int sx, int sy)` | `setScale(II)V` | final |
| 2 | `int getParallelWidth()` | `getParallelWidth()I` | final |
| 2 | `int getParallelHeight()` | `getParallelHeight()I` | final |
| 2 | `void setParallelSize(int w, int h)` | `setParallelSize(II)V` | final |
| 2 | `int getCenterX()` | `getCenterX()I` | final |
| 2 | `int getCenterY()` | `getCenterY()I` | final |
| 2 | `void setCenter(int cx, int cy)` | `setCenter(II)V` | final |
| 2 | `void setPerspective(int zNear, int zFar, int angle)` | `setPerspective(III)V` | final |
| 2 | `void setPerspective(int zNear, int zFar, int width, int height)` | `setPerspective(IIII)V` | final |

### com.mascotcapsule.micro3d.v3.Graphics3D

`public class com.mascotcapsule.micro3d.v3.Graphics3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Graphics3D.html).
Binds to a MIDP `Graphics` and renders `Figure`s (from `.mbac` model files) with `Texture`s (`.bmp`) and fixed-point `AffineTrans` transforms. Values are fixed point with 4096 = 1.0.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `COMMAND_LIST_VERSION_1_0` = `-33554431` (`0xfe000001`) | `I` | static |
| 2 | `COMMAND_END` = `-2147483648` (`0x80000000`) | `I` | static |
| 2 | `COMMAND_NOP` = `-2130706432` (`0x81000000`) | `I` | static |
| 2 | `COMMAND_FLUSH` = `-2113929216` (`0x82000000`) | `I` | static |
| 2 | `COMMAND_ATTRIBUTE` = `-2097152000` (`0x83000000`) | `I` | static |
| 2 | `COMMAND_CLIP` = `-2080374784` (`0x84000000`) | `I` | static |
| 2 | `COMMAND_CENTER` = `-2063597568` (`0x85000000`) | `I` | static |
| 2 | `COMMAND_TEXTURE_INDEX` = `-2046820352` (`0x86000000`) | `I` | static |
| 2 | `COMMAND_AFFINE_INDEX` = `-2030043136` (`0x87000000`) | `I` | static |
| 2 | `COMMAND_PARALLEL_SCALE` = `-1879048192` (`0x90000000`) | `I` | static |
| 2 | `COMMAND_PARALLEL_SIZE` = `-1862270976` (`0x91000000`) | `I` | static |
| 2 | `COMMAND_PERSPECTIVE_FOV` = `-1845493760` (`0x92000000`) | `I` | static |
| 2 | `COMMAND_PERSPECTIVE_WH` = `-1828716544` (`0x93000000`) | `I` | static |
| 2 | `COMMAND_AMBIENT_LIGHT` = `-1610612736` (`0xa0000000`) | `I` | static |
| 2 | `COMMAND_DIRECTION_LIGHT` = `-1593835520` (`0xa1000000`) | `I` | static |
| 2 | `COMMAND_THRESHOLD` = `-1358954496` (`0xaf000000`) | `I` | static |
| 2 | `PRIMITVE_POINTS` = `16777216` (`0x1000000`) | `I` | static |
| 2 | `PRIMITVE_LINES` = `33554432` (`0x2000000`) | `I` | static |
| 2 | `PRIMITVE_TRIANGLES` = `50331648` (`0x3000000`) | `I` | static |
| 2 | `PRIMITVE_QUADS` = `67108864` (`0x4000000`) | `I` | static |
| 2 | `PRIMITVE_POINT_SPRITES` = `83886080` (`0x5000000`) | `I` | static |
| 2 | `POINT_SPRITE_LOCAL_SIZE` = `0` | `I` | static |
| 2 | `POINT_SPRITE_PIXEL_SIZE` = `1` | `I` | static |
| 2 | `POINT_SPRITE_PERSPECTIVE` = `0` | `I` | static |
| 2 | `POINT_SPRITE_NO_PERS` = `2` | `I` | static |
| 2 | `ENV_ATTR_LIGHTING` = `1` | `I` | static |
| 2 | `ENV_ATTR_SPHERE_MAP` = `2` | `I` | static |
| 2 | `ENV_ATTR_TOON_SHADING` = `4` | `I` | static |
| 2 | `ENV_ATTR_SEMI_TRANSPARENT` = `8` | `I` | static |
| 2 | `PATTR_LIGHTING` = `1` | `I` | static |
| 2 | `PATTR_SPHERE_MAP` = `2` | `I` | static |
| 2 | `PATTR_COLORKEY` = `16` | `I` | static |
| 2 | `PATTR_BLEND_NORMAL` = `0` | `I` | static |
| 2 | `PATTR_BLEND_HALF` = `32` | `I` | static |
| 2 | `PATTR_BLEND_ADD` = `64` | `I` | static |
| 2 | `PATTR_BLEND_SUB` = `96` | `I` | static |
| 2 | `PDATA_NORMAL_NONE` = `0` | `I` | static |
| 2 | `PDATA_NORMAL_PER_FACE` = `512` (`0x200`) | `I` | static |
| 2 | `PDATA_NORMAL_PER_VERTEX` = `768` | `I` | static |
| 2 | `PDATA_COLOR_NONE` = `0` | `I` | static |
| 2 | `PDATA_COLOR_PER_COMMAND` = `1024` (`0x400`) | `I` | static |
| 2 | `PDATA_COLOR_PER_FACE` = `2048` (`0x800`) | `I` | static |
| 2 | `PDATA_TEXURE_COORD_NONE` = `0` | `I` | static |
| 2 | `PDATA_POINT_SPRITE_PARAMS_PER_CMD` = `4096` (`0x1000`) | `I` | static |
| 2 | `PDATA_POINT_SPRITE_PARAMS_PER_FACE` = `8192` (`0x2000`) | `I` | static |
| 2 | `PDATA_POINT_SPRITE_PARAMS_PER_VERTEX` = `12288` | `I` | static |
| 2 | `PDATA_TEXURE_COORD` = `12288` | `I` | static |
| 2 | `()` | `<init>()V` |  |
| 2 | `void bind(Graphics graphics)` | `bind(Ljavax/microedition/lcdui/Graphics;)V` | final |
| 2 | `void release(Graphics graphics)` | `release(Ljavax/microedition/lcdui/Graphics;)V` | final |
| 2 | `void renderPrimitives(Texture texture, int x, int y, FigureLayout layout, Effect3D effect, int command, int numPrimitives, int[] vertexCoords, int[] normals, int[] textureCoords, int[] colors)` | `renderPrimitives(Lcom/mascotcapsule/micro3d/v3/Texture;IILcom/mascotcapsule/micro3d/v3/FigureLayout;Lcom/mascotcapsule/micro3d/v3/Effect3D;II[I[I[I[I)V` | final |
| 2 | `void drawCommandList(Texture[] textures, int x, int y, FigureLayout layout, Effect3D effect, int[] commandList)` | `drawCommandList([Lcom/mascotcapsule/micro3d/v3/Texture;IILcom/mascotcapsule/micro3d/v3/FigureLayout;Lcom/mascotcapsule/micro3d/v3/Effect3D;[I)V` | final |
| 2 | `void drawCommandList(Texture texture, int x, int y, FigureLayout layout, Effect3D effect, int[] commandList)` | `drawCommandList(Lcom/mascotcapsule/micro3d/v3/Texture;IILcom/mascotcapsule/micro3d/v3/FigureLayout;Lcom/mascotcapsule/micro3d/v3/Effect3D;[I)V` | final |
| 2 | `void renderFigure(Figure figure, int x, int y, FigureLayout layout, Effect3D effect)` | `renderFigure(Lcom/mascotcapsule/micro3d/v3/Figure;IILcom/mascotcapsule/micro3d/v3/FigureLayout;Lcom/mascotcapsule/micro3d/v3/Effect3D;)V` | final |
| 2 | `void drawFigure(Figure figure, int x, int y, FigureLayout layout, Effect3D effect)` | `drawFigure(Lcom/mascotcapsule/micro3d/v3/Figure;IILcom/mascotcapsule/micro3d/v3/FigureLayout;Lcom/mascotcapsule/micro3d/v3/Effect3D;)V` | final |
| 2 | `void flush()` | `flush()V` | final |
| 2 | `void dispose()` | `dispose()V` | final |

### com.mascotcapsule.micro3d.v3.Light

`public class com.mascotcapsule.micro3d.v3.Light`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Light.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `()` | `<init>()V` |  |
| 2 | `(Vector3D dir, int dirIntensity, int ambIntensity)` | `<init>(Lcom/mascotcapsule/micro3d/v3/Vector3D;II)V` |  |
| 2 | `int getDirIntensity()` | `getDirIntensity()I` | final deprecated |
| 2 | `int getParallelLightIntensity()` | `getParallelLightIntensity()I` | final |
| 2 | `void setDirIntensity(int p)` | `setDirIntensity(I)V` | final deprecated |
| 2 | `void setParallelLightIntensity(int p)` | `setParallelLightIntensity(I)V` | final |
| 2 | `int getAmbIntensity()` | `getAmbIntensity()I` | final deprecated |
| 2 | `int getAmbientIntensity()` | `getAmbientIntensity()I` | final |
| 2 | `void setAmbIntensity(int p)` | `setAmbIntensity(I)V` | final deprecated |
| 2 | `void setAmbientIntensity(int p)` | `setAmbientIntensity(I)V` | final |
| 2 | `Vector3D getDirection()` | `getDirection()Lcom/mascotcapsule/micro3d/v3/Vector3D;` | final deprecated |
| 2 | `Vector3D getParallelLightDirection()` | `getParallelLightDirection()Lcom/mascotcapsule/micro3d/v3/Vector3D;` | final |
| 2 | `void setDirection(Vector3D v)` | `setDirection(Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final deprecated |
| 2 | `void setParallelLightDirection(Vector3D v)` | `setParallelLightDirection(Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final |

### Texture

`public class com.mascotcapsule.micro3d.v3.Texture`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Texture.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `(byte[] b, boolean isForModel)` | `<init>([BZ)V` |  |
| 2 | `(String name, boolean isForModel)` | `<init>(Ljava/lang/String;Z)V` |  |
| 2 | `void dispose()` | `dispose()V` | final |

### Util3D

`public class com.mascotcapsule.micro3d.v3.Util3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Util3D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `int sqrt(int p)` | `sqrt(I)I` | static final |
| 2 | `int sin(int p)` | `sin(I)I` | static final |
| 2 | `int cos(int p)` | `cos(I)I` | static final |

### Vector3D

`public class com.mascotcapsule.micro3d.v3.Vector3D`. Tier 2. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/Mascot_Capsule_API/com/mascotcapsule/micro3d/v3/Vector3D.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 2 | `x` = (not a constant) | `I` | not final |
| 2 | `y` = (not a constant) | `I` | not final |
| 2 | `z` = (not a constant) | `I` | not final |
| 2 | `()` | `<init>()V` |  |
| 2 | `(Vector3D v)` | `<init>(Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` |  |
| 2 | `(int x, int y, int z)` | `<init>(III)V` |  |
| 2 | `void unit()` | `unit()V` | final |
| 2 | `int getX()` | `getX()I` | final |
| 2 | `int getY()` | `getY()I` | final |
| 2 | `int getZ()` | `getZ()I` | final |
| 2 | `void setX(int x)` | `setX(I)V` | final |
| 2 | `void setY(int y)` | `setY(I)V` | final |
| 2 | `void setZ(int z)` | `setZ(I)V` | final |
| 2 | `void set(Vector3D v)` | `set(Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final |
| 2 | `void set(int x, int y, int z)` | `set(III)V` | final |
| 2 | `int innerProduct(Vector3D v)` | `innerProduct(Lcom/mascotcapsule/micro3d/v3/Vector3D;)I` | final |
| 2 | `void outerProduct(Vector3D v)` | `outerProduct(Lcom/mascotcapsule/micro3d/v3/Vector3D;)V` | final |
| 2 | `int innerProduct(Vector3D v1, Vector3D v2)` | `innerProduct(Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;)I` | static final |
| 2 | `Vector3D outerProduct(Vector3D v1, Vector3D v2)` | `outerProduct(Lcom/mascotcapsule/micro3d/v3/Vector3D;Lcom/mascotcapsule/micro3d/v3/Vector3D;)Lcom/mascotcapsule/micro3d/v3/Vector3D;` | static final |

## JSR-75: FileConnection

FileConnection lets games read and write files outside RMS. A few games
keep saves or downloaded content there.

### ConnectionClosedException

`public class javax.microedition.io.file.ConnectionClosedException extends java.lang.RuntimeException`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/ConnectionClosedException.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `()` | `<init>()V` |  |
| 3 | `(String detailMessage)` | `<init>(Ljava/lang/String;)V` |  |

### FileConnection

`public interface javax.microedition.io.file.FileConnection extends javax.microedition.io.StreamConnection`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/FileConnection.html).
Opened with `Connector.open("file:///root/path")`. Roots come from `FileSystemRegistry.listRoots()` and the `fileconn.dir.*` properties.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `boolean isOpen()` | `isOpen()Z` |  |
| 3 | `InputStream openInputStream()` | `openInputStream()Ljava/io/InputStream;` |  |
| 3 | `DataInputStream openDataInputStream()` | `openDataInputStream()Ljava/io/DataInputStream;` |  |
| 3 | `OutputStream openOutputStream()` | `openOutputStream()Ljava/io/OutputStream;` |  |
| 3 | `DataOutputStream openDataOutputStream()` | `openDataOutputStream()Ljava/io/DataOutputStream;` |  |
| 3 | `OutputStream openOutputStream(long byteOffset)` | `openOutputStream(J)Ljava/io/OutputStream;` |  |
| 3 | `long totalSize()` | `totalSize()J` |  |
| 3 | `long availableSize()` | `availableSize()J` |  |
| 3 | `long usedSize()` | `usedSize()J` |  |
| 3 | `long directorySize(boolean includeSubDirs)` | `directorySize(Z)J` |  |
| 3 | `long fileSize()` | `fileSize()J` |  |
| 3 | `boolean canRead()` | `canRead()Z` |  |
| 3 | `boolean canWrite()` | `canWrite()Z` |  |
| 3 | `boolean isHidden()` | `isHidden()Z` |  |
| 3 | `void setReadable(boolean readable)` | `setReadable(Z)V` |  |
| 3 | `void setWritable(boolean writable)` | `setWritable(Z)V` |  |
| 3 | `void setHidden(boolean hidden)` | `setHidden(Z)V` |  |
| 3 | `Enumeration list()` | `list()Ljava/util/Enumeration;` |  |
| 3 | `Enumeration list(String filter, boolean includeHidden)` | `list(Ljava/lang/String;Z)Ljava/util/Enumeration;` |  |
| 3 | `void create()` | `create()V` |  |
| 3 | `void mkdir()` | `mkdir()V` |  |
| 3 | `boolean exists()` | `exists()Z` |  |
| 3 | `boolean isDirectory()` | `isDirectory()Z` |  |
| 3 | `void delete()` | `delete()V` |  |
| 3 | `void rename(String newName)` | `rename(Ljava/lang/String;)V` |  |
| 3 | `void truncate(long byteOffset)` | `truncate(J)V` |  |
| 3 | `void setFileConnection(String fileName)` | `setFileConnection(Ljava/lang/String;)V` |  |
| 3 | `String getName()` | `getName()Ljava/lang/String;` |  |
| 3 | `String getPath()` | `getPath()Ljava/lang/String;` |  |
| 3 | `String getURL()` | `getURL()Ljava/lang/String;` |  |
| 3 | `long lastModified()` | `lastModified()J` |  |

### FileSystemListener

`public interface javax.microedition.io.file.FileSystemListener`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/FileSystemListener.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `ROOT_ADDED` = `0` | `I` | static |
| 3 | `ROOT_REMOVED` = `1` | `I` | static |
| 3 | `void rootChanged(int state, String rootName)` | `rootChanged(ILjava/lang/String;)V` |  |

### FileSystemRegistry

`public class javax.microedition.io.file.FileSystemRegistry`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/FileSystemRegistry.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `boolean addFileSystemListener(FileSystemListener listener)` | `addFileSystemListener(Ljavax/microedition/io/file/FileSystemListener;)Z` | static |
| 3 | `boolean removeFileSystemListener(FileSystemListener listener)` | `removeFileSystemListener(Ljavax/microedition/io/file/FileSystemListener;)Z` | static |
| 3 | `Enumeration listRoots()` | `listRoots()Ljava/util/Enumeration;` | static |

### IllegalModeException

`public class javax.microedition.io.file.IllegalModeException extends java.lang.RuntimeException`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr75/javax/microedition/io/file/IllegalModeException.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `()` | `<init>()V` |  |
| 3 | `(String detailMessage)` | `<init>(Ljava/lang/String;)V` |  |

## JSR-120: Wireless Messaging

### BinaryMessage

`public interface javax.wireless.messaging.BinaryMessage extends javax.wireless.messaging.Message`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/javax/wireless/messaging/BinaryMessage.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `byte[] getPayloadData()` | `getPayloadData()[B` |  |
| 3 | `void setPayloadData(byte[] data)` | `setPayloadData([B)V` |  |

### Message

`public interface javax.wireless.messaging.Message`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/javax/wireless/messaging/Message.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `String getAddress()` | `getAddress()Ljava/lang/String;` |  |
| 3 | `void setAddress(String addr)` | `setAddress(Ljava/lang/String;)V` |  |
| 3 | `Date getTimestamp()` | `getTimestamp()Ljava/util/Date;` |  |

### MessageConnection

`public interface javax.wireless.messaging.MessageConnection extends javax.microedition.io.Connection`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/javax/wireless/messaging/MessageConnection.html).
Opened with `Connector.open("sms://...")`. FastEmu must never send a real message.

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `TEXT_MESSAGE` = `"text"` | `Ljava/lang/String;` | static |
| 3 | `BINARY_MESSAGE` = `"binary"` | `Ljava/lang/String;` | static |
| 3 | `Message newMessage(String type)` | `newMessage(Ljava/lang/String;)Ljavax/wireless/messaging/Message;` |  |
| 3 | `Message newMessage(String type, String address)` | `newMessage(Ljava/lang/String;Ljava/lang/String;)Ljavax/wireless/messaging/Message;` |  |
| 3 | `void send(Message msg)` | `send(Ljavax/wireless/messaging/Message;)V` |  |
| 3 | `Message receive()` | `receive()Ljavax/wireless/messaging/Message;` |  |
| 3 | `void setMessageListener(MessageListener l)` | `setMessageListener(Ljavax/wireless/messaging/MessageListener;)V` |  |
| 3 | `int numberOfSegments(Message msg)` | `numberOfSegments(Ljavax/wireless/messaging/Message;)I` |  |

### MessageListener

`public interface javax.wireless.messaging.MessageListener`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/javax/wireless/messaging/MessageListener.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `void notifyIncomingMessage(MessageConnection conn)` | `notifyIncomingMessage(Ljavax/wireless/messaging/MessageConnection;)V` |  |

### TextMessage

`public interface javax.wireless.messaging.TextMessage extends javax.wireless.messaging.Message`. Tier 3. [Javadoc](https://nikita36078.github.io/J2ME_Docs/docs/jsr120/javax/wireless/messaging/TextMessage.html).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `String getPayloadText()` | `getPayloadText()Ljava/lang/String;` |  |
| 3 | `void setPayloadText(String data)` | `setPayloadText(Ljava/lang/String;)V` |  |

## JSR-75: PIM

| Class | Kind | Tier | Members | Javadoc |
|---|---|---|---|---|
| `javax.microedition.pim.Contact` | interface | 3 | 45 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/Contact.html) |
| `javax.microedition.pim.ContactList` | interface | 3 | 3 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/ContactList.html) |
| `javax.microedition.pim.Event` | interface | 3 | 14 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/Event.html) |
| `javax.microedition.pim.EventList` | interface | 3 | 8 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/EventList.html) |
| `javax.microedition.pim.FieldEmptyException` | class | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/FieldEmptyException.html) |
| `javax.microedition.pim.FieldFullException` | class | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/FieldFullException.html) |
| `javax.microedition.pim.PIM` | class | 3 | 14 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/PIM.html) |
| `javax.microedition.pim.PIMException` | class | 3 | 11 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/PIMException.html) |
| `javax.microedition.pim.PIMItem` | interface | 3 | 38 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/PIMItem.html) |
| `javax.microedition.pim.PIMList` | interface | 3 | 25 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/PIMList.html) |
| `javax.microedition.pim.RepeatRule` | class | 3 | 53 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/RepeatRule.html) |
| `javax.microedition.pim.ToDo` | interface | 3 | 12 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/ToDo.html) |
| `javax.microedition.pim.ToDoList` | interface | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/ToDoList.html) |
| `javax.microedition.pim.UnsupportedFieldException` | class | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr75-pim/javax/microedition/pim/UnsupportedFieldException.html) |

## JSR-82: Bluetooth and OBEX

| Class | Kind | Tier | Members | Javadoc |
|---|---|---|---|---|
| `javax.bluetooth.BluetoothConnectionException` | class | 3 | 9 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/BluetoothConnectionException.html) |
| `javax.bluetooth.BluetoothStateException` | class | 3 | 2 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/BluetoothStateException.html) |
| `javax.bluetooth.DataElement` | class | 3 | 29 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/DataElement.html) |
| `javax.bluetooth.DeviceClass` | class | 3 | 4 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/DeviceClass.html) |
| `javax.bluetooth.DiscoveryAgent` | class | 3 | 11 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/DiscoveryAgent.html) |
| `javax.bluetooth.DiscoveryListener` | interface | 3 | 12 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/DiscoveryListener.html) |
| `javax.bluetooth.L2CAPConnection` | interface | 3 | 7 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/L2CAPConnection.html) |
| `javax.bluetooth.L2CAPConnectionNotifier` | interface | 3 | 1 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/L2CAPConnectionNotifier.html) |
| `javax.bluetooth.LocalDevice` | class | 3 | 11 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/LocalDevice.html) |
| `javax.bluetooth.RemoteDevice` | class | 3 | 13 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/RemoteDevice.html) |
| `javax.bluetooth.ServiceRecord` | interface | 3 | 10 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/ServiceRecord.html) |
| `javax.bluetooth.ServiceRegistrationException` | class | 3 | 2 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/ServiceRegistrationException.html) |
| `javax.bluetooth.UUID` | class | 3 | 5 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/bluetooth/UUID.html) |
| `javax.obex.Authenticator` | interface | 3 | 2 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/Authenticator.html) |
| `javax.obex.ClientSession` | interface | 3 | 10 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/ClientSession.html) |
| `javax.obex.HeaderSet` | interface | 3 | 17 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/HeaderSet.html) |
| `javax.obex.Operation` | interface | 3 | 4 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/Operation.html) |
| `javax.obex.PasswordAuthentication` | class | 3 | 3 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/PasswordAuthentication.html) |
| `javax.obex.ResponseCodes` | class | 3 | 37 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/ResponseCodes.html) |
| `javax.obex.ServerRequestHandler` | class | 3 | 11 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/ServerRequestHandler.html) |
| `javax.obex.SessionNotifier` | interface | 3 | 2 | [page](https://docs.oracle.com/javame/config/cldc/opt-pkgs/api/bluetooth/jsr082/javax/obex/SessionNotifier.html) |

## JSR-179: Location

| Class | Kind | Tier | Members | Javadoc |
|---|---|---|---|---|
| `javax.microedition.location.AddressInfo` | class | 3 | 20 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/AddressInfo.html) |
| `javax.microedition.location.Coordinates` | class | 3 | 13 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/Coordinates.html) |
| `javax.microedition.location.Criteria` | class | 3 | 21 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/Criteria.html) |
| `javax.microedition.location.Landmark` | class | 3 | 9 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/Landmark.html) |
| `javax.microedition.location.LandmarkException` | class | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/LandmarkException.html) |
| `javax.microedition.location.LandmarkStore` | class | 3 | 14 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/LandmarkStore.html) |
| `javax.microedition.location.Location` | class | 3 | 19 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/Location.html) |
| `javax.microedition.location.LocationException` | class | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/LocationException.html) |
| `javax.microedition.location.LocationListener` | interface | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/LocationListener.html) |
| `javax.microedition.location.LocationProvider` | class | 3 | 12 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/LocationProvider.html) |
| `javax.microedition.location.Orientation` | class | 3 | 6 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/Orientation.html) |
| `javax.microedition.location.ProximityListener` | interface | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/ProximityListener.html) |
| `javax.microedition.location.QualifiedCoordinates` | class | 3 | 5 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr179-1_1-mrel-javadoc/javax/microedition/location/QualifiedCoordinates.html) |

## JSR-226: Scalable 2D Vector Graphics

| Class | Kind | Tier | Members | Javadoc |
|---|---|---|---|---|
| `javax.microedition.m2g.ExternalResourceHandler` | interface | 3 | 1 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/ExternalResourceHandler.html) |
| `javax.microedition.m2g.SVGAnimator` | class | 3 | 12 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/SVGAnimator.html) |
| `javax.microedition.m2g.SVGEventListener` | interface | 3 | 7 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/SVGEventListener.html) |
| `javax.microedition.m2g.SVGImage` | class | 3 | 12 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/SVGImage.html) |
| `javax.microedition.m2g.ScalableGraphics` | class | 3 | 8 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/ScalableGraphics.html) |
| `javax.microedition.m2g.ScalableImage` | class | 3 | 8 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/javax/microedition/m2g/ScalableImage.html) |
| `org.w3c.dom.DOMException` | class | 3 | 12 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/DOMException.html) |
| `org.w3c.dom.Document` | interface | 3 | 3 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/Document.html) |
| `org.w3c.dom.Element` | interface | 3 | 0 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/Element.html) |
| `org.w3c.dom.Node` | interface | 3 | 6 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/Node.html) |
| `org.w3c.dom.events.Event` | interface | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/events/Event.html) |
| `org.w3c.dom.events.EventListener` | interface | 3 | 1 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/events/EventListener.html) |
| `org.w3c.dom.events.EventTarget` | interface | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/events/EventTarget.html) |
| `org.w3c.dom.svg.SVGAnimationElement` | interface | 3 | 2 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGAnimationElement.html) |
| `org.w3c.dom.svg.SVGElement` | interface | 3 | 18 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGElement.html) |
| `org.w3c.dom.svg.SVGException` | class | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGException.html) |
| `org.w3c.dom.svg.SVGLocatableElement` | interface | 3 | 3 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGLocatableElement.html) |
| `org.w3c.dom.svg.SVGMatrix` | interface | 3 | 6 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGMatrix.html) |
| `org.w3c.dom.svg.SVGPath` | interface | 3 | 13 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGPath.html) |
| `org.w3c.dom.svg.SVGPoint` | interface | 3 | 4 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGPoint.html) |
| `org.w3c.dom.svg.SVGRGBColor` | interface | 3 | 3 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGRGBColor.html) |
| `org.w3c.dom.svg.SVGRect` | interface | 3 | 8 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGRect.html) |
| `org.w3c.dom.svg.SVGSVGElement` | interface | 3 | 11 | [page](https://nikita36078.github.io/J2ME_Docs/docs/jsr226/org/w3c/dom/svg/SVGSVGElement.html) |

## Cross-check

Every member above comes from the javadoc named first under Sources. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
