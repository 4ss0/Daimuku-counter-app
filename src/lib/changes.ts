// "The user's data has changed": a session saved or corrected, the voice
// recordings, the preferences (milestones, goals...). Told by api.ts and
// prefs.ts, heard by the automatic save (autosave.ts).

const listeners = new Set<() => void>();

/** Calls `fn` after every change; returns the function that stops it. */
export function onDataChange(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function dataChanged() {
  for (const fn of listeners) fn();
}
