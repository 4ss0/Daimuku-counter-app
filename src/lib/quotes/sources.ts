// Where each quote of the day comes from. A quote names its source with a
// key (quotes/pools.ts); the names, in each language, are in
// src/lib/locales/<code>/sources.ts.
//   'g:<writing>'  a writing of Nichiren Daishonin (GOSHO_IDS)
//   'nd'           Nichiren Daishonin, writing not given
//   'ls:<n>'       the Lotus Sutra, chapter n
//   'ott'          the Record of the Orally Transmitted Teachings
//   'a:<author>'   another author (AUTHOR_IDS)
//   's:<sutra>'    another sutra (SUTRA_IDS)
//   ''             no source: the quote is shown without one
import type { SourceTexts } from '../locales/types';

export const GOSHO_IDS = [
  'akimoto', 'attaining', 'believer', 'brothers', 'buddhaland', 'daimoku',
  'debts', 'dragon', 'drum', 'evil', 'eyes', 'faith',
  'flowering', 'food', 'fourteen', 'gohonzon', 'happiness', 'heritage',
  'hero', 'itai', 'kalpa', 'karma', 'kyoo', 'lifespan',
  'newyear', 'nichigon', 'nichiro', 'niike', 'object', 'obstacles',
  'persecutions', 'personlaw', 'pondered', 'practicing', 'prayer', 'prophecy',
  'qa', 'rain', 'realaspect', 'rice', 'rissho', 'sado',
  'ship', 'stonetiger', 'strategy', 'supremacy', 'sustain', 'time',
  'tower', 'tranquil', 'treasures', 'twofaith', 'unseen', 'votary',
  'winds', 'winter',
] as const;
export const AUTHOR_IDS = ['ikeda', 'toda', 'makiguchi', 'tiantai', 'miaolo', 'dengyo', 'nikko'] as const;
export const SUTRA_IDS = ['nirvana', 'meanings', 'worthy', 'mindground'] as const;

export type GoshoId = (typeof GOSHO_IDS)[number];
export type AuthorId = (typeof AUTHOR_IDS)[number];
export type SutraId = (typeof SUTRA_IDS)[number];

const has = <T extends string>(list: readonly T[], x: string | undefined): x is T => list.includes(x as T);

/** {name} in `template` replaced by `vars[name]`. */
function fill(template: string, vars: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (m, k: string) => (k in vars ? String(vars[k]) : m));
}

/** The source of a quote, written out with the names `s` ('' when it has none). */
export function sourceLabel(key: string, s: SourceTexts): string {
  if (!key) return '';
  const [kind, name] = key.split(':');
  if (kind === 'nd') return s.nichiren;
  if (kind === 'ott') return s.ott;
  if (kind === 'g' && has(GOSHO_IDS, name)) return fill(s.writing, { author: s.nichiren, title: s.gosho[name] });
  if (kind === 'ls') {
    const n = Number(name);
    return fill(s.lotus, { n, chapter: s.lotusChapters?.[n - 1] ?? String(n) }).trim();
  }
  if (kind === 'a' && has(AUTHOR_IDS, name)) return s.authors[name];
  if (kind === 's' && has(SUTRA_IDS, name)) return s.sutras[name];
  return '';
}

/** Every key used by the quotes must be known. */
export function knownSource(key: string): boolean {
  if (!key) return true;
  const [kind, name] = key.split(':');
  if (kind === 'nd' || kind === 'ott') return name === undefined;
  if (kind === 'g') return has(GOSHO_IDS, name);
  if (kind === 'ls') return Number(name) >= 1 && Number(name) <= 28;
  if (kind === 'a') return has(AUTHOR_IDS, name);
  if (kind === 's') return has(SUTRA_IDS, name);
  return false;
}
