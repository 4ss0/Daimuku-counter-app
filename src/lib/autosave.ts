// The automatic save (Android). After every change to the user's data - a
// session saved or corrected, the voice recordings, the milestones and the
// other preferences - the whole backup is written again to a file the user
// chose once in the system "save" screen (on the phone or on Drive). On a
// new install the first-start screen can load it back (Welcome.svelte).
import { get, writable } from 'svelte/store';
import { api } from './api';
import { onDataChange } from './changes';
import { liveRunning } from './live';
import { autosaveChoose, autosaveInfo, autosaveOff, autosaveWrite, canAutosave, type AutosaveFile } from './native';

export interface AutosaveState {
  /** A file is chosen: every change is saved there. */
  on: boolean;
  /** Its name. */
  name: string;
  /** When it was last written (ms), if known. */
  saved: number | null;
  /** The last save did not work (the file was deleted, the permission is gone...). */
  failed: boolean;
}

/** Name suggested for the file, the same in every language so it is easy to find again. */
export const AUTOSAVE_NAME = 'daimoku-counter-autosave.json';

const OFF: AutosaveState = { on: false, name: '', saved: null, failed: false };
/** Changes close together make a single save. */
const DELAY_MS = 2000;
/** While a session is counting the save waits (its end is a change anyway). */
const LIVE_RETRY_MS = 15_000;

export const autosave = writable<AutosaveState>({ ...OFF });

let started = false;
let timer: ReturnType<typeof setTimeout> | null = null;
let running = false;
let again = false;

function show(f: AutosaveFile | null, failed = false) {
  autosave.set(f ? { on: true, name: f.name, saved: f.modified, failed: failed || !f.ok } : { ...OFF });
}

/** Reads the state from the Android side and starts listening for changes. */
export async function initAutosave() {
  if (started || !canAutosave()) return;
  started = true;
  show(await autosaveInfo());
  onDataChange(schedule);
  // leaving the app: what is waiting is saved right away
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden' && timer) {
      clearTimeout(timer);
      void save();
    }
  });
}

function schedule() {
  if (!get(autosave).on) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(save, DELAY_MS);
}

/** Writes the backup to the chosen file now. */
export async function save() {
  timer = null;
  if (!get(autosave).on) return;
  if (get(liveRunning)) {
    timer = setTimeout(save, LIVE_RETRY_MS);
    return;
  }
  if (running) {
    again = true;
    return;
  }
  running = true;
  try {
    const f = await api.prepareAutosave();
    const r = await autosaveWrite(f.path);
    show(r.file, !r.ok);
  } catch {
    autosave.update((s) => ({ ...s, failed: true }));
  } finally {
    running = false;
    if (again) {
      again = false;
      schedule();
    }
  }
}

/** The "save" screen: where to save from now on; then a first save. False if cancelled. */
export async function chooseAutosave(): Promise<boolean> {
  const f = await autosaveChoose(AUTOSAVE_NAME);
  if (!f) return false;
  show(f);
  await save();
  return true;
}

/** After a save was loaded on the first start, the app keeps saving to that file. */
export function useAutosave(f: AutosaveFile | null) {
  show(f);
}

export function stopAutosave() {
  autosaveOff();
  if (timer) clearTimeout(timer);
  timer = null;
  show(null);
}
