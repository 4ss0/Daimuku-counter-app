// The quote of the day shown when the app opens: one for every day of the
// year, 29 February included, the same each year, in the app's language.
//
// A few quotes are tied to a day (New Year, the anniversaries of Nichiren
// Daishonin and of the Soka Gakkai); the others fill the remaining days
// taking turns between the sources, so that the same kind does not come
// many days in a row.
import type { Lang } from '../i18n';
import { GOSHO } from './gosho';
import { LOTUS } from './lotus';
import { OTHER } from './other';
import { SOKA } from './soka';
import { sourceLabel, type Q } from './sources';

export const DAYS = 366;

/** Day of the year as in a leap year: 1 January = 0, 29 February = 59, 31 December = 365. */
export function dayIndex(d: Date): number {
  return Math.round((Date.UTC(2024, d.getMonth(), d.getDate()) - Date.UTC(2024, 0, 1)) / 86_400_000);
}

/** The day of a year (leap) for a day index: for showing "23 October". */
export function dayDate(day: number): Date {
  return new Date(2024, 0, 1 + day);
}

function build(): Q[] {
  const pools = [GOSHO, LOTUS, SOKA, OTHER];
  const days: (Q | undefined)[] = new Array(DAYS);
  for (const q of pools.flat()) {
    if (!q[4]) continue;
    const [m, d] = q[4].split('-').map(Number);
    days[dayIndex(new Date(2024, m - 1, d))] = q;
  }
  const rest = pools.map((p) => p.filter((q) => !q[4]));
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
  return days.filter((q): q is Q => !!q);
}

const ALL = build();

export interface DailyQuote {
  text: string;
  /** Where it comes from ('' when not given). */
  source: string;
}

/** The quote for a day of the year (any number: it wraps around). */
export function quoteFor(day: number, lang: Lang): DailyQuote {
  const q = ALL[((day % ALL.length) + ALL.length) % ALL.length];
  const i = lang === 'it' ? 0 : lang === 'en' ? 1 : 2;
  return { text: q[i], source: sourceLabel(q[3], lang) };
}

/** For the quote check: every quote in the order of the days. */
export function allQuotes(): readonly Q[] {
  return ALL;
}
