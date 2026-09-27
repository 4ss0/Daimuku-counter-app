<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy } from 'svelte';

  interface TrainingTakeMeta {
    id: number;
    expected_daimoku_count: number;
    duration_ms: number;
    sample_rate: number;
    created_at: string;
  }

  interface DaimokuCountResult {
    count: number;
    period_ms: number;
    confidence: number;
    duration_secs: number;
    segment_count: number;
    active_duration_secs: number;
    // new fields (kept optional for safety)
    mean_period_ms?: number;
    phrase_count?: number;
    method?: string;
  }

  interface ValidationResult {
    expected: number;
    detected: number;
    ok: boolean;
    analysis: DaimokuCountResult;
  }

  type StreamingState = 'warming' | 'locked' | 'idle';

  interface LiveStatus {
    count: number;
    state: StreamingState;
    period_ms: number | null;
    sample_rate: number;
  }

  let takes = $state<TrainingTakeMeta[]>([]);
  let expectedCount = $state(10);
  let recording = $state(false);
  let error = $state<string | null>(null);
  let exportedPath = $state<string | null>(null);
  let validation = $state<{ index: number; result: ValidationResult } | null>(null);

  // ---------------------------------------------------------------------------
  // Live counter
  // ---------------------------------------------------------------------------
  let live = $state<LiveStatus>({
    count: 0,
    state: 'idle',
    period_ms: null,
    sample_rate: 0,
  });
  let pollHandle: number | null = null;

  let hasTake = $derived(takes.length > 0);
  let targetReached = $derived(recording && live.count >= expectedCount);

  let stateLabel = $derived(
    live.state === 'warming'
      ? 'Listening…'
      : live.state === 'locked'
        ? 'Counting'
        : recording
          ? 'Waiting for voice'
          : 'Idle'
  );

  let stateClass = $derived(
    live.state === 'locked'
      ? 'badge locked'
      : live.state === 'warming'
        ? 'badge warming'
        : 'badge idle'
  );

  let periodLabel = $derived(
    live.period_ms && live.period_ms > 0
      ? `${(live.period_ms / 1000).toFixed(2)} s / Daimoku`
      : '—'
  );

  let progressPct = $derived(
    Math.min(100, (live.count / Math.max(1, expectedCount)) * 100)
  );

  async function refresh() {
    try {
      takes = await invoke<TrainingTakeMeta[]>('list_training_takes');
    } catch (e) {
      error = String(e);
    }
  }

  async function start() {
    error = null;
    exportedPath = null;
    validation = null;
    try {
      await invoke('start_recording');
      recording = true;
      startPolling();
    } catch (e) {
      error = String(e);
    }
  }

  async function stop() {
    error = null;
    stopPolling();
    try {
      await invoke<TrainingTakeMeta>('stop_recording', {
        expectedDaimokuCount: expectedCount,
      });
      recording = false;
      await refresh();
    } catch (e) {
      error = String(e);
      recording = false;
    }
  }

  async function clearAll() {
    if (!confirm('Delete all training recordings?')) return;
    error = null;
    exportedPath = null;
    validation = null;
    try {
      await invoke<number>('clear_training_takes');
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function exportWav(index: number) {
    error = null;
    exportedPath = null;
    try {
      exportedPath = await invoke<string>('export_training_wav', { index });
    } catch (e) {
      error = String(e);
    }
  }

  async function validate(index: number) {
    error = null;
    validation = null;
    try {
      const result = await invoke<ValidationResult>('validate_training_take', { index });
      validation = { index, result };
    } catch (e) {
      error = String(e);
    }
  }

  // ---------------------------------------------------------------------------
  // Live counter polling
  // ---------------------------------------------------------------------------
  function startPolling() {
    stopPolling();
    void tick();
    pollHandle = window.setInterval(tick, 150);
  }

  function stopPolling() {
    if (pollHandle !== null) {
      clearInterval(pollHandle);
      pollHandle = null;
    }
    // Reset displayed live values when we stop.
    live = { count: 0, state: 'idle', period_ms: null, sample_rate: 0 };
  }

  async function tick() {
    try {
      live = await invoke<LiveStatus>('live_status');
    } catch {
      /* keep last value */
    }
  }

  onDestroy(stopPolling);

  refresh();
</script>

<div class="page">
  <h1>Teach the Daimoku</h1>

  <p class="instructions">
    Recite <strong>{expectedCount}</strong> Daimoku in one continuous take — no need to
    stop between them. Press <em>Start</em>, recite, then press <em>Stop</em>.
    The system will learn the rhythm from your recording.
  </p>

  <label class="count-input">
    Number of Daimoku to recite:
    <input
      type="number"
      min="3"
      max="60"
      bind:value={expectedCount}
      disabled={recording}
    />
  </label>

  <div class="actions">
    <button on:click={start} disabled={recording}>Start</button>
    <button on:click={stop} disabled={!recording}>Stop</button>
    <button class="secondary" on:click={clearAll} disabled={recording || !hasTake}>
      Clear all
    </button>
  </div>

  <!-- ---------------------------------------------------------------------- -->
  <!-- Live counter (visible only while recording)                             -->
  <!-- ---------------------------------------------------------------------- -->
  {#if recording}
    <section class="live">
      <div class="live-count">
        <span class="live-number">{live.count}</span>
        <span class="live-target">/ {expectedCount}</span>
      </div>

      <div class="live-meta">
        <span class={stateClass}>{stateLabel}</span>
        <span class="live-period">{periodLabel}</span>
      </div>

      <div class="progress">
        <div class="progress-fill" style="width: {progressPct}%"></div>
      </div>

      <div class="recording">
        {#if targetReached}
          ● Target reached — press Stop to save
        {:else}
          ● Recording… recite {expectedCount - live.count} more
        {/if}
      </div>
    </section>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if exportedPath}
    <div class="info">
      Exported to <code>{exportedPath}</code>
    </div>
  {/if}

  {#if hasTake}
    <section class="list">
      <h2>Recordings</h2>
      <ul>
        {#each takes as t (t.id)}
          <li>
            <span class="count">{t.expected_daimoku_count} Daimoku</span>
            <span class="dur">{(t.duration_ms / 1000).toFixed(1)} s</span>
            <button class="btn-mini primary" on:click={() => validate(t.id)}>Validate</button>
            <button class="btn-mini" on:click={() => exportWav(t.id)}>WAV</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if validation}
    <section class="validation" class:ok={validation.result.ok} class:fail={!validation.result.ok}>
      <h2>
        {validation.result.ok ? '✓ Training validated' : '✗ Validation failed'}
        — take #{validation.index}
      </h2>
      <p>
        Recited: <strong>{validation.result.expected}</strong>
        · Detected: <strong>{validation.result.detected}</strong>
      </p>
      <p class="details">
        Period: {validation.result.analysis.period_ms.toFixed(0)} ms
        · Confidence: {(validation.result.analysis.confidence * 100).toFixed(0)}%
        · Segments: {validation.result.analysis.segment_count}
        · Active: {validation.result.analysis.active_duration_secs.toFixed(1)} s
      </p>
      {#if !validation.result.ok}
        <p class="hint">
          Recite again with clearer syllable separation and a steadier rhythm.
        </p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    text-align: center;
    gap: 1.25rem;
    padding-top: 1rem;
    max-width: 42rem;
    margin: 0 auto;
    width: 100%;
  }

  h1 { margin: 0; }

  .instructions {
    margin: 0;
    opacity: 0.8;
    line-height: 1.6;
    max-width: 34rem;
  }

  .count-input {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.95rem;
  }

  .count-input input {
    width: 5rem;
    padding: 0.4rem 0.6rem;
    border: 1px solid #444;
    border-radius: 6px;
    background-color: #1e1e1e;
    color: inherit;
    font-size: 1rem;
    text-align: center;
  }

  .actions { display: flex; gap: 1rem; }

  button {
    padding: 0.6rem 1.4rem;
    border: none;
    border-radius: 8px;
    background-color: #3b6ea5;
    color: #fff;
    cursor: pointer;
    font-size: 1rem;
  }
  button.secondary { background-color: #4a4a4a; }
  button:disabled { background-color: #2f2f2f; color: #666; cursor: not-allowed; }
  button:hover:not(:disabled) { filter: brightness(1.15); }

  .btn-mini {
    padding: 0.25rem 0.6rem;
    font-size: 0.75rem;
    background-color: #4a4a4a;
  }
  .btn-mini.primary { background-color: #3b6ea5; }

  .recording { color: #ff6b6b; }

  /* ---- Live counter --------------------------------------------------- */
  .live {
    width: 100%;
    max-width: 34rem;
    padding: 1.25rem 1.5rem 1rem;
    background-color: #1a1d21;
    border: 1px solid #23272c;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
  }

  .live-count {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    font-variant-numeric: tabular-nums;
  }
  .live-number {
    font-size: 3.5rem;
    font-weight: 700;
    color: #7dd3fc;
    line-height: 1;
  }
  .live-target {
    font-size: 1.4rem;
    color: #8a8a8a;
  }

  .live-meta {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.85rem;
    color: #b8b8b8;
  }

  .badge {
    padding: 0.15rem 0.6rem;
    border-radius: 999px;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .badge.warming { background: #3a2f10; color: #f0c674; }
  .badge.locked  { background: #133a1c; color: #6ee7a8; }
  .badge.idle    { background: #2a2a2a; color: #b8b8b8; }

  .live-period {
    font-variant-numeric: tabular-nums;
  }

  .progress {
    width: 100%;
    height: 6px;
    background: #1f2226;
    border-radius: 999px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #3b6ea5, #7dd3fc);
    transition: width 0.15s linear;
  }

  /* ---- Messages ------------------------------------------------------- */
  .error {
    color: #ff9b9b;
    background-color: #2a1515;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 34rem;
  }

  .info {
    color: #9bd4ff;
    background-color: #152433;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 32rem;
    word-break: break-all;
  }
  .info code {
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
  }

  /* ---- Recordings list ------------------------------------------------ */
  .list {
    width: 100%;
    max-width: 30rem;
    text-align: left;
  }
  .list h2 {
    font-size: 0.95rem;
    opacity: 0.7;
    margin: 0 0 0.5rem;
    font-weight: normal;
  }
  .list ul { list-style: none; padding: 0; margin: 0; }
  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    background-color: #242424;
    border-radius: 6px;
    margin-bottom: 0.3rem;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
  }
  .count { flex: 1; color: #cfcfcf; }
  .dur { flex: 1; color: #888; text-align: right; }

  /* ---- Validation ----------------------------------------------------- */
  .validation {
    width: 100%;
    max-width: 34rem;
    text-align: left;
    padding: 0.75rem 1rem;
    border-radius: 8px;
  }
  .validation.ok {
    background-color: #152a15;
    border: 1px solid #2f4a2f;
    color: #8fe38f;
  }
  .validation.fail {
    background-color: #2a1515;
    border: 1px solid #4a2f2f;
    color: #ff9b9b;
  }
  .validation h2 {
    font-size: 1rem;
    margin: 0 0 0.5rem;
    font-weight: normal;
  }
  .validation p { margin: 0.25rem 0; font-size: 0.9rem; }
  .validation .details,
  .validation .hint {
    font-size: 0.85rem;
    opacity: 0.8;
  }
</style>