package io.github.fzn0x.fastemu

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
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
 * together share a folder, so the descriptor finds its JAR beside it. A file
 * over [MAX_FILE_BYTES] is not copied.
 */
object GameFiles {
    private const val TAG = "fastemu_files"

    /**
     * The most bytes of one file copied: 64 MiB, the largest JAR the suite
     * reader opens, so a bigger file could not run anyway.
     */
    const val MAX_FILE_BYTES = 64L * 1024 * 1024

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

    /**
     * Copies one file into [folder]; its name and its bytes' digest. A file
     * over [MAX_FILE_BYTES], or one that fails midway, leaves no part of
     * itself in the folder.
     */
    private fun copyOne(context: Context, uri: Uri, folder: File): String? {
        var target: File? = null
        try {
            val file = File(folder, displayName(context, uri))
            target = file
            val digest = MessageDigest.getInstance("SHA-256")
            val input = context.contentResolver.openInputStream(uri)
                ?: throw IOException("no stream for $uri")
            input.use { source ->
                file.outputStream().use { sink -> copyLimited(source, sink, digest) }
            }
            return "${file.name}\n${hex(digest.digest())}"
        } catch (error: IOException) {
            Log.e(TAG, "couldn't copy $uri", error)
        } catch (error: SecurityException) {
            Log.e(TAG, "no permission to read $uri", error)
        }
        target?.delete()
        return null
    }

    /**
     * Copies [source] to [sink], feeding every byte copied to [digest]. A
     * source with more than [limit] bytes fails with an [IOException] before
     * the byte over the limit is written, so what a stream that never ends
     * (a provider that streams without end, a device file) can put into the
     * app's storage is bounded.
     */
    internal fun copyLimited(
        source: InputStream,
        sink: OutputStream,
        digest: MessageDigest,
        limit: Long = MAX_FILE_BYTES,
    ) {
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val read = source.read(buffer)
            if (read < 0) break
            total += read
            if (total > limit) throw IOException("more than $limit bytes")
            digest.update(buffer, 0, read)
            sink.write(buffer, 0, read)
        }
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
