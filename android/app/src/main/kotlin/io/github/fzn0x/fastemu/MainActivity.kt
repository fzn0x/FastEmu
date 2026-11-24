package io.github.fzn0x.fastemu

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Process
import android.util.Log
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.view.WindowManager
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity
import kotlin.concurrent.thread

/**
 * Hosts FastEmu's GPUI window. GameActivity hands its surface and input to
 * the native library (`android_main` in libfastemu_android.so); this class
 * adds only what that library asks for over JNI, marked "called from Rust".
 */
class MainActivity : GameActivity() {
    /** The [pickSuite] request the open picker answers, or 0 for none. */
    private var pickRequest = 0L

    private val openDocuments =
        registerForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            val request = pickRequest
            pickRequest = 0
            if (request != 0L) copyPickedSuites(request, uris)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enterImmersiveMode()
        reportSafeArea()
        // Reopening from Recents brings back the intent that started the task.
        val relaunched = savedInstanceState != null ||
            (intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY) != 0
        if (!relaunched) openFromIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        openFromIntent(intent)
    }

    /**
     * GameActivity hands every key but volume and camera to the native
     * library, so Back never reaches Android. Back leaves the game where it
     * is and shows the home screen instead of closing the app.
     */
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.action == KeyEvent.ACTION_UP && !event.isCanceled) moveTaskToBack(true)
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    /**
     * GPUI starts once per process and can't start again in it, so the
     * process ends with the Activity and the next launch starts afresh.
     */
    override fun onDestroy() {
        Log.i(TAG, "onDestroy: ending the process")
        super.onDestroy()
        Process.killProcess(Process.myPid())
    }

    /** Full screen, as games are; a swipe from the edge shows the bars for a while. */
    private fun enterImmersiveMode() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }

    /**
     * Tells GPUI where the system bars and display cutouts cover the window.
     * A 1x1 view listens so GameActivity's own insets listener stays in place.
     */
    private fun reportSafeArea() {
        val probe = View(this)
        addContentView(probe, ViewGroup.LayoutParams(1, 1))
        ViewCompat.setOnApplyWindowInsetsListener(probe) { _, insets ->
            val safe = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
            )
            NativeBridge.nativeOnWindowInsetsChanged(safe.left, safe.top, safe.right, safe.bottom)
            insets
        }
    }

    /** A game another app opened with FastEmu ("Open with"). */
    private fun openFromIntent(intent: Intent?) {
        if (intent?.action != Intent.ACTION_VIEW) return
        val uri = intent.data ?: return
        thread(name = "fastemu_open") {
            val copy = GameFiles.copyIn(this, uri) ?: return@thread
            NativeBridge.nativeOpenUrl(Uri.fromFile(copy).toString())
        }
    }

    /**
     * Shows Android's document picker for a game. Picking a `.jad` and its
     * `.jar` together opens the descriptor with its JAR beside it. Called
     * from Rust; the answer goes back through [NativeBridge.nativeSuitePicked].
     */
    @Suppress("unused")
    fun pickSuite(request: Long) {
        runOnUiThread {
            // One picker at a time: an earlier request is cancelled.
            if (pickRequest != 0L) NativeBridge.nativeSuitePicked(pickRequest, null)
            pickRequest = request
            try {
                openDocuments.launch(GameFiles.suiteTypes)
            } catch (error: ActivityNotFoundException) {
                Log.e(TAG, "no document picker", error)
                pickRequest = 0
                NativeBridge.nativeSuitePicked(request, null)
            }
        }
    }

    private fun copyPickedSuites(request: Long, uris: List<Uri>) {
        if (uris.isEmpty()) {
            NativeBridge.nativeSuitePicked(request, null)
            return
        }
        thread(name = "fastemu_copy") {
            val copies = uris.mapNotNull { GameFiles.copyIn(this, it) }
            // A descriptor names its JAR, so it is what opens when both came.
            val suite = copies.firstOrNull { it.extension.equals("jad", ignoreCase = true) }
                ?: copies.firstOrNull()
            NativeBridge.nativeSuitePicked(request, suite?.absolutePath)
        }
    }

    /**
     * `ViewConfiguration`'s values for GPUI's touch recognizer: touch slop
     * and minimum fling velocity in logical pixels, then the double-tap and
     * long-press timeouts in milliseconds. Called from Rust.
     */
    @Suppress("unused")
    fun gestureConfiguration(): FloatArray {
        val configuration = ViewConfiguration.get(this)
        val density = resources.displayMetrics.density
        return floatArrayOf(
            configuration.scaledTouchSlop / density,
            configuration.scaledMinimumFlingVelocity / density,
            ViewConfiguration.getDoubleTapTimeout().toFloat(),
            ViewConfiguration.getLongPressTimeout().toFloat(),
        )
    }

    /**
     * What a hardware key types on its keyboard's layout, as
     * `KeyCharacterMap.get` returns it, looked up on each press because the
     * layout can change while the app runs. Called from Rust.
     */
    @Suppress("unused")
    fun keyCharacter(deviceId: Int, keyCode: Int, metaState: Int): Int =
        try {
            KeyCharacterMap.load(deviceId).get(keyCode, metaState)
        } catch (error: KeyCharacterMap.UnavailableException) {
            Log.w(TAG, "no key map for input device $deviceId", error)
            0
        }

    /** Opens a link in the user's browser: GPUI's `open_url`. Called from Rust. */
    @Suppress("unused")
    fun openUrl(url: String) {
        runOnUiThread {
            try {
                startActivity(
                    Intent(Intent.ACTION_VIEW, Uri.parse(url))
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                )
            } catch (error: ActivityNotFoundException) {
                Log.e(TAG, "no app opens $url", error)
            }
        }
    }

    /** GPUI's `prevent_idle_sleep`. Called from Rust. */
    @Suppress("unused")
    fun setKeepScreenOn(on: Boolean) {
        runOnUiThread {
            if (on) {
                window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            } else {
                window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }

    private companion object {
        const val TAG = "fastemu_activity"
    }
}
