// User preferences (language, theme, colour), saved by the Rust side in
// prefs.json next to the history, and applied to the document.
import { invoke } from '@tauri-apps/api/core';
import { get, writable } from 'svelte/store';
import { detectLang, lang, type Lang } from './i18n';
import { BACKGROUNDS, loadCustomBackground, type Background } from './background';
import { BELLS, type BellKind } from './bell';
import { PACE } from './gongyo';

export type ThemePref = 'system' | 'light' | 'dark';
export type Accent = 'lotus' | 'gold' | 'sakura' | 'indigo';
export const ACCENTS: Accent[] = ['lotus', 'gold', 'sakura', 'indigo'];

export interface Prefs {
  lang: 'auto' | Lang;
  theme: ThemePref;
  accent: Accent;
  /** Keep the screen on while counting (counting also works with it off). */
  keepAwake: boolean;
  /** Goals beyond the daily one (kept with the history); 0 = none. */
  goals: { week: number; month: number; year: number };
  /** Daily reminders of the two sessions (Android), time "HH:MM". */
  reminders: Record<'morning' | 'evening', { on: boolean; time: string }>;
  /** Days forgiven so as not to break the run of days (`dayKey`, see badges.ts). */
  forgiven: string[];
  /** Badges already announced. */
  badgesSeen: string[];
  background: Background;
  /** The bell button on the Count screen. */
  bell: { on: boolean; kind: BellKind; volume: number };
  /** The period the Stats page opens on. */
  statsPeriod: StatsPeriod;
  /** Pace of the Gongyo page, in beats (kanji) a second. */
  gongyoPace: number;
}

export type StatsPeriod = 'day' | 'week' | 'month' | 'year' | 'all';
export const STATS_PERIODS: StatsPeriod[] = ['day', 'week', 'month', 'year', 'all'];

const DEFAULTS: Prefs = {
  lang: 'auto',
  theme: 'system',
  accent: 'lotus',
  keepAwake: false,
  goals: { week: 0, month: 0, year: 0 },
  reminders: { morning: { on: false, time: '07:00' }, evening: { on: false, time: '19:00' } },
  forgiven: [],
  badgesSeen: [],
  background: 'lotus',
  bell: { on: false, kind: 'high', volume: 0.8 },
  statsPeriod: 'week',
  gongyoPace: PACE.medium,
};

/** Strings matching `re`, at most `max` of them. */
function strings(v: unknown, re: RegExp, max: number): string[] | undefined {
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string' && re.test(x)).slice(-max) : undefined;
}

function reminder(v: unknown, fallback: { on: boolean; time: string }) {
  const r = (v && typeof v === 'object' ? v : {}) as { on?: unknown; time?: unknown };
  return {
    on: typeof r.on === 'boolean' ? r.on : fallback.on,
    time: typeof r.time === 'string' && /^([01]\d|2[0-3]):[0-5]\d$/.test(r.time) ? r.time : fallback.time,
  };
}

/** A whole number in 0..max, or `fallback`. */
function count(v: unknown, max: number, fallback: number): number {
  return typeof v === 'number' && Number.isFinite(v) ? Math.min(max, Math.max(0, Math.round(v))) : fallback;
}

export const prefs = writable<Prefs>({ ...DEFAULTS });
/** True once the saved preferences have been read. */
export const prefsLoaded = writable(false);
/** The theme actually shown (system preference resolved). */
export const resolvedTheme = writable<'light' | 'dark'>('dark');

let loaded = false;
let customLoaded = false;
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
    root.dataset.bg = p.background;
    if (p.background === 'custom' && !customLoaded) {
      customLoaded = true;
      loadCustomBackground();
    }
    root.lang = get(lang);
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute('content', theme === 'dark' ? '#0c0d0f' : '#eef2ea');
    // cache for app.html, so the next start paints the right theme immediately
    try {
      localStorage.setItem('daimoku-theme', JSON.stringify({ theme: p.theme, accent: p.accent, bg: p.background }));
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
  prefsLoaded.set(true);
  apply(get(prefs));
}

function sanitize(p: Partial<Prefs> | null | undefined): Partial<Prefs> {
  const out: Partial<Prefs> = {};
  if (!p || typeof p !== 'object') return out;
  if (p.lang && ['auto', 'it', 'en', 'ja'].includes(p.lang)) out.lang = p.lang;
  if (p.theme && ['system', 'light', 'dark'].includes(p.theme)) out.theme = p.theme;
  if (p.accent && (ACCENTS as string[]).includes(p.accent)) out.accent = p.accent;
  if (typeof p.keepAwake === 'boolean') out.keepAwake = p.keepAwake;
  if (p.goals && typeof p.goals === 'object') {
    const g = p.goals as Partial<Prefs['goals']>;
    out.goals = { week: count(g.week, 1e7, 0), month: count(g.month, 1e8, 0), year: count(g.year, 1e9, 0) };
  }
  if (p.background && (BACKGROUNDS as readonly string[]).includes(p.background)) out.background = p.background;
  if (p.bell && typeof p.bell === 'object') {
    const b = p.bell as Partial<Prefs['bell']>;
    out.bell = {
      on: typeof b.on === 'boolean' ? b.on : DEFAULTS.bell.on,
      kind: b.kind && (BELLS as readonly string[]).includes(b.kind) ? b.kind : DEFAULTS.bell.kind,
      volume: typeof b.volume === 'number' && Number.isFinite(b.volume) ? Math.min(1, Math.max(0, b.volume)) : DEFAULTS.bell.volume,
    };
  }
  if (p.statsPeriod && (STATS_PERIODS as string[]).includes(p.statsPeriod)) out.statsPeriod = p.statsPeriod;
  if (typeof p.gongyoPace === 'number' && Number.isFinite(p.gongyoPace))
    out.gongyoPace = Math.min(PACE.max, Math.max(PACE.min, p.gongyoPace));
  const forgiven = strings(p.forgiven, /^\d{4}-\d{1,2}-\d{1,2}$/, 2000);
  if (forgiven) out.forgiven = forgiven;
  const seen = strings(p.badgesSeen, /^(total|streak)-\d+$/, 100);
  if (seen) out.badgesSeen = seen;
  if (p.reminders && typeof p.reminders === 'object') {
    out.reminders = {
      morning: reminder(p.reminders.morning, DEFAULTS.reminders.morning),
      evening: reminder(p.reminders.evening, DEFAULTS.reminders.evening),
    };
  }
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
