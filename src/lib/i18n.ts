// Languages and their texts. Each language is a folder of src/lib/locales
// (see README.md there): adding a folder adds the language. English is
// built in and stands in for anything missing; the other languages are
// loaded when they are chosen.
import { derived, writable } from 'svelte/store';
import en from './locales/en';
import type { Key, Locale, LocaleMeta } from './locales/types';

export type { Key, Locale };

/** A language: the name of its folder in src/lib/locales ("it", "pt"...). */
export type Lang = string;

export interface LangInfo extends LocaleMeta {
  id: Lang;
}

const metas = import.meta.glob<LocaleMeta>('./locales/*/meta.ts', { eager: true, import: 'default' });
const loaders = import.meta.glob<Locale>(['./locales/*/index.ts', '!./locales/en/index.ts'], { import: 'default' });

/** Every language, in the order of their own names. */
export const LANGS: LangInfo[] = Object.entries(metas)
  .map(([path, m]) => ({ id: path.split('/')[2], ...m }))
  .sort((a, b) => a.name.localeCompare(b.name, 'en'));

const BY_ID = new Map(LANGS.map((l) => [l.id, l]));

export function isLang(x: unknown): x is Lang {
  return typeof x === 'string' && BY_ID.has(x);
}

/** Other codes devices use for some languages. */
const ALIASES: Record<string, Lang> = { nb: 'no', nn: 'no', tl: 'fil', in: 'id' };

/** The device's language if the app has it, otherwise English. */
export function detectLang(): Lang {
  const tags = typeof navigator !== 'undefined' ? (navigator.languages?.length ? navigator.languages : [navigator.language]) : [];
  for (const tag of tags) {
    const base = (tag || '').toLowerCase().split(/[-_]/)[0];
    const id = ALIASES[base] ?? base;
    if (BY_ID.has(id)) return id;
  }
  return 'en';
}

const cache = new Map<Lang, Promise<Locale>>([['en', Promise.resolve(en)]]);

/** The texts of a language, loaded once (English if they cannot be). */
export function loadLocale(l: Lang): Promise<Locale> {
  let p = cache.get(l);
  if (!p) {
    const load = loaders[`./locales/${l}/index.ts`];
    p = load ? load().catch(() => en) : Promise.resolve(en);
    cache.set(l, p);
  }
  return p;
}

const current = writable<{ id: Lang; texts: Locale }>({ id: 'en', texts: en });
let wanted: Lang = 'en';

/** Switches the app to `l` as soon as its texts are loaded. */
export async function setLang(l: Lang): Promise<void> {
  wanted = l;
  const texts = await loadLocale(l);
  if (wanted !== l) return; // another language was chosen meanwhile
  current.set({ id: l, texts });
  if (typeof document !== 'undefined') document.documentElement.lang = l;
}

/** The language in use (already resolved from "auto"). */
export const lang = derived(current, (c) => c.id);

/** All the texts of the language in use: screens, guide, dictionary, quotes, Gongyo. */
export const texts = derived(current, (c) => c.texts);

/** The English texts, always at hand. */
export const english: Locale = en;

/** The tag for dates and numbers. */
export const locale = derived(current, (c) => BY_ID.get(c.id)?.intl ?? 'en-GB');

/** The language's own name. */
export function langName(l: Lang): string {
  return BY_ID.get(l)?.name ?? l;
}

/** Options of the language `l` (e.g. how its dictionary is sorted). */
export function langInfo(l: Lang): LangInfo | undefined {
  return BY_ID.get(l);
}

export type TFn = (key: Key, vars?: Record<string, string | number>) => string;

export const t = derived(current, ({ texts: d }): TFn => {
  return (key, vars) => {
    let s: string = d.ui[key] ?? en.ui[key] ?? key;
    if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
    return s;
  };
});
