<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { api, errText } from '$lib/api';
  import { LANGS, locale, t, type Key } from '$lib/i18n';
  import { ACCENTS, initPrefs, prefs, updatePrefs, type Accent, type ThemePref } from '$lib/prefs';
  import { hasNative, saveFile, shareFile, type ExportedFile } from '$lib/native';

  const THEMES: { id: ThemePref; label: Key }[] = [
    { id: 'system', label: 'set.themeSystem' },
    { id: 'light', label: 'set.themeLight' },
    { id: 'dark', label: 'set.themeDark' },
  ];
  // swatch colours (light-theme shade | dark-theme shade)
  const SWATCH: Record<Accent, [string, string]> = {
    lotus: ['#1f8a70', '#5cc9a7'],
    gold: ['#b07a1a', '#f2b544'],
    sakura: ['#c0507a', '#f29bbd'],
    indigo: ['#3c5aa8', '#93aaf2'],
  };

  let version = '0.1.0';
  let goal = 100;
  let goalInput = '';
  let errorText = '';
  let native = false;

  // backup / restore / export
  let working = false;
  let dataMsg = '';
  let pendingRestore: { content: string; date: string } | null = null;
  let fileInput: HTMLInputElement;

  onMount(async () => {
    native = hasNative();
    try {
      version = await getVersion();
    } catch {
      /* keep default */
    }
    try {
      goal = await api.getGoal();
      goalInput = String(goal);
    } catch (e) {
      errorText = errText(e);
    }
  });

  async function saveGoal() {
    const v = parseInt(goalInput, 10);
    if (!Number.isFinite(v) || v < 1) {
      goalInput = String(goal);
      return;
    }
    try {
      goal = await api.setGoal(v);
      goalInput = String(goal);
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function deliver(f: ExportedFile, mime: string, how: 'save' | 'share') {
    if (!native) {
      dataMsg = $t('set.fileSavedIn', { p: f.path });
      return;
    }
    if (how === 'share') {
      await shareFile(f, mime, f.name);
    } else if (await saveFile(f, mime)) {
      dataMsg = $t('set.fileSaved');
    }
  }

  async function backup(how: 'save' | 'share') {
    if (working) return;
    working = true;
    dataMsg = '';
    errorText = '';
    try {
      await deliver(await api.createBackup(), 'application/json', how);
    } catch (e) {
      errorText = errText(e);
    } finally {
      working = false;
    }
  }

  function csvCell(v: string | number): string {
    const s = String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  }

  async function exportCsv() {
    if (working) return;
    working = true;
    dataMsg = '';
    errorText = '';
    try {
      const sessions = (await api.listSessions()).slice().sort((a, b) => a.started_at.localeCompare(b.started_at));
      const pad = (n: number) => String(n).padStart(2, '0');
      const rows = sessions.map((s) => {
        const d = new Date(s.started_at);
        return [
          `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`,
          `${pad(d.getHours())}:${pad(d.getMinutes())}`,
          Math.round(s.duration_secs),
          s.count,
          s.detected,
          s.manual ? 1 : 0,
        ]
          .map(csvCell)
          .join(',');
      });
      const csv = '\ufeff' + [$t('csv.header'), ...rows].join('\r\n') + '\r\n';
      const now = new Date();
      const name = `daimoku-sessions-${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.csv`;
      await deliver(await api.writeTextExport(name, csv), 'text/csv', native ? 'share' : 'save');
    } catch (e) {
      errorText = errText(e);
    } finally {
      working = false;
    }
  }

  async function pickRestore() {
    dataMsg = '';
    errorText = '';
    const file = fileInput.files?.[0];
    fileInput.value = '';
    if (!file) return;
    try {
      const content = await file.text();
      let date = file.name;
      try {
        const head = JSON.parse(content);
        if (head?.format !== 'daimoku-counter-backup') throw new Error('backup-invalid');
        date = new Date(head.created_at).toLocaleString($locale, { dateStyle: 'medium', timeStyle: 'short' });
      } catch {
        errorText = $t('err.backupInvalid');
        return;
      }
      pendingRestore = { content, date };
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function confirmRestore() {
    if (!pendingRestore || working) return;
    working = true;
    try {
      const r = await api.restoreBackup(pendingRestore.content);
      pendingRestore = null;
      await initPrefs();
      goal = await api.getGoal();
      goalInput = String(goal);
      dataMsg = $t('set.restoreDone', { s: r.sessions, n: r.takes });
    } catch (e) {
      errorText = errText(e);
    } finally {
      working = false;
    }
  }
</script>

<div class="page">
  <h1>{$t('set.title')}</h1>

  <section class="card glass">
    <div class="label">{$t('set.language')}</div>
    <div class="options">
      <button class:on={$prefs.lang === 'auto'} on:click={() => updatePrefs({ lang: 'auto' })}>{$t('set.langAuto')}</button>
      {#each LANGS as l}
        <button class:on={$prefs.lang === l.id} on:click={() => updatePrefs({ lang: l.id })} lang={l.id}>{l.name}</button>
      {/each}
    </div>
  </section>

  <section class="card glass">
    <div class="label">{$t('set.theme')}</div>
    <div class="options three">
      {#each THEMES as th}
        <button class:on={$prefs.theme === th.id} on:click={() => updatePrefs({ theme: th.id })}>
          <span class="theme-dot" data-kind={th.id}></span>{$t(th.label)}
        </button>
      {/each}
    </div>
  </section>

  <section class="card glass">
    <div class="label">{$t('set.accent')}</div>
    <div class="swatches">
      {#each ACCENTS as a}
        <button class="swatch" class:on={$prefs.accent === a} on:click={() => updatePrefs({ accent: a })} aria-pressed={$prefs.accent === a}>
          <span class="chip" style="--c1:{SWATCH[a][0]};--c2:{SWATCH[a][1]}"></span>
          <span>{$t(`set.accent.${a}` as Key)}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="card glass">
    <div class="row">
      <div class="label">{$t('set.goal')}</div>
      <form class="inline" on:submit|preventDefault={saveGoal}>
        <input type="number" inputmode="numeric" min="1" bind:value={goalInput} on:blur={saveGoal} aria-label={$t('set.goal')} />
        <span class="unit">{$t('count.unit')}</span>
      </form>
    </div>
  </section>

  {#if native}
    <section class="card glass">
      <label class="toggle">
        <span>
          <span class="label">{$t('set.keepAwake')}</span>
          <span class="muted small-hint">{$t('set.keepAwakeHint')}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          checked={$prefs.keepAwake}
          on:change={(e) => updatePrefs({ keepAwake: e.currentTarget.checked })}
        />
      </label>
    </section>
  {/if}

  <section class="card glass">
    <div class="label">{$t('set.data')}</div>
    <p class="muted small-hint">{$t('set.dataHint')}</p>
    <div class="options data-actions">
      {#if native}
        <button on:click={() => backup('save')} disabled={working}>{$t('set.backupSave')}</button>
        <button on:click={() => backup('share')} disabled={working}>{$t('set.backupShare')}</button>
      {:else}
        <button class="wide" on:click={() => backup('save')} disabled={working}>{$t('set.backupCreate')}</button>
      {/if}
      <button class="wide" on:click={() => fileInput.click()} disabled={working}>{$t('set.restore')}</button>
      <button class="wide" on:click={exportCsv} disabled={working}>{$t('set.exportCsv')}</button>
    </div>
    <input class="hidden-file" type="file" bind:this={fileInput} on:change={pickRestore} />
    {#if pendingRestore}
      <div class="confirm">
        <p>{$t('set.restoreConfirm', { d: pendingRestore.date })}</p>
        <div class="confirm-actions">
          <button on:click={() => (pendingRestore = null)} disabled={working}>{$t('common.cancel')}</button>
          <button class="danger" on:click={confirmRestore} disabled={working}>{$t('common.yes')}</button>
        </div>
      </div>
    {/if}
    {#if working}<p class="muted small-hint">{$t('set.working')}</p>{/if}
    {#if dataMsg}<p class="ok-msg">{dataMsg}</p>{/if}
  </section>

  <section class="card glass about">
    <div class="label">{$t('set.about')}</div>
    <div class="app-line">
      <img src="/app-icon.png" alt="" width="44" height="44" />
      <div>
        <div class="app-name">Daimoku Counter</div>
        <div class="muted">{$t('set.version', { v: version })} · beta</div>
      </div>
    </div>
    <p class="muted small">{$t('set.beta')}</p>
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
  .card {
    padding: 14px 16px;
  }
  .label {
    font-weight: 600;
    margin-bottom: 10px;
  }
  .row .label {
    margin-bottom: 0;
  }
  .options {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .options.three {
    grid-template-columns: repeat(3, 1fr);
  }
  .options button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 8px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    cursor: pointer;
    font-size: 0.9rem;
  }
  .options button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .theme-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: linear-gradient(135deg, #f4f6f0 50%, #16171a 50%);
  }
  .theme-dot[data-kind='light'] {
    background: #f4f6f0;
  }
  .theme-dot[data-kind='dark'] {
    background: #16171a;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .swatch {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 4px;
    border-radius: 12px;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .swatch.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    font-weight: 600;
  }
  .chip {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--c1) 50%, var(--c2) 50%);
    box-shadow: 0 0 0 2px var(--surface-solid);
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  input {
    width: 6.5rem;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 10px;
    color: var(--text);
    padding: 9px 10px;
    font-size: 1rem;
    text-align: right;
  }
  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .unit {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .app-line {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .app-line img {
    border-radius: 11px;
  }
  .app-name {
    font-weight: 600;
  }
  .muted {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .small {
    font-size: 0.8rem;
    line-height: 1.4;
    margin: 10px 0 0;
  }
  .error {
    color: var(--red);
    font-size: 0.85rem;
  }
  .toggle {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 14px;
    cursor: pointer;
  }
  .toggle .label {
    display: block;
    margin-bottom: 4px;
  }
  .toggle input {
    appearance: none;
    -webkit-appearance: none;
    flex-shrink: 0;
    width: 46px;
    height: 28px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    position: relative;
    cursor: pointer;
    transition: background 0.2s;
  }
  .toggle input::after {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--muted);
    transition: transform 0.2s, background 0.2s;
  }
  .toggle input:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .toggle input:checked::after {
    transform: translateX(18px);
    background: var(--accent-ink);
  }
  .small-hint {
    display: block;
    font-size: 0.8rem;
    line-height: 1.4;
    margin: 0 0 10px;
  }
  .data-actions .wide {
    grid-column: 1 / -1;
  }
  .options button:disabled {
    opacity: 0.55;
  }
  .hidden-file {
    display: none;
  }
  .confirm {
    margin-top: 12px;
    padding: 12px;
    border-radius: 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
  }
  .confirm p {
    margin: 0 0 10px;
    font-size: 0.9rem;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .confirm-actions button {
    padding: 8px 14px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: transparent;
    cursor: pointer;
  }
  .confirm-actions .danger {
    background: var(--red);
    border-color: var(--red);
    color: #fff;
    font-weight: 600;
  }
  .ok-msg {
    color: var(--green);
    font-size: 0.85rem;
    margin: 10px 0 0;
    word-break: break-all;
  }
</style>
