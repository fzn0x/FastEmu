# The native library calls these by name over JNI, which R8 can't see.
# Removing or renaming any of them fails at run time, not at build time.

# MainActivity's methods for Rust: gestureConfiguration, keyCharacter,
# openUrl, setKeepScreenOn, pickSuite and enterText.
-keep class io.github.fzn0x.fastemu.MainActivity {
    public <methods>;
}

# The native methods Kotlin calls, bound by their JNI names.
-keep class io.github.fzn0x.fastemu.NativeBridge {
    *;
}

# GameActivity's native code (android-activity's copy, not the AAR's)
# looks up its Java side's fields and methods by name.
-keep class com.google.androidgamesdk.** {
    *;
}
