// Bridge to the Android side (MainActivity.kt): counting with the screen
// off, keep-screen-on, the system "save as" / "share" screens, donations
// through Google Play and the automatic save.
// On desktop there is no bridge and every call is a harmless no-op.

export interface ExportedFile {
  path: string;
  name: string;
}

interface Bridge {
  startCounting(title: string, text: string): void;
  updateCounting(title: string): void;
  stopCounting(): void;
  countFrom?(base: number, template: string, locale: string): void;
  keepScreenOn(on: boolean): void;
  saveFile(path: string, name: string, mime: string): void;
  shareFile(path: string, name: string, mime: string, title: string): void;
  shareText(text: string, title: string): void;
  setReminder?(kind: string, on: boolean, hour: number, minute: number, title: string, text: string): void;
  donationProducts?(ids: string): void;
  donate?(productId: string): void;
  autosaveInfo?(): void;
  autosaveChoose?(name: string): void;
  autosaveOpen?(): void;
  autosaveKeep?(keep: boolean): void;
  autosaveWrite?(path: string): void;
  autosaveOff?(): void;
}

function bridge(): Bridge | undefined {
  return typeof window !== 'undefined' ? (window as unknown as { DaimokuAndroid?: Bridge }).DaimokuAndroid : undefined;
}

/** True inside the Android app. */
export function hasNative(): boolean {
  return !!bridge();
}

type Reply = { ok: boolean; data: string | null };
const waiters: Record<string, (r: Reply) => void> = {};
if (typeof window !== 'undefined') {
  (window as unknown as { __daimokuNative: (ev: string, ok: boolean, data?: string | null) => void }).__daimokuNative = (
    ev,
    ok,
    data,
  ) => {
    const w = waiters[ev];
    delete waiters[ev];
    w?.({ ok, data: data ?? null });
  };
}
/** The next reply to `ev` (a newer wait for the same event cancels this one). */
function reply(ev: string): Promise<Reply> {
  return new Promise((resolve) => {
    waiters[ev]?.({ ok: false, data: null });
    waiters[ev] = resolve;
  });
}
function waitFor(ev: string): Promise<boolean> {
  return reply(ev).then((r) => r.ok);
}

function safe(fn: (b: Bridge) => void) {
  const b = bridge();
  if (!b) return;
  try {
    fn(b);
  } catch {
    /* an old APK without the method: ignore */
  }
}

export function startCounting(title: string, text: string) {
  safe((b) => b.startCounting(title, text));
}

/** Android: updates the notification title (the current count). */
export function updateCounting(title: string) {
  safe((b) => b.updateCounting(title));
}

/**
 * Android: the counting service writes the count in the notification by
 * itself, reading it from the engine: `template` with `{n}` replaced by
 * `base` plus the engine's count. False on an older APK, where the page
 * must keep the notification up to date (`updateCounting`).
 */
export function countInNotification(base: number, template: string, locale: string): boolean {
  const b = bridge();
  if (typeof b?.countFrom !== 'function') return false;
  try {
    b.countFrom(base, template, locale);
    return true;
  } catch {
    return false;
  }
}

export function stopCounting() {
  safe((b) => b.stopCounting());
}

/** Android: lets the user pick where to save the file. Resolves false if cancelled. */
export function saveFile(f: ExportedFile, mime: string): Promise<boolean> {
  const b = bridge();
  if (!b) return Promise.resolve(false);
  const p = waitFor('save');
  try {
    b.saveFile(f.path, f.name, mime);
  } catch {
    return Promise.resolve(false);
  }
  return p;
}

/**
 * Shares a short text: the Android share sheet, the system share dialog
 * where the platform has one, otherwise the clipboard.
 * Resolves 'shared', 'copied' or 'failed'.
 */
export async function shareText(text: string, title: string): Promise<'shared' | 'copied' | 'failed'> {
  const b = bridge();
  if (b) {
    const p = waitFor('shareText');
    try {
      b.shareText(text, title);
    } catch {
      return 'failed';
    }
    return (await p) ? 'shared' : 'failed';
  }
  const nav = typeof navigator !== 'undefined' ? navigator : undefined;
  if (nav?.share) {
    try {
      await nav.share({ title, text });
      return 'shared';
    } catch {
      /* cancelled or unsupported: fall back to the clipboard */
    }
  }
  try {
    await nav?.clipboard?.writeText(text);
    return 'copied';
  } catch {
    return 'failed';
  }
}

/** Android: opens the share sheet (Drive, e-mail, messaging apps...). */
export function shareFile(f: ExportedFile, mime: string, title: string): Promise<boolean> {
  const b = bridge();
  if (!b) return Promise.resolve(false);
  const p = waitFor('share');
  try {
    b.shareFile(f.path, f.name, mime, title);
  } catch {
    return Promise.resolve(false);
  }
  return p;
}

// ---------------------------------------------------------------- reminders

/** True inside an Android app that can show reminders (older APKs cannot). */
export function canRemind(): boolean {
  return typeof bridge()?.setReminder === 'function';
}

/** Android: the daily reminder `kind` at `time` ("HH:MM"), or off. */
export function setReminder(kind: 'morning' | 'evening', on: boolean, time: string, title: string, text: string) {
  const [h, m] = time.split(':').map((x) => parseInt(x, 10));
  safe((b) => b.setReminder?.(kind, on, Number.isFinite(h) ? h : 7, Number.isFinite(m) ? m : 0, title, text));
}

// ---------------------------------------------------------------- donations

export interface DonationProduct {
  id: string;
  /** Price in the user's currency, formatted by Google Play. */
  price: string;
}

export type DonationResult = 'thanks' | 'pending' | 'cancelled' | 'unavailable' | 'error';

const PRODUCTS_TIMEOUT_MS = 15_000;

/** True inside an Android app that can take donations (older APKs cannot). */
export function canDonate(): boolean {
  return typeof bridge()?.donate === 'function';
}

/** The products Google Play knows, with their prices; null if Play is not available. */
export async function donationProducts(ids: string[]): Promise<DonationProduct[] | null> {
  const b = bridge();
  if (!b?.donationProducts) return null;
  const p = reply('products');
  try {
    b.donationProducts(ids.join(','));
  } catch {
    return null;
  }
  // Play may never answer (no Play Store, no network): give up after a while
  const r = await Promise.race([p, new Promise<Reply>((res) => setTimeout(() => res({ ok: false, data: null }), PRODUCTS_TIMEOUT_MS))]);
  if (!r.ok || !r.data) return null;
  try {
    return (JSON.parse(r.data) as DonationProduct[]).filter((x) => ids.includes(x.id));
  } catch {
    return null;
  }
}

/** Opens the Google Play payment sheet; resolves when it closes. */
export async function donate(productId: string): Promise<DonationResult> {
  const b = bridge();
  if (!b?.donate) return 'unavailable';
  const p = reply('donate');
  try {
    b.donate(productId);
  } catch {
    return 'error';
  }
  const r = await p;
  const d = r.data as DonationResult | null;
  return d && ['thanks', 'pending', 'cancelled', 'unavailable', 'error'].includes(d) ? d : r.ok ? 'thanks' : 'error';
}

// ---------------------------------------------------------- automatic save

/** The file of the automatic save (MainActivity / Autosave.kt). */
export interface AutosaveFile {
  name: string;
  /** Last written, in ms, if the place where it is says so. */
  modified: number | null;
  /** False if it can no longer be written (deleted, permission gone). */
  ok: boolean;
}

function autosaveFile(data: string | null): AutosaveFile | null {
  if (!data) return null;
  try {
    const f = JSON.parse(data) as Partial<AutosaveFile>;
    return { name: String(f.name ?? ''), modified: typeof f.modified === 'number' ? f.modified : null, ok: f.ok === true };
  } catch {
    return null;
  }
}

/** True inside an Android app that can save by itself (older APKs cannot). */
export function canAutosave(): boolean {
  return typeof bridge()?.autosaveWrite === 'function';
}

/** Calls `fn` on the bridge and waits for the reply `ev` (a failed call: no reply). */
async function ask(ev: string, fn: (b: Bridge) => void): Promise<Reply> {
  const b = bridge();
  if (!b) return { ok: false, data: null };
  const p = reply(ev);
  try {
    fn(b);
  } catch {
    return { ok: false, data: null };
  }
  return p;
}

/** The file chosen for the automatic save, or null. */
export async function autosaveInfo(): Promise<AutosaveFile | null> {
  return autosaveFile((await ask('autosaveInfo', (b) => b.autosaveInfo?.())).data);
}

/** The system "save" screen, suggesting `name`: the file where to save from now on. Null if cancelled. */
export async function autosaveChoose(name: string): Promise<AutosaveFile | null> {
  const r = await ask('autosaveChoose', (b) => b.autosaveChoose?.(name));
  return r.ok ? autosaveFile(r.data) : null;
}

/** The system "open" screen: the text of a save to load, or null if cancelled. Throws if the file cannot be read. */
export async function autosaveOpen(): Promise<string | null> {
  const r = await ask('autosaveOpen', (b) => b.autosaveOpen?.());
  if (r.ok) return r.data;
  if (r.data === 'unreadable') throw new Error('unreadable');
  return null;
}

/** After `autosaveOpen`: keep saving to that file (its details), or let it go. */
export async function autosaveKeep(keep: boolean): Promise<AutosaveFile | null> {
  const r = await ask('autosaveKeep', (b) => b.autosaveKeep?.(keep));
  return r.ok ? autosaveFile(r.data) : null;
}

/** Copies the backup written at `path` to the chosen file. */
export async function autosaveWrite(path: string): Promise<{ ok: boolean; file: AutosaveFile | null }> {
  const r = await ask('autosaveWrite', (b) => b.autosaveWrite?.(path));
  return { ok: r.ok, file: autosaveFile(r.data) };
}

/** No more automatic saves. */
export function autosaveOff() {
  safe((b) => b.autosaveOff?.());
}

// The screen is kept on while counting (if chosen in Settings) and while
// Gongyo plays. Each asks under its own name, so that one letting go does
// not turn the screen off under the other.
const awake = new Set<'counting' | 'gongyo'>();

/** Keeps the screen on (Android) for `reason`, or stops keeping it for it. */
export function keepScreen(reason: 'counting' | 'gongyo', on: boolean) {
  if (on) awake.add(reason);
  else awake.delete(reason);
  safe((b) => b.keepScreenOn(awake.size > 0));
}
