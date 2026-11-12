package com.daimuku.counter

import android.content.ContentValues
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.AtomicFile
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

  private fun enabled() = !store.getBoolean("disabled", false)
  private fun localFile() = File(context.filesDir, DEFAULT_NAME)

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
    store.edit().putString(KEY_URI, uri.toString()).putBoolean("owned", false).putBoolean("disabled", false).apply()
  }

  /** No more automatic saves. */
  fun forget() {
    target()?.let { release(it) }
    store.edit().remove(KEY_URI).remove("owned").putBoolean("disabled", true).apply()
  }

  /**
   * The chosen file as JSON {name, modified, ok}: `modified` in ms (or
   * null), `ok` false if it can no longer be written (deleted, permission
   * gone). Null without a file.
   */
  fun info(): String? {
    if (!enabled()) return null
    val uri = target()
    if (uri == null) {
      val publicDefault = Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q
      val file = localFile()
      return JSONObject().put("name", if (publicDefault) "Download/Daimoku Counter/$DEFAULT_NAME" else file.absolutePath)
        .put("modified", if (!publicDefault && file.exists()) file.lastModified() else JSONObject.NULL)
        .put("ok", true).toString()
    }
    var name = uri.lastPathSegment ?: ""
    var modified: Long? = null
    val allowed = store.getBoolean("owned", false) || context.contentResolver.persistedUriPermissions.any { it.uri == uri && it.isWritePermission }
    var found = false
    if (allowed) {
      try {
        context.contentResolver.query(uri, null, null, null, null)?.use { c ->
          if (c.moveToFirst()) {
            found = true
            val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (n >= 0 && !c.isNull(n)) name = c.getString(n)
            val owned = store.getBoolean("owned", false)
            val m = c.getColumnIndex(if (owned) MediaStore.MediaColumns.DATE_MODIFIED else DocumentsContract.Document.COLUMN_LAST_MODIFIED)
            if (m >= 0 && !c.isNull(m)) modified = c.getLong(m) * (if (owned) 1000L else 1L)
            if (owned) name = "Download/Daimoku Counter/$name"
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
      val ok = try {
        if (!enabled()) throw IOException("autosave disabled")
        if (uri != null) copy(src, uri)
        else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) createDefault(src)
        else {
          // Android 8/9 require user selection for shared storage. Keep a
          // local atomic copy until the user chooses a destination.
          val file = AtomicFile(localFile())
          val output = file.startWrite()
          try {
            src.inputStream().use { it.copyTo(output) }
            file.finishWrite(output)
          } catch (e: Exception) {
            file.failWrite(output)
            throw e
          }
        }
        true
      } catch (e: Exception) {
        false
      }
      done(ok)
    }
  }

  @android.annotation.TargetApi(Build.VERSION_CODES.Q)
  private fun createDefault(src: File) {
    val resolver = context.contentResolver
    val values = ContentValues().apply {
      put(MediaStore.MediaColumns.DISPLAY_NAME, DEFAULT_NAME)
      put(MediaStore.MediaColumns.MIME_TYPE, "application/json")
      put(MediaStore.MediaColumns.RELATIVE_PATH, "${Environment.DIRECTORY_DOWNLOADS}/Daimoku Counter")
      put(MediaStore.MediaColumns.IS_PENDING, 1)
    }
    val uri = resolver.insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI, values)
      ?: throw IOException("cannot create automatic backup")
    try {
      copy(src, uri)
      resolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
      store.edit().putString(KEY_URI, uri.toString()).putBoolean("owned", true).apply()
    } catch (e: Exception) {
      resolver.delete(uri, null, null)
      throw e
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
    private const val DEFAULT_NAME = "daimoku-counter-autosave.json"
    /** Bigger than any backup (a few MB); the page could not take more. */
    private const val MAX_BYTES = 64L * 1024 * 1024

    /** Where the "save"/"open" screens start: the Documents folder of the phone. */
    val DOCUMENTS: Uri = DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", "primary:Documents")
  }
}
