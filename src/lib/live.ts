// State of the live counting session that must survive page changes:
// the session runs in the Rust audio engine and the Android service, so
// leaving the Count screen must not stop it.
import { get, writable } from 'svelte/store';
import { api, fmtNum } from '$lib/api';
import { locale, t } from '$lib/i18n';
import { updateCounting } from '$lib/native';

/** True from Start to Stop (module state: it outlives the pages). */
export const liveRunning = writable(false);

// While the Count page is not shown nobody polls the engine, so keep the
// notification count fresh from here.
let notifier: ReturnType<typeof setInterval> | null = null;
liveRunning.subscribe((on) => {
  if (on && !notifier) {
    notifier = setInterval(async () => {
      try {
        const v = await api.liveView();
        if (!get(liveRunning)) return;
        updateCounting(get(t)('notif.count', { n: fmtNum(v.count ?? 0, get(locale)) }));
      } catch {
        /* the next tick retries */
      }
    }, 1000);
  } else if (!on && notifier) {
    clearInterval(notifier);
    notifier = null;
  }
});
