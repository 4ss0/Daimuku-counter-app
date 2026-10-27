// The goals the user chants for (menu "Goals"): written freely, with a
// slider for how far along they feel, and closed when reached or when it
// has become clear that a better way exists. Kept in the preferences, so
// they are also in the backups and in the automatic save.
import { get } from 'svelte/store';
import { MAX_INTENT_TEXT, prefs, updatePrefs, type Intent, type IntentReason } from './prefs';

function change(fn: (list: Intent[]) => Intent[]) {
  return updatePrefs({ intents: fn(get(prefs).intents) });
}

function update(id: string, patch: (g: Intent) => Intent) {
  return change((list) => list.map((g) => (g.id === id ? patch(g) : g)));
}

export function addIntent(text: string) {
  const clean = text.trim().slice(0, MAX_INTENT_TEXT);
  if (!clean) return;
  const id = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  return change((list) => [...list, { id, text: clean, progress: 0, created: new Date().toISOString() }]);
}

export function editIntent(id: string, text: string) {
  const clean = text.trim().slice(0, MAX_INTENT_TEXT);
  if (!clean) return;
  return update(id, (g) => ({ ...g, text: clean }));
}

/** How far along, 0-100. */
export function setProgress(id: string, progress: number) {
  const p = Math.min(100, Math.max(0, Math.round(progress)));
  return update(id, (g) => ({ ...g, progress: p }));
}

export function closeIntent(id: string, reason: IntentReason) {
  return update(id, (g) => ({ ...g, closed: new Date().toISOString(), reason }));
}

export function reopenIntent(id: string) {
  return update(id, (g) => {
    const open = { ...g };
    delete open.closed;
    delete open.reason;
    return open;
  });
}

export function deleteIntent(id: string) {
  return change((list) => list.filter((g) => g.id !== id));
}
