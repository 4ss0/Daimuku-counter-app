// Typed wrappers around the Rust commands (src-tauri/src/commands.rs).
import { invoke } from '@tauri-apps/api/core';

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
  exportLiveWav: () => invoke<string>('export_live_session_wav'),

  listSessions: () => invoke<SessionRecord[]>('list_sessions'),
  updateSessionCount: (id: number, count: number) =>
    invoke<SessionRecord>('update_session_count', { id, count }),
  deleteSession: (id: number) => invoke<void>('delete_session', { id }),
  addManual: (count: number) => invoke<SessionRecord>('add_manual_session', { count }),
  getGoal: () => invoke<number>('get_daily_goal'),
  setGoal: (goal: number) => invoke<number>('set_daily_goal', { goal }),

  profile: () => invoke<PersonalProfile>('get_personal_profile'),
  clearProfile: () => invoke<void>('clear_personal_profile'),
  startRecording: () => invoke<void>('start_recording'),
  stopRecording: (expected: number) =>
    invoke<TrainingTakeMeta>('stop_recording', { expectedDaimokuCount: expected }),
  validateTake: (index: number) => invoke<ValidationResult>('validate_training_take', { index }),
  addTakeToProfile: (index: number) => invoke<PersonalProfile>('add_take_to_profile', { index }),
  clearTrainingTakes: () => invoke<number>('clear_training_takes'),
};

export function errText(e: unknown): string {
  const s = String(e);
  if (/permission|denied|autorizz|build input stream|start stream|input config/i.test(s)) {
    return "L'app non può usare il microfono: consenti l'accesso nelle impostazioni del telefono.";
  }
  if (/no default input device/i.test(s)) {
    return 'Nessun microfono trovato.';
  }
  return s;
}

export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${String(m).padStart(2, '0')}:${ss}`;
}

export function fmtDuration(secs: number): string {
  const m = Math.round(secs / 60);
  if (m < 1) return secs > 0 ? '<1 min' : '—';
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${String(m % 60).padStart(2, '0')}`;
}

export function fmtNum(n: number): string {
  return n.toLocaleString('it-IT');
}
