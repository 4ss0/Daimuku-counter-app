package com.daimuku.counter

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.core.app.NotificationCompat
import java.util.Calendar

/**
 * Daily reminders for the morning and the evening session. The page sets
 * them (`DaimokuAndroid.setReminder`); they are kept in SharedPreferences
 * and scheduled with AlarmManager, again after each one fires and after a
 * reboot or a change of time (`BootReceiver`).
 *
 * The alarms are inexact (`setAndAllowWhileIdle`): exact ones need a
 * permission the user must grant by hand, and a reminder a few minutes
 * late is fine.
 */
object Reminders {
  const val ACTION = "com.daimuku.counter.REMINDER"
  const val EXTRA_KIND = "kind"
  val KINDS = listOf("morning", "evening")
  private const val PREFS = "reminders"
  private const val CHANNEL_ID = "reminders"

  fun set(context: Context, kind: String, on: Boolean, hour: Int, minute: Int, title: String, text: String) {
    if (kind !in KINDS) return
    prefs(context).edit()
      .putBoolean("$kind.on", on)
      .putInt("$kind.hour", hour.coerceIn(0, 23))
      .putInt("$kind.minute", minute.coerceIn(0, 59))
      .putString("$kind.title", title)
      .putString("$kind.text", text)
      .apply()
    schedule(context, kind)
  }

  fun scheduleAll(context: Context) = KINDS.forEach { schedule(context, it) }

  /** The next alarm of `kind`, or none if it is off. */
  fun schedule(context: Context, kind: String) {
    val p = prefs(context)
    val am = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
    val pi = pending(context, kind)
    am.cancel(pi)
    if (!p.getBoolean("$kind.on", false)) return
    val next = Calendar.getInstance().apply {
      set(Calendar.HOUR_OF_DAY, p.getInt("$kind.hour", 7))
      set(Calendar.MINUTE, p.getInt("$kind.minute", 0))
      set(Calendar.SECOND, 0)
      set(Calendar.MILLISECOND, 0)
      if (timeInMillis <= System.currentTimeMillis() + 1000) add(Calendar.DAY_OF_YEAR, 1)
    }
    am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, next.timeInMillis, pi)
  }

  fun show(context: Context, kind: String) {
    val p = prefs(context)
    if (!p.getBoolean("$kind.on", false)) return
    val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    if (Build.VERSION.SDK_INT >= 26 && nm.getNotificationChannel(CHANNEL_ID) == null) {
      val ch = NotificationChannel(CHANNEL_ID, context.getString(R.string.channel_reminders), NotificationManager.IMPORTANCE_DEFAULT)
      ch.description = context.getString(R.string.channel_reminders_text)
      nm.createNotificationChannel(ch)
    }
    val open = PendingIntent.getActivity(
      context,
      0,
      Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
    val n = NotificationCompat.Builder(context, CHANNEL_ID)
      .setSmallIcon(R.drawable.ic_notification)
      .setContentTitle(p.getString("$kind.title", "") ?: "")
      .setContentText(p.getString("$kind.text", "") ?: "")
      .setContentIntent(open)
      .setAutoCancel(true)
      .setCategory(NotificationCompat.CATEGORY_REMINDER)
      .build()
    try {
      nm.notify(100 + KINDS.indexOf(kind), n)
    } catch (e: SecurityException) {
      // notifications not allowed
    }
  }

  private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

  private fun pending(context: Context, kind: String): PendingIntent =
    PendingIntent.getBroadcast(
      context,
      KINDS.indexOf(kind),
      Intent(context, ReminderReceiver::class.java).setAction(ACTION).putExtra(EXTRA_KIND, kind),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
}

/** A reminder is due: show it and schedule the next day's. */
class ReminderReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action != Reminders.ACTION) return
    val kind = intent.getStringExtra(Reminders.EXTRA_KIND) ?: return
    Reminders.show(context, kind)
    Reminders.schedule(context, kind)
  }
}

/** Alarms are lost at reboot and must follow a change of time or time zone. */
class BootReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    when (intent.action) {
      Intent.ACTION_BOOT_COMPLETED,
      Intent.ACTION_MY_PACKAGE_REPLACED,
      Intent.ACTION_TIME_CHANGED,
      Intent.ACTION_TIMEZONE_CHANGED -> Reminders.scheduleAll(context)
    }
  }
}
