// Badges (milestones of all the Daimoku chanted, and of consecutive days)
// and the forgiven day: once a week a day without Daimoku can be forgiven,
// so that it does not break the run of consecutive days.
//
// Badges are worked out from the sessions every time (a run of days earned
// once stays earned, even if the run later breaks); the preferences only
// keep the forgiven days and which badges have been announced.
import { get, writable } from 'svelte/store';
import type { SessionRecord } from './api';
import type { Key } from './i18n';
import { prefs, prefsLoaded, updatePrefs } from './prefs';
import { addDays, chantedDays, dayKey, startOfDay, startOfWeek } from './stats';

/** All the Daimoku ever chanted. */
export const TOTAL_BADGES = [100, 1_000, 10_000, 1_000_000, 5_000_000, 10_000_000];
/** Consecutive days: 1 week, 1 month, 6 months, 1, 2, 5, 10, 15, 20 years. */
export const STREAK_BADGES = [7, 30, 180, 365, 730, 1825, 3650, 5475, 7300];
/** A forgiven day must be at most this many days ago. */
const GRACE_DAYS = 7;

export interface Badge {
  id: string;
  kind: 'total' | 'streak';
  /** Daimoku or days needed. */
  n: number;
  earned: boolean;
  /** What there is so far (all the Daimoku, or the longest run of days). */
  have: number;
}

function total(sessions: SessionRecord[]): number {
  return sessions.reduce((a, s) => a + s.count, 0);
}

/** Days chanted or forgiven, oldest first. */
function presentDays(sessions: SessionRecord[], forgiven: string[]): Date[] {
  const keys = chantedDays(sessions);
  for (const k of forgiven) keys.add(k);
  return [...keys]
    .map((k) => {
      const [y, m, d] = k.split('-').map(Number);
      return new Date(y, m - 1, d);
    })
    .sort((a, b) => a.getTime() - b.getTime());
}

/** The longest run of consecutive days, forgiven ones included. */
export function longestStreak(sessions: SessionRecord[], forgiven: string[] = []): number {
  let best = 0;
  let run = 0;
  let prev: Date | null = null;
  for (const d of presentDays(sessions, forgiven)) {
    run = prev && dayKey(addDays(prev, 1)) === dayKey(d) ? run + 1 : 1;
    best = Math.max(best, run);
    prev = d;
  }
  return best;
}

/** Every badge, earned or not. `live`: Daimoku of a session still running. */
export function badges(sessions: SessionRecord[], forgiven: string[] = [], live = 0): Badge[] {
  const all = total(sessions) + live;
  const days = longestStreak(sessions, forgiven);
  return [
    ...TOTAL_BADGES.map((n) => ({ id: `total-${n}`, kind: 'total' as const, n, earned: all >= n, have: all })),
    ...STREAK_BADGES.map((n) => ({ id: `streak-${n}`, kind: 'streak' as const, n, earned: days >= n, have: days })),
  ];
}

/**
 * The day that can be forgiven now, if any: the most recent day without
 * Daimoku in the last week, right after a day with some (so that
 * forgiving it keeps a run going), in a week where no day has been
 * forgiven yet.
 */
export function graceOffer(sessions: SessionRecord[], forgiven: string[], now = new Date()): { day: Date; key: string } | null {
  const present = chantedDays(sessions);
  for (const k of forgiven) present.add(k);
  const today = startOfDay(now);
  for (let back = 1; back <= GRACE_DAYS; back++) {
    const d = addDays(today, -back);
    if (present.has(dayKey(d))) continue;
    // the first day missing going back from yesterday
    if (!present.has(dayKey(addDays(d, -1)))) return null;
    const week = dayKey(startOfWeek(d));
    const used = forgiven.some((k) => {
      const [y, m, dd] = k.split('-').map(Number);
      return dayKey(startOfWeek(new Date(y, m - 1, dd))) === week;
    });
    return used ? null : { day: d, key: dayKey(d) };
  }
  return null;
}

export function forgive(key: string) {
  const p = get(prefs);
  if (!p.forgiven.includes(key)) updatePrefs({ forgiven: [...p.forgiven, key] });
}

/** Badges earned and not announced yet (shown by the layout, then cleared). */
export const newBadges = writable<Badge[]>([]);

/** Announces the badges earned since last time. */
export function noteBadges(sessions: SessionRecord[], live = 0) {
  if (!get(prefsLoaded)) return; // the announced ones are not known yet
  const p = get(prefs);
  const earned = badges(sessions, p.forgiven, live).filter((b) => b.earned);
  const fresh = earned.filter((b) => !p.badgesSeen.includes(b.id));
  if (fresh.length === 0) return;
  updatePrefs({ badgesSeen: [...p.badgesSeen, ...fresh.map((b) => b.id)] });
  newBadges.update((q) => [...q, ...fresh]);
}

/** Short text inside a badge: 100, 1K, 10K, 1M... (Japanese, Chinese, Korean: 1万, 100万...). */
export function badgeShort(b: Badge, loc: string): string {
  if (b.kind === 'streak') return '';
  if (/^(ja|zh|ko)/.test(loc)) return new Intl.NumberFormat(loc, { notation: 'compact' }).format(b.n);
  if (b.n >= 1_000_000) return `${b.n / 1_000_000}M`;
  if (b.n >= 1_000) return `${b.n / 1_000}K`;
  return String(b.n);
}

/** A run of days as weeks, months or years: [unit, how many]. */
export function streakSpan(days: number): ['week' | 'month' | 'year', number] {
  if (days < 30) return ['week', Math.round(days / 7)];
  if (days < 365) return ['month', Math.round(days / 30)];
  return ['year', Math.round(days / 365)];
}

/** Full name of a badge ("1.000 daimoku", "1 month in a row"). */
export function badgeName(b: Badge, tr: (k: Key, v?: Record<string, string | number>) => string, loc: string): string {
  if (b.kind === 'total') return tr('badge.daimoku', { n: b.n.toLocaleString(loc) });
  const [unit, n] = streakSpan(b.n);
  const key = (n === 1 ? `badge.${unit}` : `badge.${unit}s`) as Key;
  return tr('badge.inARow', { span: tr(key, { n }) });
}
