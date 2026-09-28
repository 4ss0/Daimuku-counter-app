<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onDestroy, onMount } from 'svelte';

  interface LiveView {
    count: number;
    state: 'idle' | 'warming' | 'locked';
    speaking: boolean;
    elapsed_secs: number;
    period_ms: number | null;
    recent_events_secs: number[];
    finished: boolean;
  }

  interface LiveSessionSummary {
    count: number;
    duration_secs: number;
    period_ms: number | null;
    saved_audio_secs: number;
  }

  const POLL_MS = 150;

  let view: LiveView | null = null;
  let running = false;
  let busy = false;
  let summary: LiveSessionSummary | null = null;
  let errorText = '';
  let savedPath = '';
  let pulse = false;
  let lastCount = 0;
  let timer: ReturnType<typeof setInterval> | null = null;

  $: count = view?.count ?? 0;
  $: elapsed = view?.elapsed_secs ?? 0;
  $: perMinute = count > 0 && elapsed > 5 ? (count / elapsed) * 60 : null;
  $: recent = (view?.recent_events_secs ?? []).slice(-12).reverse();
  $: status = statusText(view, running);

  function statusText(v: LiveView | null, isRunning: boolean): string {
    if (!isRunning) return v?.finished ? 'Sessione terminata' : 'Pronto';
    if (!v || v.state === 'idle') return v?.speaking ? 'In ascolto…' : 'In attesa della voce…';
    if (v.state === 'locked') return 'Sto contando';
    return 'In ascolto…';
  }

  function fmtTime(secs: number): string {
    const s = Math.max(0, Math.floor(secs));
    const m = Math.floor(s / 60);
    const h = Math.floor(m / 60);
    const mm = String(m % 60).padStart(2, '0');
    const ss = String(s % 60).padStart(2, '0');
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }

  async function poll() {
    try {
      const v = await invoke<LiveView>('live_view');
      if (v.count > lastCount) {
        pulse = false;
        // restart the CSS animation on every new Daimoku
        requestAnimationFrame(() => (pulse = true));
      }
      lastCount = v.count;
      view = v;
    } catch (e) {
      errorText = `Errore di lettura: ${e}`;
    }
  }

  async function start() {
    if (busy || running) return;
    busy = true;
    errorText = '';
    savedPath = '';
    summary = null;
    lastCount = 0;
    view = null;
    try {
      await invoke('start_live_session');
      running = true;
      timer = setInterval(poll, POLL_MS);
    } catch (e) {
      errorText = `Impossibile avviare il microfono: ${e}`;
    } finally {
      busy = false;
    }
  }

  async function stop() {
    if (busy || !running) return;
    busy = true;
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    try {
      summary = await invoke<LiveSessionSummary>('stop_live_session');
      await poll();
    } catch (e) {
      errorText = `Errore durante l'arresto: ${e}`;
    } finally {
      running = false;
      busy = false;
    }
  }

  async function saveWav() {
    try {
      savedPath = await invoke<string>('export_live_session_wav');
    } catch (e) {
      errorText = `Impossibile salvare il WAV: ${e}`;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.code === 'Space' && !(e.target instanceof HTMLInputElement)) {
      e.preventDefault();
      running ? stop() : start();
    }
  }

  onMount(() => window.addEventListener('keydown', onKey));
  onDestroy(() => {
    window.removeEventListener('keydown', onKey);
    if (timer) clearInterval(timer);
    // leaving the page must not leave the microphone on
    if (running) invoke('stop_live_session').catch(() => {});
  });
</script>

<div class="page">
  <div class="status-row">
    <span class="dot" class:on={running && view?.speaking} class:running></span>
    <span class="status" class:locked={running && view?.state === 'locked'}>{status}</span>
    {#if running || summary}
      <span class="time">{fmtTime(elapsed)}</span>
    {/if}
  </div>

  <div class="counter" class:pulse on:animationend={() => (pulse = false)}>
    {count}
  </div>
  <div class="label">daimoku</div>

  <div class="meta">
    {#if perMinute !== null}
      <span>{perMinute.toFixed(0)} / min</span>
    {/if}
    {#if view?.period_ms}
      <span>~{(view.period_ms / 1000).toFixed(1)} s ciascuno</span>
    {/if}
  </div>

  <button
    class="main"
    class:stop={running}
    on:click={running ? stop : start}
    disabled={busy}
  >
    {running ? 'Ferma' : summary ? 'Nuova sessione' : 'Inizia'}
  </button>
  <div class="hint">Barra spaziatrice per iniziare / fermare</div>

  {#if recent.length > 0}
    <div class="events">
      <div class="events-title">Ultimi conteggi</div>
      <div class="chips">
        {#each recent as t, i}
          <span class="chip">#{count - i} · {fmtTime(t)}</span>
        {/each}
      </div>
    </div>
  {/if}

  {#if summary && !running}
    <div class="summary">
      <div>
        <strong>{summary.count}</strong> daimoku in {fmtTime(summary.duration_secs)}
      </div>
      <p class="small">
        Se il conteggio non corrisponde, salva l'audio e mandamelo: servono proprio le
        registrazioni dove sbaglia.
        {#if summary.saved_audio_secs > 0 && summary.saved_audio_secs + 1 < summary.duration_secs}
          (Vengono salvati i primi {Math.round(summary.saved_audio_secs / 60)} minuti.)
        {/if}
      </p>
      <button class="ghost" on:click={saveWav}>Salva WAV della sessione</button>
      {#if savedPath}
        <p class="small ok">Salvato in {savedPath}</p>
      {/if}
    </div>
  {/if}

  {#if errorText}
    <p class="error">{errorText}</p>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.9rem;
    text-align: center;
    user-select: none;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.95rem;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #444;
    transition: background 0.15s;
  }
  .dot.running {
    background: #6b6b6b;
  }
  .dot.on {
    background: #7fd99a;
    box-shadow: 0 0 8px #7fd99a;
  }
  .status {
    padding: 0.25rem 0.8rem;
    border-radius: 999px;
    background: #262626;
    color: #bbb;
  }
  .status.locked {
    background: #1c3d27;
    color: #7fd99a;
  }
  .time {
    font-variant-numeric: tabular-nums;
    color: #999;
  }

  .counter {
    font-size: clamp(6rem, 22vw, 11rem);
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    margin-top: 0.5rem;
  }
  .counter.pulse {
    animation: pulse 0.45s ease-out;
  }
  @keyframes pulse {
    0% {
      transform: scale(1.12);
      color: #9cc4ee;
    }
    100% {
      transform: scale(1);
      color: inherit;
    }
  }
  .label {
    margin-top: -0.6rem;
    color: #888;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-size: 0.85rem;
  }

  .meta {
    display: flex;
    gap: 1.2rem;
    min-height: 1.3rem;
    color: #999;
    font-size: 0.9rem;
  }

  button.main {
    margin-top: 0.8rem;
    min-width: 12rem;
    padding: 1rem 2.5rem;
    font-size: 1.2rem;
    border: none;
    border-radius: 999px;
    background: #3b6ea5;
    color: #fff;
    cursor: pointer;
  }
  button.main.stop {
    background: #a53b3b;
  }
  button.main:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .hint {
    font-size: 0.75rem;
    color: #666;
  }

  .events {
    max-width: 34rem;
  }
  .events-title {
    font-size: 0.8rem;
    color: #777;
    margin-bottom: 0.4rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    justify-content: center;
  }
  .chip {
    font-size: 0.78rem;
    padding: 0.2rem 0.55rem;
    border-radius: 6px;
    background: #232323;
    color: #aaa;
    font-variant-numeric: tabular-nums;
  }

  .summary {
    margin-top: 0.5rem;
    padding: 1rem 1.4rem;
    border-radius: 12px;
    background: #232323;
    max-width: 30rem;
  }
  .small {
    font-size: 0.82rem;
    color: #999;
    margin: 0.5rem 0;
  }
  .small.ok {
    color: #7fd99a;
    word-break: break-all;
  }
  button.ghost {
    background: transparent;
    border: 1px solid #444;
    color: #ccc;
    padding: 0.55rem 1.1rem;
    border-radius: 8px;
    cursor: pointer;
  }
  .error {
    color: #e58b8b;
    font-size: 0.85rem;
  }
</style>
