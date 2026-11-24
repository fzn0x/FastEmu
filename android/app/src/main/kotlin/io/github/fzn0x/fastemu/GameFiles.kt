package io.github.fzn0x.fastemu

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import java.io.IOException

/**
 * Games the user opens arrive as content URIs, which the emulator can't read
 * by path, so they are copied into the app's own storage first. A file with
 * the same name replaces the earlier copy, as a game's newer version would.
 */
object GameFiles {
    private const val TAG = "fastemu_files"

    /** The MIME types file managers and browsers give `.jar` and `.jad` files. */
    val suiteTypes = arrayOf(
        "application/java-archive",
        "application/x-java-archive",
        "text/vnd.sun.j2me.app-descriptor",
        // What Android reports for a file whose type it doesn't know.
        "application/octet-stream",
    )

    /** Copies [uri] into `files/games`; null when it can't be read. Blocks. */
    fun copyIn(context: Context, uri: Uri): File? {
        val games = File(context.filesDir, "games").apply { mkdirs() }
        var copy: File? = null
        return try {
            val target = File(games, displayName(context, uri))
            copy = target
            val input = context.contentResolver.openInputStream(uri)
                ?: throw IOException("no stream for $uri")
            input.use { source -> target.outputStream().use { source.copyTo(it) } }
            Log.i(TAG, "copied $uri to $target (${target.length()} bytes)")
            target
        } catch (error: IOException) {
            Log.e(TAG, "couldn't copy $uri", error)
            copy?.delete()
            null
        } catch (error: SecurityException) {
            Log.e(TAG, "no permission to read $uri", error)
            null
        }
    }

    /** The file's name as its provider reports it, safe to use as a file name. */
    private fun displayName(context: Context, uri: Uri): String {
        val reported = if (uri.scheme == "file") {
            uri.lastPathSegment
        } else {
            context.contentResolver
                .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                ?.use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        }
        // A provider's name may hold a path; only its last part is kept.
        val name = reported?.substringAfterLast('/')?.trim()
        return name?.takeIf { it.isNotEmpty() && it != "." && it != ".." } ?: "game.jar"
    }
}
