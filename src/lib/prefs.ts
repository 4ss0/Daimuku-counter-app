// User preferences (language, theme, colour), saved by the Rust side in
// prefs.json next to the history, and applied to the document.
import { invoke } from '@tauri-apps/api/core';
import { get, writable } from 'svelte/store';
import { detectLang, lang, type Lang } from './i18n';

export type ThemePref = 'system' | 'light' | 'dark';
export type Accent = 'lotus' | 'gold' | 'sakura' | 'indigo';
export const ACCENTS: Accent[] = ['lotus', 'gold', 'sakura', 'indigo'];

export interface Prefs {
  lang: 'auto' | Lang;
  theme: ThemePref;
  accent: Accent;
  /** Keep the screen on while counting (counting also works with it off). */
  keepAwake: boolean;
}

const DEFAULTS: Prefs = { lang: 'auto', theme: 'system', accent: 'lotus', keepAwake: false };

export const prefs = writable<Prefs>({ ...DEFAULTS });
/** The theme actually shown (system preference resolved). */
export const resolvedTheme = writable<'light' | 'dark'>('dark');

let loaded = false;
let media: MediaQueryList | null = null;

function apply(p: Prefs) {
  lang.set(p.lang === 'auto' ? detectLang() : p.lang);
  const sysDark = media ? media.matches : true;
  const theme = p.theme === 'system' ? (sysDark ? 'dark' : 'light') : p.theme;
  resolvedTheme.set(theme);
  if (typeof document !== 'undefined') {
    const root = document.documentElement;
    root.dataset.theme = theme;
    root.dataset.accent = p.accent;
    root.lang = get(lang);
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute('content', theme === 'dark' ? '#0c0d0f' : '#eef2ea');
    // cache for app.html, so the next start paints the right theme immediately
    try {
      localStorage.setItem('daimoku-theme', JSON.stringify({ theme: p.theme, accent: p.accent }));
    } catch {
      /* storage unavailable: only costs a brief flash */
    }
  }
}

export async function initPrefs() {
  if (typeof window !== 'undefined' && window.matchMedia) {
    media = window.matchMedia('(prefers-color-scheme: dark)');
    media.addEventListener?.('change', () => apply(get(prefs)));
  }
  try {
    const saved = await invoke<Partial<Prefs>>('get_prefs');
    prefs.set({ ...DEFAULTS, ...sanitize(saved) });
  } catch {
    prefs.set({ ...DEFAULTS });
  }
  loaded = true;
  apply(get(prefs));
}

function sanitize(p: Partial<Prefs> | null | undefined): Partial<Prefs> {
  const out: Partial<Prefs> = {};
  if (!p || typeof p !== 'object') return out;
  if (p.lang && ['auto', 'it', 'en', 'ja'].includes(p.lang)) out.lang = p.lang;
  if (p.theme && ['system', 'light', 'dark'].includes(p.theme)) out.theme = p.theme;
  if (p.accent && (ACCENTS as string[]).includes(p.accent)) out.accent = p.accent;
  if (typeof p.keepAwake === 'boolean') out.keepAwake = p.keepAwake;
  return out;
}

export async function updatePrefs(patch: Partial<Prefs>) {
  const next = { ...get(prefs), ...patch };
  prefs.set(next);
  apply(next);
  if (!loaded) return;
  try {
    await invoke('set_prefs', { prefs: next });
  } catch {
    /* not fatal: the choice still applies until the app is closed */
  }
}
