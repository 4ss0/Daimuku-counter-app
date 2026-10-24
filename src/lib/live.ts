// State of the live counting session that must survive page changes:
// the session runs in the Rust audio engine and the Android service, so
// leaving the Count screen must not stop it.
import { get, writable } from 'svelte/store';
import { api, fmtNum } from '$lib/api';
import { locale, t } from '$lib/i18n';
import { countInNotification, keepScreen, updateCounting } from '$lib/native';
import { prefs } from '$lib/prefs';

/** True from Start to Stop (module state: it outlives the pages). */
export const liveRunning = writable(false);

/** The session of the day the running one continues (chosen on the Count page). */
export const liveHalf = writable<'morning' | 'evening'>('morning');

/** That session's Daimoku of today before the running one started. */
export const liveBase = writable(0);

/**
 * The number shown while a session runs, as on the Count page: `liveBase`
 * plus the engine's count, refreshed every second.
 */
export const liveCount = writable(0);

// The notification shows the count. On Android the counting service
// writes it by itself, reading the engine (CountingService.kt): with the
// screen locked the page's timers are slowed down to one a minute and the
// number used to stop. An older APK cannot, and the page posts it from
// here, whatever page is shown: at most once a second (Android drops the
// updates of an app that posts more than about 5 a second), when the count
// changes, plus a periodic refresh in case an update was lost.
const NOTIFY_EVERY_MS = 1000;
const REFRESH_MS = 10_000;

let notifier: ReturnType<typeof setInterval> | null = null;
let nativeNotifies = false;
let lastText = '';
let lastPostedAt = 0;

async function notifyNow() {
  try {
    const v = await api.liveView();
    if (!get(liveRunning)) return;
    const n = get(liveBase) + (v.count ?? 0);
    liveCount.set(n);
    if (nativeNotifies) return;
    const text = get(t)('notif.count', { n: fmtNum(n, get(locale)) });
    const now = Date.now();
    if (text === lastText && now - lastPostedAt < REFRESH_MS) return;
    lastText = text;
    lastPostedAt = now;
    updateCounting(text);
  } catch {
    /* the next tick retries */
  }
}

// back from the screen off: the timers may have been slowed down, so bring
// the notification up to date at once
function onVisibility() {
  if (!document.hidden) notifyNow();
}

liveRunning.subscribe((on) => {
  if (on && !notifier) {
    lastText = '';
    lastPostedAt = 0;
    liveCount.set(get(liveBase));
    nativeNotifies = countInNotification(get(liveBase), get(t)('notif.count', { n: '{n}' }), get(locale));
    notifier = setInterval(notifyNow, NOTIFY_EVERY_MS);
    document.addEventListener('visibilitychange', onVisibility);
  } else if (!on && notifier) {
    clearInterval(notifier);
    notifier = null;
    document.removeEventListener('visibilitychange', onVisibility);
  }
});

// "Keep the screen on while counting" (Settings) holds for the whole
// session, whatever page is shown, and a change applies at once.
let running = false;
let keepAwake = false;
liveRunning.subscribe((on) => {
  running = on;
  keepScreen('counting', running && keepAwake);
});
prefs.subscribe((p) => {
  keepAwake = p.keepAwake;
  keepScreen('counting', running && keepAwake);
});
