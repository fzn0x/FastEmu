package io.github.fzn0x.fastemu

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import java.io.IOException
import java.security.MessageDigest

/**
 * Games the user opens arrive as content URIs, which the emulator can't read
 * by path, so they are copied into the app's own storage first.
 *
 * Each pick goes into a folder of `files/games` named after what it holds,
 * the files' names and bytes. Opening the same files again lands on the same
 * copies, so the library keeps one entry for them; a different file with the
 * same name, such as a game's newer version, gets a folder of its own and
 * never overwrites a JAR a running game reads. A `.jad` and its `.jar` picked
 * together share a folder, so the descriptor finds its JAR beside it.
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

    /** Copies [uris] into `files/games`; the copies that worked. Blocks. */
    fun copyIn(context: Context, uris: List<Uri>): List<File> {
        val games = File(context.filesDir, "games").apply { mkdirs() }
        val incoming = File(games, ".incoming-${System.nanoTime()}")
        try {
            if (!incoming.mkdirs()) throw IOException("can't make $incoming")
            // Each copy's name and the SHA-256 of its bytes, sorted so the
            // order of the pick doesn't matter.
            val copied = uris.mapNotNull { copyOne(context, it, incoming) }.sorted()
            if (copied.isEmpty()) return emptyList()
            val folder = File(games, hex(sha256(copied.joinToString("\n"))).take(16))
            if (!folder.isDirectory && !incoming.renameTo(folder)) {
                throw IOException("can't move the copies to $folder")
            }
            val copies = copied.map { File(folder, it.substringBefore('\n')) }
            Log.i(TAG, "copied ${uris.size} file(s) to $folder")
            return copies
        } catch (error: IOException) {
            Log.e(TAG, "couldn't copy $uris", error)
            return emptyList()
        } finally {
            incoming.deleteRecursively()
        }
    }

    /** Copies one file into [folder]; its name and its bytes' digest. */
    private fun copyOne(context: Context, uri: Uri, folder: File): String? =
        try {
            val target = File(folder, displayName(context, uri))
            val digest = MessageDigest.getInstance("SHA-256")
            val input = context.contentResolver.openInputStream(uri)
                ?: throw IOException("no stream for $uri")
            input.use { source ->
                target.outputStream().use { sink ->
                    val buffer = ByteArray(64 * 1024)
                    while (true) {
                        val read = source.read(buffer)
                        if (read < 0) break
                        digest.update(buffer, 0, read)
                        sink.write(buffer, 0, read)
                    }
                }
            }
            "${target.name}\n${hex(digest.digest())}"
        } catch (error: IOException) {
            Log.e(TAG, "couldn't copy $uri", error)
            null
        } catch (error: SecurityException) {
            Log.e(TAG, "no permission to read $uri", error)
            null
        }

    private fun sha256(text: String): ByteArray =
        MessageDigest.getInstance("SHA-256").digest(text.toByteArray())

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it.toInt() and 0xff) }

    /** The file's name as its provider reports it, safe to use as a file name. */
    private fun displayName(context: Context, uri: Uri): String {
        val reported = if (uri.scheme == "file") {
            uri.lastPathSegment
        } else {
            context.contentResolver
                .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                ?.use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        }
        // A provider's name may hold a path; only its last part is kept, and
        // a line break would split the list the native side reads.
        val name = reported?.substringAfterLast('/')?.replace('\n', ' ')?.trim()
        return name?.takeIf { it.isNotEmpty() && it != "." && it != ".." } ?: "game.jar"
    }
}
