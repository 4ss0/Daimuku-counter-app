// Typed wrappers around the Rust commands (src-tauri/src/commands.rs).
import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import { locale, t, type TFn } from './i18n';
import type { ExportedFile } from './native';

export interface RestoreSummary {
  sessions: number;
  takes: number;
  takes_skipped: number;
}

export interface LiveView {
  count: number;
  state: 'idle' | 'warming' | 'locked';
  speaking: boolean;
  elapsed_secs: number;
  period_ms: number | null;
  recent_events_secs: number[];
  finished: boolean;
}

export interface SessionRecord {
  id: number;
  started_at: string; // ISO 8601, UTC
  duration_secs: number;
  count: number;
  detected: number;
  manual: boolean;
}

export interface LiveSessionSummary {
  count: number;
  duration_secs: number;
  period_ms: number | null;
  saved_audio_secs: number;
  session: SessionRecord | null;
}

export interface TakeRecord {
  id: number;
  n_daimoku: number;
  duration_secs: number;
  period_ms: number | null;
  created_at: string;
}

export interface PersonalProfile {
  takes: TakeRecord[];
  base_clip_count: number;
  ref_llr: number;
  min_cycle_ms: number;
  max_cycle_ms: number;
}

export interface TrainingTakeMeta {
  id: number;
  expected_daimoku_count: number;
  duration_ms: number;
  sample_rate: number;
  created_at: string;
}

export interface ValidationResult {
  expected: number;
  detected: number;
  ok: boolean;
}

export const api = {
  startLive: () => invoke<void>('start_live_session'),
  stopLive: () => invoke<LiveSessionSummary>('stop_live_session'),
  liveView: () => invoke<LiveView>('live_view'),
  exportLiveWav: () => invoke<ExportedFile>('export_live_session_wav'),

  listSessions: () => invoke<SessionRecord[]>('list_sessions'),
  updateSessionCount: (id: number, count: number) =>
    invoke<SessionRecord>('update_session_count', { id, count }),
  deleteSession: (id: number) => invoke<void>('delete_session', { id }),
  addManual: (count: number) => invoke<SessionRecord>('add_manual_session', { count }),
  getGoal: () => invoke<number>('get_daily_goal'),
  setGoal: (goal: number) => invoke<number>('set_daily_goal', { goal }),

  profile: () => invoke<PersonalProfile>('get_personal_profile'),
  clearProfile: () => invoke<void>('clear_personal_profile'),
  deleteTake: (id: number) => invoke<PersonalProfile>('delete_profile_take', { id }),
  startRecording: () => invoke<void>('start_recording'),
  stopRecording: (expected: number) =>
    invoke<TrainingTakeMeta>('stop_recording', { expectedDaimokuCount: expected }),
  validateTake: (index: number) => invoke<ValidationResult>('validate_training_take', { index }),
  addTakeToProfile: (index: number) => invoke<PersonalProfile>('add_take_to_profile', { index }),
  clearTrainingTakes: () => invoke<number>('clear_training_takes'),

  createBackup: () => invoke<ExportedFile>('create_backup'),
  restoreBackup: (content: string) => invoke<RestoreSummary>('restore_backup', { content }),
  writeTextExport: (name: string, content: string) => invoke<ExportedFile>('write_text_export', { name, content }),
};

export function errText(e: unknown): string {
  const s = String(e);
  const tr = get(t);
  if (/permission|denied|build input stream|start stream|input config|audio backend/i.test(s)) {
    return `${tr('err.mic')} (${s})`;
  }
  if (/no default input device/i.test(s)) {
    return tr('err.noMic');
  }
  if (s.includes('backup-newer')) return tr('err.backupNewer');
  if (s.includes('backup-invalid')) return tr('err.backupInvalid');
  return s;
}

export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${String(m).padStart(2, '0')}:${ss}`;
}

export function fmtDuration(secs: number, tr: TFn = get(t)): string {
  const m = Math.round(secs / 60);
  if (m < 1) return secs > 0 ? tr('dur.lessMin') : '—';
  if (m < 60) return tr('dur.min', { m });
  return tr('dur.hm', { h: Math.floor(m / 60), m: String(m % 60).padStart(2, '0') });
}

export function fmtNum(n: number, loc = get(locale)): string {
  return n.toLocaleString(loc);
}
