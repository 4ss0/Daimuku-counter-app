<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, errText, fmtClock, fmtNum, type LiveSessionSummary, type LiveView, type SessionRecord } from '$lib/api';
  import { todayTotal } from '$lib/stats';

  const POLL_MS = 150;
  const R = 118;
  const CIRC = 2 * Math.PI * R;

  let view: LiveView | null = null;
  let running = false;
  let busy = false;
  let summary: LiveSessionSummary | null = null;
  let corrected = 0;
  let errorText = '';
  let savedPath = '';
  let pulse = false;
  let lastCount = 0;
  let timer: ReturnType<typeof setInterval> | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  let sessions: SessionRecord[] = [];
  let goal = 100;
  let userTakes = -1;

  $: liveCount = running ? view?.count ?? 0 : summary ? corrected : 0;
  $: baseToday = todayTotal(sessions);
  // while counting, the session is not saved yet: add it on top
  $: today = running ? baseToday + liveCount : baseToday;
  $: progress = Math.min(1, today / Math.max(1, goal));
  $: goalReached = today >= goal;
  $: elapsed = view?.elapsed_secs ?? 0;
  $: statusLabel = !running
    ? summary
      ? 'Sessione salvata'
      : 'Pronto'
    : view?.state === 'locked'
      ? 'Sto contando'
      : view?.speaking
        ? 'Ti ascolto…'
        : 'In attesa della voce…';
  $: dateLabel = new Date().toLocaleDateString('it-IT', { weekday: 'long', day: 'numeric', month: 'long' });

  async function refresh() {
    try {
      [sessions, goal] = await Promise.all([api.listSessions(), api.getGoal()]);
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function poll() {
    try {
      const v = await api.liveView();
      if (v.count > lastCount) {
        pulse = false;
        requestAnimationFrame(() => (pulse = true));
        if (navigator.vibrate) navigator.vibrate(12);
      }
      lastCount = v.count;
      view = v;
    } catch (e) {
      errorText = errText(e);
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
      await api.startLive();
      running = true;
      timer = setInterval(poll, POLL_MS);
    } catch (e) {
      errorText = errText(e);
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
      summary = await api.stopLive();
      corrected = summary.count;
      await poll();
      await refresh();
    } catch (e) {
      errorText = errText(e);
    } finally {
      running = false;
      busy = false;
    }
  }

  function adjust(delta: number) {
    if (!summary?.session) return;
    corrected = Math.max(0, corrected + delta);
    if (saveTimer) clearTimeout(saveTimer);
    const id = summary.session.id;
    const value = corrected;
    saveTimer = setTimeout(async () => {
      try {
        await api.updateSessionCount(id, value);
        await refresh();
      } catch (e) {
        errorText = errText(e);
      }
    }, 500);
  }

  async function discard() {
    if (!summary?.session) return;
    try {
      await api.deleteSession(summary.session.id);
      summary = null;
      await refresh();
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function saveWav() {
    try {
      savedPath = await api.exportLiveWav();
    } catch (e) {
      errorText = errText(e);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.code === 'Space' && !(e.target instanceof HTMLInputElement)) {
      e.preventDefault();
      running ? stop() : start();
    }
  }

  onMount(async () => {
    window.addEventListener('keydown', onKey);
    await refresh();
    try {
      userTakes = (await api.profile()).takes.length;
    } catch {
      userTakes = -1;
    }
  });

  onDestroy(() => {
    window.removeEventListener('keydown', onKey);
    if (timer) clearInterval(timer);
    if (running) api.stopLive().catch(() => {});
  });
</script>

<div class="page">
  <header>
    <div class="date">{dateLabel}</div>
    <div class="today">
      Oggi <strong>{fmtNum(today)}</strong> <span class="of">/ {fmtNum(goal)}</span>
      {#if goalReached}<span class="done" aria-label="obiettivo raggiunto">✓</span>{/if}
    </div>
  </header>

  <div class="ring-wrap" class:speaking={running && view?.speaking}>
    <svg class="ring" viewBox="0 0 280 280" aria-hidden="true">
      <circle class="track" cx="140" cy="140" r={R} />
      <circle
        class="bar"
        class:full={goalReached}
        cx="140"
        cy="140"
        r={R}
        stroke-dasharray={CIRC}
        stroke-dashoffset={CIRC * (1 - progress)}
      />
    </svg>
    <div class="center">
      <div class="count" class:pulse on:animationend={() => (pulse = false)} aria-live="polite">
        {liveCount}
      </div>
      <div class="unit">daimoku</div>
    </div>
  </div>

  <div class="status">
    <span class="dot" class:on={running && view?.speaking} class:live={running}></span>
    <span>{statusLabel}</span>
    {#if running || summary}
      <span class="clock">{fmtClock(running ? elapsed : summary?.duration_secs ?? 0)}</span>
    {/if}
  </div>

  <button
    class="go"
    class:stop={running}
    on:click={running ? stop : start}
    disabled={busy}
    aria-label={running ? 'Ferma' : 'Inizia'}
  >
    {#if running}
      <svg viewBox="0 0 24 24"><rect x="6.5" y="6.5" width="11" height="11" rx="2.5" /></svg>
    {:else}
      <svg viewBox="0 0 24 24"><path d="M8.5 5.8v12.4a1 1 0 0 0 1.5.86l10-6.2a1 1 0 0 0 0-1.72l-10-6.2a1 1 0 0 0-1.5.86z" /></svg>
    {/if}
  </button>
  <div class="go-label">{running ? 'Ferma' : 'Inizia'}</div>

  {#if summary && !running}
    <section class="sheet">
      {#if summary.session}
        <div class="sheet-row">
          <div>
            <div class="sheet-title">Sessione salvata</div>
            <div class="sheet-sub">
              {fmtClock(summary.duration_secs)}
              {#if corrected !== summary.count}· rilevati {summary.count}{/if}
            </div>
          </div>
          <div class="stepper" role="group" aria-label="Correggi il numero">
            <button on:click={() => adjust(-1)} aria-label="Uno in meno">−</button>
            <span>{corrected}</span>
            <button on:click={() => adjust(1)} aria-label="Uno in più">+</button>
          </div>
        </div>
        <p class="hint">Se ne ha persi o aggiunti, correggi il numero: le statistiche usano questo valore.</p>
        <div class="sheet-actions">
          <button class="link" on:click={saveWav}>Salva l'audio per segnalare un errore</button>
          <button class="link danger" on:click={discard}>Elimina sessione</button>
        </div>
        {#if savedPath}<p class="saved">Salvato in {savedPath}</p>{/if}
      {:else}
        <div class="sheet-title">Nessun daimoku rilevato</div>
        <p class="hint">
          Avvicina il telefono e recita a voce chiara. Puoi aggiungere daimoku a mano dalle
          <a href="/stats">statistiche</a>.
        </p>
      {/if}
    </section>
  {:else if !running && userTakes === 0}
    <a class="tip" href="/voce">
      <strong>Insegna all'app la tua voce</strong>
      <span>Bastano tre brevi registrazioni per contare molto meglio →</span>
    </a>
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
    padding: 18px 20px 24px;
    gap: 14px;
    user-select: none;
    -webkit-user-select: none;
  }

  header {
    width: 100%;
    max-width: 420px;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .date::first-letter {
    text-transform: uppercase;
  }
  .today strong {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .today .of {
    color: var(--faint);
  }
  .done {
    color: var(--green);
    margin-left: 4px;
  }

  .ring-wrap {
    position: relative;
    width: min(78vw, 320px);
    aspect-ratio: 1;
    margin-top: 8px;
    border-radius: 50%;
    transition: box-shadow 0.25s;
  }
  .ring-wrap.speaking {
    box-shadow: 0 0 60px rgba(242, 181, 68, 0.16);
  }
  .ring {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  .ring circle {
    fill: none;
    stroke-width: 10;
  }
  .ring .track {
    stroke: var(--surface-2);
  }
  .ring .bar {
    stroke: var(--gold);
    stroke-linecap: round;
    transition: stroke-dashoffset 0.4s ease;
  }
  .ring .bar.full {
    stroke: var(--green);
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .count {
    font-size: clamp(4.5rem, 24vw, 7.5rem);
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.03em;
  }
  .count.pulse {
    animation: pulse 0.45s ease-out;
  }
  @keyframes pulse {
    0% {
      transform: scale(1.1);
      color: var(--gold);
    }
    100% {
      transform: scale(1);
    }
  }
  .unit {
    margin-top: 6px;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.9rem;
    min-height: 1.4rem;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--line);
    transition: background 0.15s, box-shadow 0.15s;
  }
  .dot.live {
    background: var(--faint);
  }
  .dot.on {
    background: var(--green);
    box-shadow: 0 0 10px var(--green);
  }
  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--faint);
  }

  button.go {
    margin-top: 6px;
    width: 84px;
    height: 84px;
    border-radius: 50%;
    border: none;
    background: var(--gold);
    color: #1b1406;
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 8px 28px rgba(242, 181, 68, 0.28);
    transition: transform 0.1s, background 0.2s, box-shadow 0.2s;
  }
  button.go:active {
    transform: scale(0.95);
  }
  button.go.stop {
    background: var(--red);
    color: #fff;
    box-shadow: 0 8px 28px rgba(229, 103, 95, 0.28);
  }
  button.go:disabled {
    opacity: 0.6;
  }
  button.go svg {
    width: 34px;
    height: 34px;
    fill: currentColor;
  }
  .go-label {
    margin-top: -6px;
    font-size: 0.8rem;
    color: var(--faint);
  }

  .sheet {
    width: 100%;
    max-width: 420px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 16px 18px;
    margin-top: 4px;
  }
  .sheet-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .sheet-title {
    font-weight: 600;
  }
  .sheet-sub {
    color: var(--muted);
    font-size: 0.85rem;
    margin-top: 2px;
    font-variant-numeric: tabular-nums;
  }
  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 4px;
  }
  .stepper button {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: transparent;
    font-size: 1.3rem;
    cursor: pointer;
  }
  .stepper button:active {
    background: var(--line);
  }
  .stepper span {
    min-width: 3ch;
    text-align: center;
    font-weight: 700;
    font-size: 1.15rem;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.4;
    margin: 10px 0 4px;
  }
  .hint a {
    color: var(--gold);
  }
  .sheet-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    margin-top: 6px;
  }
  button.link {
    background: none;
    border: none;
    padding: 6px 0;
    color: var(--muted);
    font-size: 0.82rem;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  button.link.danger {
    color: var(--red);
  }
  .saved {
    color: var(--green);
    font-size: 0.78rem;
    word-break: break-all;
    margin: 6px 0 0;
  }

  .tip {
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--gold-soft);
    color: var(--text);
    text-decoration: none;
    font-size: 0.88rem;
  }
  .tip span {
    color: var(--muted);
  }

  .error {
    color: var(--red);
    font-size: 0.85rem;
    text-align: center;
    max-width: 420px;
  }
</style>
