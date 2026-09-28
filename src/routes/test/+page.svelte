<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy } from 'svelte';

  interface LiveStatus {
    count: number;
    state: 'warming' | 'locked' | 'idle';
    period_ms: number | null;
    sample_rate: number;
    profile_used: boolean;
  }

  let status: LiveStatus | null = null;
  let isRecording = false;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let errorText = '';

  const STATE_LABEL: Record<string, string> = {
    idle: 'In attesa',
    warming: 'In ascolto…',
    locked: 'In conteggio',
  };

  async function start() {
    errorText = '';
    try {
      // sample_rate is decided by the input device once recording
      // starts; reset once we know it, right after start_recording.
      await invoke('start_recording');
      isRecording = true;
      pollTimer = setInterval(poll, 200);
    } catch (e) {
      errorText = `Impossibile avviare: ${e}`;
    }
  }

  async function stop() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    isRecording = false;
    try {
      // expected count is irrelevant here (0): this recording is not
      // being saved as a training take, just counted live.
      await invoke('stop_recording', { expectedDaimokuCount: 0 });
      await poll();
    } catch (e) {
      errorText = `Errore durante l'arresto: ${e}`;
    }
  }

  async function poll() {
    try {
      status = await invoke<LiveStatus>('live_status');
    } catch (e) {
      errorText = `Errore di lettura: ${e}`;
    }
  }

  async function resetCounter() {
    if (!status) return;
    await invoke('reset_live_counter', { sampleRate: status.sample_rate || 48000 });
    await poll();
  }

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
  });
</script>

<div class="page">
  <header>
    <h1>Conta Daimoku</h1>
    <p class="subtitle">Recita liberamente: il conteggio si aggiorna in tempo reale.</p>
  </header>

  <div class="counter-card">
    <div class="count">{status?.count ?? 0}</div>
    <div class="state" class:locked={status?.state === 'locked'}>
      {STATE_LABEL[status?.state ?? 'idle']}
    </div>
    {#if status?.period_ms}
      <div class="period">~{(status.period_ms / 1000).toFixed(2)} s / daimoku</div>
    {/if}
  </div>

  <div class="controls">
    {#if !isRecording}
      <button class="primary" on:click={start}>Avvia</button>
    {:else}
      <button class="danger" on:click={stop}>Ferma</button>
    {/if}
    <button class="ghost" on:click={resetCounter} disabled={isRecording}>Azzera</button>
  </div>

  {#if errorText}
    <p class="error">{errorText}</p>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2rem;
    flex: 1;
    text-align: center;
  }

  header h1 {
    margin: 0 0 0.3rem;
    font-size: 1.8rem;
  }
  .subtitle {
    margin: 0;
    opacity: 0.65;
  }

  .counter-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }
  .count {
    font-size: 7rem;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .state {
    font-size: 1rem;
    opacity: 0.6;
    padding: 0.25rem 0.9rem;
    border-radius: 999px;
    background: #262626;
  }
  .state.locked {
    background: #1c3d27;
    color: #7fd99a;
    opacity: 1;
  }
  .period {
    font-size: 0.9rem;
    opacity: 0.55;
  }

  .controls {
    display: flex;
    gap: 0.75rem;
  }
  button.primary {
    background: #3b6ea5;
    color: #fff;
    border: none;
    padding: 0.8rem 2.2rem;
    border-radius: 10px;
    font-size: 1.05rem;
    cursor: pointer;
  }
  button.danger {
    background: #a53b3b;
    color: #fff;
    border: none;
    padding: 0.8rem 2.2rem;
    border-radius: 10px;
    font-size: 1.05rem;
    cursor: pointer;
  }
  button.ghost {
    background: transparent;
    border: 1px solid #444;
    color: #ccc;
    padding: 0.8rem 1.4rem;
    border-radius: 10px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .error {
    color: #e58b8b;
    font-size: 0.85rem;
  }
</style>
