// The quote of the day shown when the app opens: one for every day of the
// year, 29 February included, the same each year, in the app's language.
//
// A few quotes are tied to a day (New Year, the anniversaries of Nichiren
// Daishonin and of the Soka Gakkai); the others fill the remaining days
// taking turns between the sources, so that the same kind does not come
// many days in a row.
import type { Locale } from '../locales/types';
import { POOLS, type PoolId, type QuoteMeta } from './pools';
import { sourceLabel } from './sources';

export const DAYS = 366;

/** A quote: the pool it is in and its place there (as in every language). */
export interface QuoteRef {
  pool: PoolId;
  i: number;
  meta: QuoteMeta;
}

/** Day of the year as in a leap year: 1 January = 0, 29 February = 59, 31 December = 365. */
export function dayIndex(d: Date): number {
  return Math.round((Date.UTC(2024, d.getMonth(), d.getDate()) - Date.UTC(2024, 0, 1)) / 86_400_000);
}

/** The day of a year (leap) for a day index: for showing "23 October". */
export function dayDate(day: number): Date {
  return new Date(2024, 0, 1 + day);
}

function build(): QuoteRef[] {
  const ids = Object.keys(POOLS) as PoolId[];
  const pools: QuoteRef[][] = ids.map((pool) => POOLS[pool].map((meta, i) => ({ pool, i, meta })));
  const days: (QuoteRef | undefined)[] = new Array(DAYS);
  for (const q of pools.flat()) {
    if (!q.meta[1]) continue;
    const [m, d] = q.meta[1].split('-').map(Number);
    days[dayIndex(new Date(2024, m - 1, d))] = q;
  }
  const rest = pools.map((p) => p.filter((q) => !q.meta[1]));
  const total = rest.reduce((a, p) => a + p.length, 0);
  const taken = rest.map(() => 0);
  let k = 0;
  for (let i = 0; i < DAYS; i++) {
    if (days[i]) continue;
    // the source furthest behind its share of the days filled so far
    let best = -1;
    let gap = -Infinity;
    rest.forEach((p, j) => {
      if (taken[j] >= p.length) return;
      const g = ((k + 1) * p.length) / total - taken[j];
      if (g > gap) {
        gap = g;
        best = j;
      }
    });
    if (best < 0) break;
    days[i] = rest[best][taken[best]++];
    k++;
  }
  return days.filter((q): q is QuoteRef => !!q);
}

const ALL = build();

export interface DailyQuote {
  text: string;
  /** Where it comes from ('' when not given). */
  source: string;
}

/** The quote for a day of the year (any number: it wraps around), in the language `texts`. */
export function quoteFor(day: number, texts: Locale): DailyQuote {
  const q = ALL[((day % ALL.length) + ALL.length) % ALL.length];
  return { text: texts.quotes[q.pool][q.i], source: sourceLabel(q.meta[0], texts.sources) };
}

/** For checks: every quote in the order of the days. */
export function allQuotes(): readonly QuoteRef[] {
  return ALL;
}
