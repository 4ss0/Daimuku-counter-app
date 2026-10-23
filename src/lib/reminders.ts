// Morning and evening reminders: kept in the preferences, scheduled by the
// Android side (Reminders.kt), which keeps its own copy of the times and of
// the text, in the user's language.
import { get } from 'svelte/store';
import { t } from './i18n';
import { canRemind, setReminder } from './native';
import type { Prefs } from './prefs';

export const KINDS = ['morning', 'evening'] as const;

let last = '';

/** Hands the reminders to Android whenever they, or the language of their text, change. */
export function syncReminders(p: Prefs) {
  if (!canRemind()) return;
  const tr = get(t);
  const key = JSON.stringify([p.reminders, tr('rem.notifText')]);
  if (key === last) return;
  last = key;
  for (const kind of KINDS) {
    const r = p.reminders[kind];
    setReminder(kind, r.on, r.time, tr(kind === 'morning' ? 'rem.notifMorning' : 'rem.notifEvening'), tr('rem.notifText'));
  }
}
