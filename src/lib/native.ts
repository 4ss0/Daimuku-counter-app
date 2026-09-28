// Bridge to the Android side (MainActivity.kt): counting with the screen
// off, keep-screen-on, and the system "save as" / "share" screens.
// On desktop there is no bridge and every call is a harmless no-op.

export interface ExportedFile {
  path: string;
  name: string;
}

interface Bridge {
  startCounting(title: string, text: string): void;
  stopCounting(): void;
  keepScreenOn(on: boolean): void;
  saveFile(path: string, name: string, mime: string): void;
  shareFile(path: string, name: string, mime: string, title: string): void;
}

function bridge(): Bridge | undefined {
  return typeof window !== 'undefined' ? (window as unknown as { DaimokuAndroid?: Bridge }).DaimokuAndroid : undefined;
}

/** True inside the Android app. */
export function hasNative(): boolean {
  return !!bridge();
}

const waiters: Record<string, (ok: boolean) => void> = {};
if (typeof window !== 'undefined') {
  (window as unknown as { __daimokuNative: (ev: string, ok: boolean) => void }).__daimokuNative = (ev, ok) => {
    const w = waiters[ev];
    delete waiters[ev];
    w?.(ok);
  };
}
function waitFor(ev: string): Promise<boolean> {
  return new Promise((resolve) => {
    waiters[ev]?.(false);
    waiters[ev] = resolve;
  });
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

export function startCounting(title: string, text: string, keepScreen: boolean) {
  safe((b) => b.startCounting(title, text));
  safe((b) => b.keepScreenOn(keepScreen));
}

export function stopCounting() {
  safe((b) => b.stopCounting());
  safe((b) => b.keepScreenOn(false));
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
