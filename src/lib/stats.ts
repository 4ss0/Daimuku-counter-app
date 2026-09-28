// Statistics computed from the saved sessions, in the phone's local time.
import type { SessionRecord } from './api';

export type Period = 'day' | 'week' | 'month' | 'year' | 'all';

export interface Bucket {
  label: string; // short axis label
  title: string; // full label for the tooltip
  value: number;
  current: boolean; // the bucket containing "now"
}

export interface PeriodStats {
  total: number;
  sessions: number;
  seconds: number;
  prevTotal: number | null; // same-length previous period (null for "all")
  perDay: number | null; // average per day elapsed in the period
  buckets: Bucket[];
  title: string;
}

const DAY_MS = 86_400_000;

// Month / weekday names in the user's language.
function fmt(loc: string, o: Intl.DateTimeFormatOptions, d: Date): string {
  return new Intl.DateTimeFormat(loc, o).format(d);
}
const monthNarrow = (loc: string, m: number) => fmt(loc, { month: 'narrow' }, new Date(2021, m, 1));
const monthLong = (loc: string, y: number, m: number) => fmt(loc, { month: 'long', year: 'numeric' }, new Date(y, m, 1));
const monthOnly = (loc: string, m: number) => {
  const s = fmt(loc, { month: 'long' }, new Date(2021, m, 1));
  return s.charAt(0).toUpperCase() + s.slice(1);
};
// 1 Jan 2024 was a Monday
const weekdayShort = (loc: string, i: number) => fmt(loc, { weekday: 'short' }, new Date(2024, 0, 1 + i));
const dayTitle = (loc: string, d: Date) => fmt(loc, { weekday: 'short', day: 'numeric', month: 'short' }, d);

export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}
function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
}
export function startOfWeek(d: Date): Date {
  const s = startOfDay(d);
  const dow = (s.getDay() + 6) % 7; // Monday = 0
  return addDays(s, -dow);
}
function daysInMonth(y: number, m: number): number {
  return new Date(y, m + 1, 0).getDate();
}
export function dayKey(d: Date): string {
  return `${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`;
}

interface Parsed {
  t: Date;
  count: number;
  seconds: number;
}

function parse(sessions: SessionRecord[]): Parsed[] {
  return sessions
    .map((s) => ({ t: new Date(s.started_at), count: s.count, seconds: s.duration_secs }))
    .filter((p) => !isNaN(p.t.getTime()));
}

function sumIn(ps: Parsed[], from: Date, to: Date) {
  let total = 0;
  let sessions = 0;
  let seconds = 0;
  for (const p of ps) {
    if (p.t >= from && p.t < to) {
      total += p.count;
      sessions += 1;
      seconds += p.seconds;
    }
  }
  return { total, sessions, seconds };
}

export function todayTotal(sessions: SessionRecord[], now = new Date()): number {
  const from = startOfDay(now);
  return sumIn(parse(sessions), from, addDays(from, 1)).total;
}

/** Consecutive days with at least one Daimoku, ending today (or
 *  yesterday, if nothing has been chanted yet today). */
export function streak(sessions: SessionRecord[], now = new Date()): number {
  const days = new Set(parse(sessions).filter((p) => p.count > 0).map((p) => dayKey(p.t)));
  let d = startOfDay(now);
  if (!days.has(dayKey(d))) d = addDays(d, -1);
  let n = 0;
  while (days.has(dayKey(d))) {
    n += 1;
    d = addDays(d, -1);
  }
  return n;
}

export function bestDay(sessions: SessionRecord[]): { date: Date; total: number } | null {
  const m = new Map<string, { date: Date; total: number }>();
  for (const p of parse(sessions)) {
    const k = dayKey(p.t);
    const e = m.get(k) ?? { date: startOfDay(p.t), total: 0 };
    e.total += p.count;
    m.set(k, e);
  }
  let best: { date: Date; total: number } | null = null;
  for (const e of m.values()) if (!best || e.total > best.total) best = e;
  return best && best.total > 0 ? best : null;
}

/** Days (from the start of the month of `now`) on which the goal was met. */
export function goalDaysThisMonth(sessions: SessionRecord[], goal: number, now = new Date()) {
  const ps = parse(sessions);
  const y = now.getFullYear();
  const m = now.getMonth();
  let met = 0;
  for (let d = 1; d <= now.getDate(); d++) {
    const from = new Date(y, m, d);
    if (sumIn(ps, from, addDays(from, 1)).total >= goal) met += 1;
  }
  return { met, elapsed: now.getDate() };
}

export function periodStats(
  sessions: SessionRecord[],
  period: Period,
  now = new Date(),
  loc = 'it-IT',
): PeriodStats {
  const ps = parse(sessions);
  const buckets: Bucket[] = [];
  let from: Date;
  let to: Date;
  let prevFrom: Date | null = null;
  let title = '';

  if (period === 'day') {
    from = startOfDay(now);
    to = addDays(from, 1);
    prevFrom = addDays(from, -1);
    title = '';
    for (let h = 0; h < 24; h++) {
      const a = new Date(from.getFullYear(), from.getMonth(), from.getDate(), h);
      const b = new Date(from.getFullYear(), from.getMonth(), from.getDate(), h + 1);
      buckets.push({
        label: h % 6 === 0 ? String(h) : '',
        title: `${h}:00–${h + 1}:00`,
        value: sumIn(ps, a, b).total,
        current: now >= a && now < b,
      });
    }
  } else if (period === 'week') {
    from = startOfWeek(now);
    to = addDays(from, 7);
    prevFrom = addDays(from, -7);
    title = '';
    for (let i = 0; i < 7; i++) {
      const a = addDays(from, i);
      buckets.push({
        label: weekdayShort(loc, i),
        title: dayTitle(loc, a),
        value: sumIn(ps, a, addDays(a, 1)).total,
        current: dayKey(a) === dayKey(now),
      });
    }
  } else if (period === 'month') {
    from = new Date(now.getFullYear(), now.getMonth(), 1);
    to = new Date(now.getFullYear(), now.getMonth() + 1, 1);
    prevFrom = new Date(now.getFullYear(), now.getMonth() - 1, 1);
    title = monthOnly(loc, now.getMonth());
    const n = daysInMonth(now.getFullYear(), now.getMonth());
    for (let d = 1; d <= n; d++) {
      const a = new Date(now.getFullYear(), now.getMonth(), d);
      buckets.push({
        label: d === 1 || d % 5 === 0 ? String(d) : '',
        title: dayTitle(loc, a),
        value: sumIn(ps, a, addDays(a, 1)).total,
        current: d === now.getDate(),
      });
    }
  } else if (period === 'year') {
    from = new Date(now.getFullYear(), 0, 1);
    to = new Date(now.getFullYear() + 1, 0, 1);
    prevFrom = new Date(now.getFullYear() - 1, 0, 1);
    title = String(now.getFullYear());
    for (let m = 0; m < 12; m++) {
      const a = new Date(now.getFullYear(), m, 1);
      const b = new Date(now.getFullYear(), m + 1, 1);
      buckets.push({
        label: monthNarrow(loc, m),
        title: monthLong(loc, now.getFullYear(), m),
        value: sumIn(ps, a, b).total,
        current: m === now.getMonth(),
      });
    }
  } else {
    title = '';
    const first = ps.length ? ps.reduce((a, p) => (p.t < a ? p.t : a), ps[0].t) : now;
    from = new Date(0);
    to = new Date(8.64e15);
    const firstYear = first.getFullYear();
    if (now.getFullYear() - firstYear >= 2) {
      for (let y = firstYear; y <= now.getFullYear(); y++) {
        buckets.push({
          label: String(y).slice(2),
          title: String(y),
          value: sumIn(ps, new Date(y, 0, 1), new Date(y + 1, 0, 1)).total,
          current: y === now.getFullYear(),
        });
      }
    } else {
      // up to the last 24 months, starting at the first month with data
      let m0 = new Date(first.getFullYear(), first.getMonth(), 1);
      const last = new Date(now.getFullYear(), now.getMonth(), 1);
      const minStart = new Date(now.getFullYear(), now.getMonth() - 23, 1);
      if (m0 < minStart) m0 = minStart;
      const lastYear = new Date(now.getFullYear(), now.getMonth() - 5, 1);
      if (m0 > lastYear) m0 = lastYear; // always show at least 6 months
      for (let d = m0; d <= last; d = new Date(d.getFullYear(), d.getMonth() + 1, 1)) {
        const b = new Date(d.getFullYear(), d.getMonth() + 1, 1);
        buckets.push({
          label: monthNarrow(loc, d.getMonth()),
          title: monthLong(loc, d.getFullYear(), d.getMonth()),
          value: sumIn(ps, d, b).total,
          current: d.getTime() === last.getTime(),
        });
      }
    }
  }

  const cur = sumIn(ps, from, to);
  let prevTotal: number | null = null;
  if (prevFrom) {
    // compare with the same elapsed part of the previous period
    const elapsed = Math.min(now.getTime(), to.getTime()) - from.getTime();
    prevTotal = sumIn(ps, prevFrom, new Date(Math.min(from.getTime(), prevFrom.getTime() + elapsed))).total;
  }
  let perDay: number | null = null;
  if (period === 'week' || period === 'month' || period === 'year') {
    const days = Math.max(1, Math.floor((startOfDay(now).getTime() - from.getTime()) / DAY_MS) + 1);
    perDay = cur.total / days;
  } else if (period === 'all' && ps.length) {
    const first = ps.reduce((a, p) => (p.t < a ? p.t : a), ps[0].t);
    const days = Math.max(1, Math.floor((startOfDay(now).getTime() - startOfDay(first).getTime()) / DAY_MS) + 1);
    perDay = cur.total / days;
  }

  return { total: cur.total, sessions: cur.sessions, seconds: cur.seconds, prevTotal, perDay, buckets, title };
}

export function sessionsIn(sessions: SessionRecord[], period: Period, now = new Date()): SessionRecord[] {
  let from = new Date(0);
  if (period === 'day') from = startOfDay(now);
  else if (period === 'week') from = startOfWeek(now);
  else if (period === 'month') from = new Date(now.getFullYear(), now.getMonth(), 1);
  else if (period === 'year') from = new Date(now.getFullYear(), 0, 1);
  return sessions
    .filter((s) => new Date(s.started_at) >= from)
    .sort((a, b) => b.started_at.localeCompare(a.started_at));
}

export function fmtWhen(
  iso: string,
  now = new Date(),
  loc = 'it-IT',
  words = { today: 'Oggi', yesterday: 'Ieri' },
): string {
  const d = new Date(iso);
  const hm = fmt(loc, { hour: 'numeric', minute: '2-digit' }, d);
  const diff = Math.round((startOfDay(now).getTime() - startOfDay(d).getTime()) / DAY_MS);
  if (diff === 0) return `${words.today}, ${hm}`;
  if (diff === 1) return `${words.yesterday}, ${hm}`;
  const same = d.getFullYear() === now.getFullYear();
  const day = fmt(loc, same ? { day: 'numeric', month: 'short' } : { day: 'numeric', month: 'short', year: 'numeric' }, d);
  return `${day}, ${hm}`;
}
