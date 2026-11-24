package io.github.fzn0x.fastemu

/**
 * The native methods of libfastemu_android.so that Kotlin calls. Every one
 * is declared here, and loading the library first makes each safe to call
 * before GameActivity has loaded it.
 */
object NativeBridge {
    init {
        System.loadLibrary("fastemu_android")
    }

    /** The window's safe area in physical pixels: system bars and cutouts. */
    @JvmStatic
    external fun nativeOnWindowInsetsChanged(left: Int, top: Int, right: Int, bottom: Int)

    /** A game another app opened with FastEmu, copied into app storage. */
    @JvmStatic
    external fun nativeOpenUrl(url: String)

    /**
     * The answer to [MainActivity.pickSuite]: the path of the picked game's
     * copy, or null when the user cancelled or the copy failed.
     */
    @JvmStatic
    external fun nativeSuitePicked(request: Long, path: String?)
}
