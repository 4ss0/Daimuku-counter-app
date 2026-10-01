<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, errText, fmtClock, type PersonalProfile, type ValidationResult } from '$lib/api';
  import { t, type Key } from '$lib/i18n';
  import { hasNative, shareFile } from '$lib/native';

  type Speed = 'slow' | 'medium' | 'fast';
  const SPEEDS: { id: Speed; label: Key; n: number; how: Key }[] = [
    { id: 'slow', label: 'voice.slow', n: 3, how: 'voice.howSlow' },
    { id: 'medium', label: 'voice.medium', n: 5, how: 'voice.howMedium' },
    { id: 'fast', label: 'voice.fast', n: 10, how: 'voice.howFast' },
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
  let confirmDelete: number | null = null;
  let errorText = '';
  let voiceMsg = '';
  let voiceFile: HTMLInputElement;
  let pendingVoice: { content: string; n: number } | null = null;

  async function shareVoice() {
    errorText = '';
    voiceMsg = '';
    try {
      const f = await api.exportVoice();
      if (hasNative()) await shareFile(f, 'application/json', $t('voice.share'));
      else voiceMsg = $t('set.fileSavedIn', { p: f.path });
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function pickVoice() {
    errorText = '';
    voiceMsg = '';
    const file = voiceFile.files?.[0];
    voiceFile.value = '';
    if (!file) return;
    try {
      const content = await file.text();
      const head = JSON.parse(content);
      const ok = head?.format === 'daimoku-counter-voice' || head?.format === 'daimoku-counter-backup';
      const n = Array.isArray(head?.takes) ? head.takes.length : 0;
      if (!ok || n === 0) throw new Error('invalid');
      pendingVoice = { content, n };
    } catch {
      errorText = $t('voice.importInvalid');
    }
  }

  async function confirmImport() {
    if (!pendingVoice || busy) return;
    busy = true;
    try {
      const r = await api.importVoice(pendingVoice.content);
      voiceMsg = $t('voice.imported', { n: r.takes });
      await loadProfile();
    } catch (e) {
      errorText = String(e).includes('voice-invalid') ? $t('voice.importInvalid') : errText(e);
    } finally {
      pendingVoice = null;
      busy = false;
    }
  }

  $: sp = SPEEDS.find((s) => s.id === speed)!;
  $: done = coverage(profile);

  function speedOf(ms: number): Speed {
    if (ms >= 2600) return 'slow';
    if (ms >= 1150) return 'medium';
    return 'fast';
  }

  function takeMs(tk: PersonalProfile['takes'][number]): number {
    return tk.period_ms ?? (tk.duration_secs * 1000) / Math.max(1, tk.n_daimoku);
  }

  function coverage(p: PersonalProfile | null): Record<Speed, boolean> {
    const c = { slow: false, medium: false, fast: false };
    for (const tk of p?.takes ?? []) c[speedOf(takeMs(tk))] = true;
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
        errorText = $t('voice.tooShort');
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

  async function removeTake(id: number) {
    busy = true;
    try {
      profile = await api.deleteTake(id);
      confirmDelete = null;
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
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
  <h1>{$t('voice.title')}</h1>
  <p class="lead glass">{$t('voice.lead')}</p>

  <div class="checks">
    {#each SPEEDS as s}
      <div class="check glass" class:ok={done[s.id]}>
        <span class="mark">{done[s.id] ? '✓' : ''}</span>{$t(s.label)}
      </div>
    {/each}
  </div>

  <section class="card glass rec">
    {#if !recording && !result}
      <div class="seg">
        {#each SPEEDS as s}
          <button class:on={speed === s.id} on:click={() => pick(s.id)}>{$t(s.label)}</button>
        {/each}
      </div>
      <div class="howmany">
        <span>{$t('voice.howMany')}</span>
        <div class="stepper">
          <button on:click={() => (n = Math.max(1, n - 1))} aria-label={$t('common.less')}>−</button>
          <span>{n}</span>
          <button on:click={() => (n = Math.min(30, n + 1))} aria-label={$t('common.more')}>+</button>
        </div>
      </div>
      <p class="instr">{$t('voice.instr', { n, how: $t(sp.how) })}</p>
    {:else if recording}
      <div class="rec-live">
        <span class="dot" class:on={speaking}></span>
        <span>{$t('voice.recording', { n })}</span>
        <span class="clock">{fmtClock(secs)}</span>
      </div>
    {/if}

    {#if !result}
      <button class="go" class:stop={recording} on:click={recording ? stop : start} disabled={busy} aria-label={recording ? $t('voice.finish') : $t('voice.record')}>
        {#if recording}
          <svg viewBox="0 0 24 24"><rect x="6.5" y="6.5" width="11" height="11" rx="2.5" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="6.5" /></svg>
        {/if}
      </button>
      <div class="go-label">{recording ? $t('voice.finish') : $t('voice.record')}</div>
    {:else}
      <div class="result" class:good={result.ok}>
        <div class="r-big">{result.detected} <span>{$t('voice.of', { n: result.expected })}</span></div>
        <div class="r-sub">
          {#if added}
            {$t('voice.added')}
          {:else if result.ok}
            {$t('voice.okAll')}
          {:else}
            {$t('voice.notAll')}
          {/if}
        </div>
      </div>
      <div class="actions">
        {#if added}
          <button class="btn primary" on:click={discard}>{$t('voice.another')}</button>
        {:else}
          <button class="btn ghost" on:click={discard} disabled={busy}>{$t('voice.discard')}</button>
          <button class="btn primary" on:click={add} disabled={busy}>{busy ? $t('voice.learning') : $t('common.add')}</button>
        {/if}
      </div>
      <p class="note">{$t('voice.note', { n: result.expected })}</p>
    {/if}
  </section>

  <section class="card glass">
    <div class="row">
      <div class="c-title">{$t('voice.learned')}</div>
      <div class="c-count">{profile?.takes.length ?? 0}</div>
    </div>
    {#if profile && profile.takes.length > 0}
      <ul>
        {#each profile.takes as tk (tk.id)}
          <li>
            <span class="tk-speed" data-speed={speedOf(takeMs(tk))}>{$t(SPEEDS.find((s) => s.id === speedOf(takeMs(tk)))!.label)}</span>
            <span class="tk-main">
              {$t('voice.takeLine', { n: tk.n_daimoku })}
              <span class="muted">· {$t('voice.each', { s: (takeMs(tk) / 1000).toFixed(1) })}</span>
            </span>
            {#if confirmDelete === tk.id}
              <span class="tk-confirm">
                <button class="mini ghost" on:click={() => (confirmDelete = null)}>{$t('common.no')}</button>
                <button class="mini danger" on:click={() => removeTake(tk.id)} disabled={busy}>{$t('common.delete')}</button>
              </span>
            {:else}
              <button class="icon-btn" on:click={() => (confirmDelete = tk.id)} aria-label={$t('voice.deleteConfirm')}>
                <svg viewBox="0 0 24 24"><path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12" /></svg>
              </button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if confirmReset}
        <div class="confirm">
          <span>{$t('voice.resetConfirm')}</span>
          <button class="btn ghost small" on:click={() => (confirmReset = false)}>{$t('common.no')}</button>
          <button class="btn danger small" on:click={resetVoice}>{$t('voice.resetYes')}</button>
        </div>
      {:else}
        <button class="reset" on:click={() => (confirmReset = true)}>{$t('voice.reset')}</button>
      {/if}
    {:else}
      <p class="muted small-text">{$t('voice.none')}</p>
    {/if}
    {#if pendingVoice}
      <div class="confirm">
        <span>{$t('voice.importConfirm', { n: pendingVoice.n })}</span>
        <button class="btn ghost small" on:click={() => (pendingVoice = null)}>{$t('common.no')}</button>
        <button class="btn primary small" on:click={confirmImport} disabled={busy}>{$t('common.yes')}</button>
      </div>
    {:else}
      <div class="voice-io">
        {#if profile && profile.takes.length > 0}
          <button class="btn ghost small" on:click={shareVoice} disabled={busy}>{$t('voice.share')}</button>
        {/if}
        <button class="btn ghost small" on:click={() => voiceFile.click()} disabled={busy}>{$t('voice.import')}</button>
      </div>
    {/if}
    {#if voiceMsg}<p class="muted small-text">{voiceMsg}</p>{/if}
    <input class="hidden-file" type="file" accept=".json,application/json" bind:this={voiceFile} on:change={pickVoice} />
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
    gap: 12px;
  }
  h1 {
    font-size: 1.5rem;
    margin: 4px 2px 0;
    text-shadow: var(--title-shadow);
  }
  .lead {
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.45;
    margin: 0;
    padding: 12px 14px;
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
    border-radius: 12px;
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
    color: var(--text);
  }
  .check.ok .mark {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }

  .card {
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
    background: var(--surface-2);
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
    background: var(--accent);
    color: var(--accent-ink);
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
    line-height: 1.4;
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
    color: var(--accent);
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
    background: var(--accent);
    color: var(--accent-ink);
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
    color: var(--accent);
    font-weight: 700;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--line);
    font-size: 0.9rem;
    min-height: 48px;
  }
  .tk-speed {
    font-size: 0.72rem;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    white-space: nowrap;
  }
  .tk-main {
    flex: 1;
    min-width: 0;
  }
  .muted {
    color: var(--muted);
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--faint);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .icon-btn svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .icon-btn:active {
    color: var(--red);
  }
  .tk-confirm {
    display: flex;
    gap: 6px;
  }
  .mini {
    border: none;
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .mini.ghost {
    background: var(--surface-2);
  }
  .mini.danger {
    background: var(--red);
    color: #fff;
  }
  .small-text {
    font-size: 0.85rem;
    margin: 8px 0 0;
  }
  .voice-io {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .hidden-file {
    display: none;
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
