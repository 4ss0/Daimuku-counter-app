package com.daimuku.counter

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import java.text.NumberFormat
import java.util.Locale

/**
 * Foreground service that keeps the process (and so the Rust audio engine)
 * running with the screen off, with a notification showing the count.
 * The service reads the count from the engine every second by itself
 * (`Native.count`): with the screen locked the web page's timers are slowed
 * down to one a minute, and the number in the notification used to stop.
 * The page only says how to write it (`countFrom`); an older page sends the
 * text itself (`DaimokuAndroid.updateCounting`).
 */
class CountingService : Service() {
  private var wakeLock: PowerManager.WakeLock? = null
  private val handler = Handler(Looper.getMainLooper())
  private var ticking = false
  private val tick = object : Runnable {
    override fun run() {
      refresh(this@CountingService)
      handler.postDelayed(this, TICK_MS)
    }
  }

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    intent?.getStringExtra(EXTRA_TITLE)?.let { title = it }
    intent?.getStringExtra(EXTRA_TEXT)?.let { text = it }
    try {
      ServiceCompat.startForeground(
        this,
        NOTIFICATION_ID,
        build(this),
        if (Build.VERSION.SDK_INT >= 30) ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE else 0,
      )
    } catch (e: Exception) {
      // no microphone permission, or not allowed to start from here
      stopSelf()
      return START_NOT_STICKY
    }
    if (wakeLock == null) {
      val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
      wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "daimoku:counting").apply {
        setReferenceCounted(false)
        acquire(MAX_WAKE_MS)
      }
    }
    running = true
    if (!ticking) {
      ticking = true
      handler.post(tick)
    }
    return START_NOT_STICKY
  }

  override fun onDestroy() {
    running = false
    handler.removeCallbacks(tick)
    ticking = false
    template = ""
    wakeLock?.let { if (it.isHeld) it.release() }
    wakeLock = null
    super.onDestroy()
  }

  companion object {
    const val EXTRA_TITLE = "title"
    const val EXTRA_TEXT = "text"
    private const val CHANNEL_ID = "counting"
    private const val NOTIFICATION_ID = 1
    /** A practice longer than this lets the phone sleep again. */
    private const val MAX_WAKE_MS = 12L * 60 * 60 * 1000
    private const val TICK_MS = 1000L

    @Volatile private var running = false
    @Volatile private var title = ""
    @Volatile private var text = ""
    /** The count in the notification: `template` with {n} = base + the engine's count. */
    @Volatile private var template = ""
    @Volatile private var base = 0
    @Volatile private var format: NumberFormat = NumberFormat.getIntegerInstance()

    /** How to write the count (from the page, before the service starts). */
    fun countFrom(base: Int, template: String, locale: String) {
      this.base = base
      this.format = NumberFormat.getIntegerInstance(Locale.forLanguageTag(locale))
      this.template = template
    }

    /** The count from the engine into the notification (when the page said how). */
    private fun refresh(context: Context) {
      val t = template
      if (t.isEmpty()) return
      val n = Native.count()
      if (n < 0) return
      update(context, t.replace("{n}", format.format((base + n).toLong())))
    }

    /** New title (the count): posted only when it changes. */
    fun update(context: Context, newTitle: String) {
      if (!running || newTitle == title) return
      title = newTitle
      val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
      nm.notify(NOTIFICATION_ID, build(context))
    }

    private fun build(context: Context): Notification {
      val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
      if (Build.VERSION.SDK_INT >= 26 && nm.getNotificationChannel(CHANNEL_ID) == null) {
        val ch = NotificationChannel(
          CHANNEL_ID,
          context.getString(R.string.channel_counting),
          NotificationManager.IMPORTANCE_LOW,
        )
        ch.description = context.getString(R.string.channel_counting_text)
        ch.setShowBadge(false)
        nm.createNotificationChannel(ch)
      }
      val open = PendingIntent.getActivity(
        context,
        0,
        Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
      )
      return NotificationCompat.Builder(context, CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification)
        .setContentTitle(title)
        .setContentText(text)
        .setContentIntent(open)
        .setOngoing(true)
        .setOnlyAlertOnce(true)
        .setSilent(true)
        .setShowWhen(false)
        .setCategory(NotificationCompat.CATEGORY_SERVICE)
        .setPriority(NotificationCompat.PRIORITY_LOW)
        .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
        .build()
    }
  }
}
