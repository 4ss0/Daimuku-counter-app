<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface DeviceInfo {
    name: string;
    is_default: boolean;
  }

  interface RecordingSummary {
    samples_count: number;
    duration_ms: number;
    sample_rate: number;
  }

  let devices: DeviceInfo[] = [];
  let summary: RecordingSummary | null = null;
  let recording = false;
  let error: string | null = null;

  async function loadDevices() {
    error = null;
    try {
      devices = await invoke<DeviceInfo[]>('list_input_devices');
    } catch (e) {
      error = String(e);
    }
  }

  async function start() {
    error = null;
    summary = null;
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
      summary = await invoke<RecordingSummary>('stop_recording');
      recording = false;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="page">
  <h1>Samples</h1>
  <p>Audio pipeline test. Real sample recording arrives in Fase 2.2.</p>

  <div class="actions">
    <button on:click={loadDevices}>List input devices</button>
    <button on:click={start} disabled={recording}>Start</button>
    <button on:click={stop} disabled={!recording}>Stop</button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if devices.length > 0}
    <section>
      <h2>Devices</h2>
      <ul>
        {#each devices as d}
          <li>{d.name}{d.is_default ? ' (default)' : ''}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if summary}
    <section>
      <h2>Last recording</h2>
      <ul>
        <li>Samples: {summary.samples_count}</li>
        <li>Duration: {summary.duration_ms} ms</li>
        <li>Sample rate: {summary.sample_rate} Hz</li>
      </ul>
    </section>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 1rem;
  }

  h1 {
    margin: 0;
  }

  p {
    opacity: 0.75;
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  button {
    padding: 0.6rem 1.2rem;
    border: none;
    border-radius: 8px;
    background-color: #3b6ea5;
    color: #fff;
    cursor: pointer;
    font-size: 1rem;
  }

  button:disabled {
    background-color: #444;
    cursor: not-allowed;
  }

  button:hover:not(:disabled) {
    background-color: #4a7fb8;
  }

  section {
    margin-top: 1rem;
    padding: 1rem;
    background-color: #242424;
    border-radius: 8px;
    text-align: left;
    min-width: 20rem;
  }

  section h2 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    opacity: 0.75;
  }

  section ul {
    margin: 0;
    padding-left: 1.2rem;
  }

  .error {
    color: #ff6b6b;
    background-color: #2a1515;
    padding: 0.75rem 1rem;
    border-radius: 8px;
  }
</style>