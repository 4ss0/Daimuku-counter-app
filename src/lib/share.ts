// Short texts shared from the app (a session, a period of statistics).
import { get } from 'svelte/store';
import { fmtDuration, fmtNum, type SessionRecord } from './api';
import { locale, t } from './i18n';
import { shareText } from './native';

/** Date and time of a session, in the user's language. */
function when(iso: string): string {
  return new Date(iso).toLocaleString(get(locale), { dateStyle: 'medium', timeStyle: 'short' });
}

export function sessionText(s: Pick<SessionRecord, 'count' | 'duration_secs' | 'started_at' | 'manual'>): string {
  const tr = get(t);
  const n = fmtNum(s.count);
  const w = when(s.started_at);
  const line = s.manual || !(s.duration_secs > 0)
    ? tr('share.sessionManual', { n, w })
    : tr('share.session', { n, d: fmtDuration(s.duration_secs, tr), w });
  return `${line}\n${tr('share.footer')}`;
}

export interface StatsShare {
  title: string;
  total: number;
  sessions: number;
  seconds: number;
  streak: number;
}

export function statsText(s: StatsShare): string {
  const tr = get(t);
  const lines = [tr('share.stats', { title: s.title, n: fmtNum(s.total) })];
  if (s.sessions > 0) lines.push(tr('share.statsSessions', { n: fmtNum(s.sessions), d: fmtDuration(s.seconds, tr) }));
  if (s.streak > 1) lines.push(tr('share.statsStreak', { n: s.streak }));
  lines.push(tr('share.footer'));
  return lines.join('\n');
}

/** Shares a text; returns a message to show (empty if nothing to say). */
export async function share(text: string): Promise<string> {
  const tr = get(t);
  const r = await shareText(text, tr('share.button'));
  return r === 'copied' ? tr('share.copied') : '';
}
