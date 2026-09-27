<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface TrainingTakeMeta {
    id: number;
    expected_daimoku_count: number;
    duration_ms: number;
    sample_rate: number;
    created_at: string;
  }

  let takes = $state<TrainingTakeMeta[]>([]);
  let expectedCount = $state(10);
  let recording = $state(false);
  let error = $state<string | null>(null);
  let exportedPath = $state<string | null>(null);

  let hasTake = $derived(takes.length > 0);
  let lastTake = $derived(takes.length > 0 ? takes[takes.length - 1] : null);

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
    try {
      await invoke('start_recording');
      recording = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function stop() {
    error = null;
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

  refresh();
</script>

<div class="page">
  <h1>Teach the Daimoku</h1>

  <p class="instructions">
    Recite <strong>{expectedCount}</strong> Daimoku in one continuous take — no need to
    stop between them. Press <em>Start</em>, recite, then press <em>Stop</em>.
    The system will learn the rhythm and the boundaries between recitations.
  </p>

  <label class="count-input">
    Number of Daimoku to recite:
    <input
      type="number"
      min="3"
      max="30"
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

  {#if recording}
    <div class="status recording">● Recording… recite {expectedCount} Daimoku then press Stop</div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if exportedPath}
    <div class="info">
      Exported to <code>{exportedPath}</code>
    </div>
  {/if}

  {#if lastTake}
    <section class="list">
      <h2>Recordings</h2>
      <ul>
        {#each takes as t (t.id)}
          <li>
            <span class="count">{t.expected_daimoku_count} Daimoku</span>
            <span class="dur">{(t.duration_ms / 1000).toFixed(1)} s</span>
            <button class="export" on:click={() => exportWav(t.id)}>Export WAV</button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if hasTake}
    <div class="ready">
      ✓ Training data ready. Learning will be implemented in Fase 2.3.
    </div>
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

  .recording { color: #ff6b6b; }

  .error {
    color: #ff9b9b;
    background-color: #2a1515;
    padding: 0.75rem 1rem;
    border-radius: 8px;
    max-width: 32rem;
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

  .ready {
    color: #8fe38f;
    background-color: #152a15;
    padding: 0.75rem 1rem;
    border-radius: 8px;
  }

  .list {
    width: 100%;
    max-width: 24rem;
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
    gap: 0.75rem;
    padding: 0.4rem 0.75rem;
    background-color: #242424;
    border-radius: 6px;
    margin-bottom: 0.3rem;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
  }
  .count { flex: 1; color: #cfcfcf; }
  .dur { flex: 1; color: #888; text-align: right; }
  .export {
    padding: 0.25rem 0.6rem;
    font-size: 0.75rem;
    background-color: #4a4a4a;
  }
</style>