// Backgrounds: the original lotus, five illustrated themes (each with a
// light and a dark picture, see tools/backgrounds) or a picture of the
// user's own. That picture is scaled down and kept in the WebView's
// storage (IndexedDB) on this device only.
export const BACKGROUNDS = ['lotus', 'sutra', 'buddha', 'temple', 'nature', 'sakura', 'custom'] as const;
export type Background = (typeof BACKGROUNDS)[number];

/** Picture of a background for a theme (the custom one has none here). */
export function backgroundUrl(bg: Background, theme: 'light' | 'dark'): string | null {
  if (bg === 'custom') return null;
  if (bg === 'lotus') return theme === 'light' ? '/bg-light.jpg' : '/bg-dark.jpg';
  return `/backgrounds/${bg}-${theme}.jpg`;
}

const DB = 'daimoku';
const STORE = 'files';
const KEY = 'background';
/** Longest side of the stored picture, in pixels. */
const MAX_SIDE = 1600;

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function run<T>(mode: IDBTransactionMode, fn: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open();
  return new Promise((resolve, reject) => {
    const req = fn(db.transaction(STORE, mode).objectStore(STORE));
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

let shownUrl: string | null = null;

function show(blob: Blob | null) {
  if (shownUrl) URL.revokeObjectURL(shownUrl);
  shownUrl = blob ? URL.createObjectURL(blob) : null;
  const root = document.documentElement;
  if (shownUrl) root.style.setProperty('--bg-custom', `url("${shownUrl}")`);
  else root.style.removeProperty('--bg-custom');
}

/** Shows the stored picture, if any; true if there is one. */
export async function loadCustomBackground(): Promise<boolean> {
  try {
    const blob = await run<Blob | undefined>('readonly', (s) => s.get(KEY));
    show(blob ?? null);
    return !!blob;
  } catch {
    return false;
  }
}

/** The stored picture as an address for a preview, or null. */
export function customBackgroundUrl(): string | null {
  return shownUrl;
}

/** Scales the picture down, stores it and shows it. */
export async function setCustomBackground(file: File): Promise<void> {
  const bitmap = await createImageBitmap(file);
  const k = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * k);
  canvas.height = Math.round(bitmap.height * k);
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('encode'))), 'image/jpeg', 0.85),
  );
  await run('readwrite', (s) => s.put(blob, KEY));
  show(blob);
}
