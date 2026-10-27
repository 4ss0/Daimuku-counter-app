package com.daimuku.counter

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.util.concurrent.Executors

/**
 * The automatic save (src/lib/autosave.ts): a file the user picked once in
 * the system "save" or "open" screen (on the phone or on Drive), kept with a
 * permission that lasts across restarts, which the app writes again after
 * every change. The backup itself is built by the Rust side; here it is
 * only copied there, or read back on a new install.
 */
class Autosave(private val context: Context) {
  private val store = context.getSharedPreferences("autosave", Context.MODE_PRIVATE)
  /** One write at a time, off the page's thread. */
  private val writer = Executors.newSingleThreadExecutor()

  /** The chosen file, if any. */
  fun target(): Uri? = store.getString(KEY_URI, null)?.let { Uri.parse(it) }

  /**
   * Keeps the permission on `uri` that the "save"/"open" screen granted
   * (`flags` of its result). False if it cannot be kept or does not allow
   * writing.
   */
  fun hold(uri: Uri, flags: Int): Boolean {
    val take = flags and (Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
    if (take and Intent.FLAG_GRANT_WRITE_URI_PERMISSION == 0) return false
    return try {
      context.contentResolver.takePersistableUriPermission(uri, take)
      true
    } catch (e: Exception) {
      false
    }
  }

  /** Lets go of a permission kept with `hold`. */
  fun release(uri: Uri) {
    try {
      context.contentResolver.releasePersistableUriPermission(
        uri,
        Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
      )
    } catch (e: Exception) {
      // already gone
    }
  }

  /** From now on the saves go to `uri` (already held); the previous file is let go. */
  fun use(uri: Uri) {
    target()?.let { if (it != uri) release(it) }
    store.edit().putString(KEY_URI, uri.toString()).apply()
  }

  /** No more automatic saves. */
  fun forget() {
    target()?.let { release(it) }
    store.edit().remove(KEY_URI).apply()
  }

  /**
   * The chosen file as JSON {name, modified, ok}: `modified` in ms (or
   * null), `ok` false if it can no longer be written (deleted, permission
   * gone). Null without a file.
   */
  fun info(): String? {
    val uri = target() ?: return null
    var name = uri.lastPathSegment ?: ""
    var modified: Long? = null
    val allowed = context.contentResolver.persistedUriPermissions.any { it.uri == uri && it.isWritePermission }
    var found = false
    if (allowed) {
      try {
        context.contentResolver.query(uri, null, null, null, null)?.use { c ->
          if (c.moveToFirst()) {
            found = true
            val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (n >= 0 && !c.isNull(n)) name = c.getString(n)
            val m = c.getColumnIndex(DocumentsContract.Document.COLUMN_LAST_MODIFIED)
            if (m >= 0 && !c.isNull(m)) modified = c.getLong(m)
          }
        }
      } catch (e: Exception) {
        found = false
      }
    }
    return JSONObject()
      .put("name", name)
      .put("modified", modified ?: JSONObject.NULL)
      .put("ok", allowed && found)
      .toString()
  }

  /** Copies `src` over the chosen file, then calls `done` (on the writer's thread). */
  fun write(src: File, done: (Boolean) -> Unit) {
    writer.execute {
      val uri = target()
      val ok = uri != null && try {
        copy(src, uri)
        true
      } catch (e: Exception) {
        false
      }
      done(ok)
    }
  }

  private fun copy(src: File, uri: Uri) {
    val resolver = context.contentResolver
    // "wt" empties the file first; a few providers only know "w"
    val out = try {
      resolver.openOutputStream(uri, "wt")
    } catch (e: Exception) {
      null
    } ?: resolver.openOutputStream(uri, "w") ?: throw IOException("cannot write $uri")
    out.use { o -> src.inputStream().use { it.copyTo(o) } }
  }

  /** The text of the file at `uri`; null if it cannot be read or is too large for a backup. */
  fun read(uri: Uri): String? {
    return try {
      context.contentResolver.openInputStream(uri)?.use { input ->
        val buf = ByteArrayOutputStream()
        val chunk = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
          val n = input.read(chunk)
          if (n < 0) break
          total += n
          if (total > MAX_BYTES) return null
          buf.write(chunk, 0, n)
        }
        buf.toString("UTF-8")
      }
    } catch (e: Exception) {
      null
    }
  }

  companion object {
    private const val KEY_URI = "uri"
    /** Bigger than any backup (a few MB); the page could not take more. */
    private const val MAX_BYTES = 64L * 1024 * 1024

    /** Where the "save"/"open" screens start: the Documents folder of the phone. */
    val DOCUMENTS: Uri = DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", "primary:Documents")
  }
}
