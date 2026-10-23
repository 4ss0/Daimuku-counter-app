// State of the live counting session that must survive page changes:
// the session runs in the Rust audio engine and the Android service, so
// leaving the Count screen must not stop it.
import { get, writable } from 'svelte/store';
import { api, fmtNum } from '$lib/api';
import { locale, t } from '$lib/i18n';
import { updateCounting } from '$lib/native';

/** True from Start to Stop (module state: it outlives the pages). */
export const liveRunning = writable(false);

// The notification is updated from here only, whatever page is shown.
// Android drops the updates of an app that posts more than about 5 per
// second: the Count page used to post one at every poll (150 ms), and with
// the app open the notification stayed frozen. Now it is posted at most
// once a second, when the count changes, plus a periodic refresh in case
// an update was lost.
const NOTIFY_EVERY_MS = 1000;
const REFRESH_MS = 10_000;

let notifier: ReturnType<typeof setInterval> | null = null;
let lastText = '';
let lastPostedAt = 0;

async function notifyNow() {
  try {
    const v = await api.liveView();
    if (!get(liveRunning)) return;
    const text = get(t)('notif.count', { n: fmtNum(v.count ?? 0, get(locale)) });
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
    notifier = setInterval(notifyNow, NOTIFY_EVERY_MS);
    document.addEventListener('visibilitychange', onVisibility);
  } else if (!on && notifier) {
    clearInterval(notifier);
    notifier = null;
    document.removeEventListener('visibilitychange', onVisibility);
  }
});
