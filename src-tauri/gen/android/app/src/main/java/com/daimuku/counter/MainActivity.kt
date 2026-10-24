package com.daimuku.counter

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.io.File

/**
 * Exposes `window.DaimokuAndroid` to the web page (src/lib/native.ts):
 * counting service, keep-screen-on, "save as" and "share" screens,
 * reminders and donations. Replies go back through `window.__daimokuNative(event, ok, data)`.
 */
class MainActivity : TauriActivity() {
  private var webView: WebView? = null
  private lateinit var donations: Donations

  /** File waiting for the place the user picks in the "save as" screen. */
  private var toSave: File? = null
  /** Event to send when the share sheet closes ("share" or "shareText"). */
  private var shareEvent = "share"

  private val askPermissions = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { }

  private val saveAs = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
    val src = toSave
    toSave = null
    val uri = result.data?.data
    var ok = false
    if (result.resultCode == RESULT_OK && src != null && uri != null) {
      ok = try {
        contentResolver.openOutputStream(uri)?.use { out -> src.inputStream().use { it.copyTo(out) } } != null
      } catch (e: Exception) {
        false
      }
    }
    notifyPage("save", ok)
  }

  // the share sheet does not tell whether something was sent
  private val shareSheet = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) {
    notifyPage(shareEvent, true)
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    donations = Donations(this) { ev, ok, data -> notifyPage(ev, ok, data) }
    askMissingPermissions()
  }

  override fun onDestroy() {
    donations.end()
    super.onDestroy()
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
    webView.addJavascriptInterface(Bridge(), "DaimokuAndroid")
  }

  private fun askMissingPermissions() {
    val wanted = mutableListOf(Manifest.permission.RECORD_AUDIO)
    if (Build.VERSION.SDK_INT >= 33) wanted.add(Manifest.permission.POST_NOTIFICATIONS)
    val missing = wanted.filter { ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED }
    if (missing.isNotEmpty()) askPermissions.launch(missing.toTypedArray())
  }

  private fun notifyPage(event: String, ok: Boolean, data: String? = null) {
    val args = listOf(JSONObject.quote(event), ok.toString(), data?.let { JSONObject.quote(it) } ?: "null")
    val js = "window.__daimokuNative && window.__daimokuNative(${args.joinToString(",")})"
    runOnUiThread { webView?.evaluateJavascript(js, null) }
  }

  /** A content:// address for a file of the app, copied where the FileProvider serves it. */
  private fun shareableUri(src: File, name: String): Uri {
    val dir = File(cacheDir, "shared").apply { mkdirs() }
    dir.listFiles()?.forEach { it.delete() }
    val copy = File(dir, name)
    src.copyTo(copy, overwrite = true)
    return FileProvider.getUriForFile(this, "$packageName.fileprovider", copy)
  }

  /** Methods called by the page (on a background thread). */
  inner class Bridge {
    @JavascriptInterface
    fun startCounting(title: String, text: String) {
      val intent = Intent(this@MainActivity, CountingService::class.java)
        .putExtra(CountingService.EXTRA_TITLE, title)
        .putExtra(CountingService.EXTRA_TEXT, text)
      try {
        ContextCompat.startForegroundService(this@MainActivity, intent)
      } catch (e: Exception) {
        // counting still works while the app is open
      }
    }

    @JavascriptInterface
    fun updateCounting(title: String) {
      CountingService.update(this@MainActivity, title)
    }

    @JavascriptInterface
    fun countFrom(base: Int, template: String, locale: String) {
      CountingService.countFrom(base, template, locale)
    }

    @JavascriptInterface
    fun stopCounting() {
      stopService(Intent(this@MainActivity, CountingService::class.java))
    }

    @JavascriptInterface
    fun keepScreenOn(on: Boolean) {
      runOnUiThread {
        if (on) window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
      }
    }

    @JavascriptInterface
    fun saveFile(path: String, name: String, mime: String) {
      runOnUiThread {
        toSave = File(path)
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
          .addCategory(Intent.CATEGORY_OPENABLE)
          .setType(mime)
          .putExtra(Intent.EXTRA_TITLE, name)
        try {
          saveAs.launch(intent)
        } catch (e: Exception) {
          toSave = null
          notifyPage("save", false)
        }
      }
    }

    @JavascriptInterface
    fun shareFile(path: String, name: String, mime: String, title: String) {
      runOnUiThread {
        try {
          val uri = shareableUri(File(path), name)
          val send = Intent(Intent.ACTION_SEND)
            .setType(mime)
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
          shareEvent = "share"
          shareSheet.launch(Intent.createChooser(send, title))
        } catch (e: Exception) {
          notifyPage("share", false)
        }
      }
    }

    @JavascriptInterface
    fun shareText(text: String, title: String) {
      runOnUiThread {
        try {
          val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, text)
          shareEvent = "shareText"
          shareSheet.launch(Intent.createChooser(send, title))
        } catch (e: Exception) {
          notifyPage("shareText", false)
        }
      }
    }

    /** Daily reminder "morning" or "evening" at hour:minute (on = false cancels it). */
    @JavascriptInterface
    fun setReminder(kind: String, on: Boolean, hour: Int, minute: Int, title: String, text: String) {
      Reminders.set(this@MainActivity, kind, on, hour, minute, title, text)
    }

    /** Comma-separated product ids; replies with event "products". */
    @JavascriptInterface
    fun donationProducts(ids: String) {
      donations.products(ids.split(',').map { it.trim() }.filter { it.isNotEmpty() })
    }

    /** Replies with event "donate". */
    @JavascriptInterface
    fun donate(productId: String) {
      donations.donate(productId)
    }
  }
}
