<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, errText, fmtClock, type PersonalProfile, type ValidationResult } from '$lib/api';

  type Speed = 'slow' | 'medium' | 'fast';
  const SPEEDS: { id: Speed; label: string; n: number; how: string }[] = [
    { id: 'slow', label: 'Lento', n: 3, how: 'molto lentamente, come quando inizi' },
    { id: 'medium', label: 'Medio', n: 5, how: 'al tuo ritmo abituale' },
    { id: 'fast', label: 'Veloce', n: 10, how: 'veloce, come quando reciti a lungo' },
  ];

  let profile: PersonalProfile | null = null;
  let speed: Speed = 'medium';
  let n = 5;
  let recording = false;
  let busy = false;
  let secs = 0;
  let speaking = false;
  let tick: ReturnType<typeof setInterval> | null = null;
  let t0 = 0;

  let takeIndex: number | null = null;
  let result: ValidationResult | null = null;
  let added = false;
  let confirmReset = false;
  let errorText = '';

  $: sp = SPEEDS.find((s) => s.id === speed)!;
  $: done = coverage(profile);

  function coverage(p: PersonalProfile | null): Record<Speed, boolean> {
    const c = { slow: false, medium: false, fast: false };
    for (const t of p?.takes ?? []) {
      const ms = t.period_ms ?? (t.duration_secs * 1000) / Math.max(1, t.n_daimoku);
      if (ms >= 2600) c.slow = true;
      else if (ms >= 1150) c.medium = true;
      else c.fast = true;
    }
    return c;
  }

  function pick(s: Speed) {
    if (recording) return;
    speed = s;
    n = SPEEDS.find((x) => x.id === s)!.n;
  }

  async function loadProfile() {
    try {
      profile = await api.profile();
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function start() {
    if (busy) return;
    busy = true;
    errorText = '';
    result = null;
    takeIndex = null;
    added = false;
    try {
      await api.startRecording();
      recording = true;
      t0 = Date.now();
      secs = 0;
      tick = setInterval(async () => {
        secs = (Date.now() - t0) / 1000;
        try {
          speaking = (await api.liveView()).speaking;
        } catch {
          speaking = false;
        }
      }, 200);
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    if (busy || !recording) return;
    busy = true;
    if (tick) clearInterval(tick);
    tick = null;
    speaking = false;
    try {
      const meta = await api.stopRecording(n);
      recording = false;
      takeIndex = meta.id;
      if (meta.duration_ms < 1500) {
        errorText = 'Registrazione troppo breve, riprova.';
        takeIndex = null;
      } else {
        result = await api.validateTake(meta.id);
      }
    } catch (e) {
      errorText = errText(e);
      recording = false;
    } finally {
      busy = false;
    }
  }

  async function add() {
    if (takeIndex === null || busy) return;
    busy = true;
    try {
      profile = await api.addTakeToProfile(takeIndex);
      added = true;
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  function discard() {
    result = null;
    takeIndex = null;
    added = false;
  }

  async function resetVoice() {
    try {
      await api.clearProfile();
      confirmReset = false;
      await loadProfile();
    } catch (e) {
      errorText = errText(e);
    }
  }

  onMount(loadProfile);
  onDestroy(() => {
    if (tick) clearInterval(tick);
    if (recording) api.stopRecording(n).catch(() => {});
  });
</script>

<div class="page">
  <h1>La tua voce</h1>
  <p class="lead">
    L'app riconosce «Nam-myoho-renge-kyo» sillaba per sillaba. Registrando la tua voce a tre
    velocità impara il tuo modo di recitare e conta molto meglio.
  </p>

  <div class="checks">
    {#each SPEEDS as s}
      <div class="check" class:ok={done[s.id]}>
        <span class="mark">{done[s.id] ? '✓' : ''}</span>{s.label}
      </div>
    {/each}
  </div>

  <section class="card rec">
    {#if !recording && !result}
      <div class="seg">
        {#each SPEEDS as s}
          <button class:on={speed === s.id} on:click={() => pick(s.id)}>{s.label}</button>
        {/each}
      </div>
      <div class="howmany">
        <span>Daimoku da recitare</span>
        <div class="stepper">
          <button on:click={() => (n = Math.max(1, n - 1))} aria-label="Meno">−</button>
          <span>{n}</span>
          <button on:click={() => (n = Math.min(30, n + 1))} aria-label="Più">+</button>
        </div>
      </div>
      <p class="instr">Premi e recita <strong>{n}</strong> daimoku {sp.how}. Poi premi di nuovo.</p>
    {:else if recording}
      <div class="rec-live">
        <span class="dot" class:on={speaking}></span>
        <span>Registrazione · recita {n} daimoku</span>
        <span class="clock">{fmtClock(secs)}</span>
      </div>
    {/if}

    {#if !result}
      <button class="go" class:stop={recording} on:click={recording ? stop : start} disabled={busy} aria-label={recording ? 'Fine' : 'Registra'}>
        {#if recording}
          <svg viewBox="0 0 24 24"><rect x="6.5" y="6.5" width="11" height="11" rx="2.5" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="6.5" /></svg>
        {/if}
      </button>
      <div class="go-label">{recording ? 'Fine' : 'Registra'}</div>
    {:else}
      <div class="result" class:good={result.ok}>
        <div class="r-big">{result.detected} <span>su {result.expected}</span></div>
        <div class="r-sub">
          {#if added}
            Aggiunta alla tua voce ✓
          {:else if result.ok}
            Riconosciuti tutti. Aggiungila comunque: rende il conteggio più sicuro.
          {:else}
            Non li ha riconosciuti tutti: aggiungendola l'app impara proprio da questo.
          {/if}
        </div>
      </div>
      <div class="actions">
        {#if added}
          <button class="btn primary" on:click={discard}>Registra un'altra</button>
        {:else}
          <button class="btn ghost" on:click={discard} disabled={busy}>Scarta</button>
          <button class="btn primary" on:click={add} disabled={busy}>{busy ? 'Imparo…' : 'Aggiungi'}</button>
        {/if}
      </div>
      <p class="note">Aggiungi solo registrazioni in cui hai recitato esattamente {result.expected} daimoku.</p>
    {/if}
  </section>

  <section class="card">
    <div class="row">
      <div class="c-title">Registrazioni imparate</div>
      <div class="c-count">{profile?.takes.length ?? 0}</div>
    </div>
    {#if profile && profile.takes.length > 0}
      <ul>
        {#each profile.takes as t}
          <li>
            <span>{t.n_daimoku} daimoku</span>
            <span class="muted">
              {t.period_ms ? `~${(t.period_ms / 1000).toFixed(1)} s ciascuno` : `${t.duration_secs.toFixed(0)} s`}
            </span>
          </li>
        {/each}
      </ul>
      {#if confirmReset}
        <div class="confirm">
          <span>Cancellare tutte le registrazioni?</span>
          <button class="btn ghost small" on:click={() => (confirmReset = false)}>No</button>
          <button class="btn danger small" on:click={resetVoice}>Sì, cancella</button>
        </div>
      {:else}
        <button class="reset" on:click={() => (confirmReset = true)}>Ricomincia da zero</button>
      {/if}
    {:else}
      <p class="muted small-text">Nessuna ancora: per ora l'app usa esempi generici.</p>
    {/if}
  </section>

  {#if errorText}<p class="error">{errorText}</p>{/if}
</div>

<style>
  .page {
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: 18px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    font-size: 1.5rem;
    margin: 4px 0 0;
  }
  .lead {
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.45;
    margin: 0;
  }
  .checks {
    display: flex;
    gap: 8px;
  }
  .check {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 0;
    border-radius: 10px;
    border: 1px dashed var(--line);
    color: var(--faint);
    font-size: 0.85rem;
  }
  .check .mark {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1.5px solid var(--faint);
    display: grid;
    place-items: center;
    font-size: 0.7rem;
  }
  .check.ok {
    border-style: solid;
    border-color: rgba(111, 211, 154, 0.35);
    color: var(--text);
  }
  .check.ok .mark {
    background: var(--green);
    border-color: var(--green);
    color: #0b2014;
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 16px;
  }
  .rec {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .seg {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    background: var(--bg);
    border-radius: 12px;
    padding: 3px;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 9px 0;
    border-radius: 9px;
    font-weight: 500;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-2);
    color: var(--text);
  }
  .howmany {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 3px;
  }
  .stepper button {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: none;
    background: transparent;
    font-size: 1.2rem;
    cursor: pointer;
  }
  .stepper span {
    min-width: 3ch;
    text-align: center;
    font-weight: 700;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .instr {
    margin: 0;
    text-align: center;
    color: var(--muted);
    font-size: 0.88rem;
  }
  .instr strong {
    color: var(--text);
  }
  .rec-live {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.9rem;
    padding: 12px 0 4px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    animation: blink 1.2s infinite;
  }
  .dot.on {
    background: var(--green);
    box-shadow: 0 0 10px var(--green);
    animation: none;
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--faint);
  }
  button.go {
    width: 76px;
    height: 76px;
    border-radius: 50%;
    border: 3px solid var(--line);
    background: var(--surface-2);
    color: var(--red);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  button.go.stop {
    background: var(--red);
    border-color: var(--red);
    color: #fff;
  }
  button.go svg {
    width: 32px;
    height: 32px;
    fill: currentColor;
  }
  .go-label {
    margin-top: -8px;
    font-size: 0.8rem;
    color: var(--faint);
  }
  .result {
    text-align: center;
    padding: 6px 0 0;
  }
  .r-big {
    font-size: 2.6rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--gold);
  }
  .result.good .r-big {
    color: var(--green);
  }
  .r-big span {
    font-size: 1.1rem;
    color: var(--muted);
    font-weight: 500;
  }
  .r-sub {
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.4;
    max-width: 30ch;
    margin: 4px auto 0;
  }
  .actions {
    display: flex;
    gap: 10px;
    width: 100%;
  }
  .btn {
    flex: 1;
    border: none;
    border-radius: 12px;
    padding: 12px 10px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.small {
    flex: 0;
    padding: 8px 12px;
    white-space: nowrap;
  }
  .btn.primary {
    background: var(--gold);
    color: #1b1406;
  }
  .btn.ghost {
    background: var(--surface-2);
    color: var(--text);
  }
  .btn.danger {
    background: var(--red);
    color: #fff;
  }
  .btn:disabled {
    opacity: 0.5;
  }
  .note {
    margin: 0;
    font-size: 0.75rem;
    color: var(--faint);
    text-align: center;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .c-title {
    font-weight: 600;
  }
  .c-count {
    color: var(--gold);
    font-weight: 700;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
  }
  li {
    display: flex;
    justify-content: space-between;
    padding: 9px 0;
    border-top: 1px solid var(--line);
    font-size: 0.9rem;
  }
  .muted {
    color: var(--muted);
  }
  .small-text {
    font-size: 0.85rem;
    margin: 8px 0 0;
  }
  .reset {
    margin-top: 10px;
    background: none;
    border: none;
    color: var(--red);
    font-size: 0.85rem;
    padding: 4px 0;
    cursor: pointer;
  }
  .confirm {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 0.85rem;
    flex-wrap: wrap;
  }
  .confirm span {
    flex: 1;
  }
  .error {
    color: var(--red);
    font-size: 0.85rem;
  }
</style>
