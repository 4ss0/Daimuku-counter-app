<script lang="ts">
  // The first start. Where the app can save by itself (Android), it first
  // asks whether there is a save to load back (a new phone, the app
  // installed again); otherwise, or for a new user, it explains on the
  // voice page why the recordings are needed and how to make them.
  import { fade } from 'svelte/transition';
  import { api, errText } from '$lib/api';
  import { locale, t } from '$lib/i18n';
  import { initPrefs, updatePrefs } from '$lib/prefs';
  import { useAutosave } from '$lib/autosave';
  import { autosaveKeep, autosaveOpen } from '$lib/native';

  /** 'ask': a save to load? 'intro': the voice recordings. */
  export let step: 'ask' | 'intro';
  /** A new user: on to the voice page and its explanation. */
  export let onvoice: () => void;
  /** The welcome is over; `takes`: voice recordings of a save just loaded (-1: none loaded). */
  export let ondone: (takes: number) => void;

  let busy = false;
  let error = '';
  /** A save picked in the "open" screen, waiting for the user's yes. */
  let found: { content: string; date: string; sessions: number; takes: number } | null = null;

  $: steps = $t('voice.introSteps').split('\n');

  async function pick() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const content = await autosaveOpen();
      if (content === null) return;
      const head = JSON.parse(content);
      if (head?.format !== 'daimoku-counter-backup') throw new Error('backup-invalid');
      found = {
        content,
        date: new Date(head.created_at).toLocaleString($locale, { dateStyle: 'medium', timeStyle: 'short' }),
        sessions: Array.isArray(head.history?.sessions) ? head.history.sessions.length : 0,
        takes: Array.isArray(head.takes) ? head.takes.length : 0,
      };
    } catch {
      error = $t('err.backupInvalid');
      void autosaveKeep(false);
    } finally {
      busy = false;
    }
  }

  function notThis() {
    found = null;
    void autosaveKeep(false);
  }

  async function load() {
    if (!found || busy) return;
    busy = true;
    error = '';
    try {
      const r = await api.restoreBackup(found.content);
      await initPrefs();
      // from now on the app keeps saving to the same file
      useAutosave(await autosaveKeep(true));
      await updatePrefs({ onboarded: true });
      ondone(r.takes);
    } catch (e) {
      error = errText(e);
    } finally {
      busy = false;
    }
  }

  async function understood() {
    await updatePrefs({ onboarded: true });
    ondone(-1);
  }
</script>

<div class="welcome" role="dialog" aria-modal="true" aria-labelledby="welcome-title" transition:fade={{ duration: 200 }}>
  <div class="card glass">
    {#if step === 'ask'}
      <img src="/app-icon.png" alt="" width="56" height="56" />
      <h2 id="welcome-title">{$t('welcome.title')}</h2>
      {#if found}
        <p class="found">{$t('welcome.found', { d: found.date })}</p>
        <p>{$t('welcome.foundText', { s: found.sessions, n: found.takes })}</p>
        <div class="actions">
          <button class="btn ghost" on:click={notThis} disabled={busy}>{$t('common.cancel')}</button>
          <button class="btn primary" on:click={load} disabled={busy}>{busy ? $t('set.working') : $t('welcome.confirm')}</button>
        </div>
      {:else}
        <p>{$t('welcome.text')}</p>
        <div class="actions column">
          <button class="btn primary" on:click={pick} disabled={busy}>{$t('welcome.load')}</button>
          <button class="btn ghost" on:click={onvoice} disabled={busy}>{$t('welcome.fresh')}</button>
        </div>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
    {:else}
      <svg class="mic" viewBox="0 0 24 24" aria-hidden="true">
        <rect x="9" y="3.5" width="6" height="11" rx="3" />
        <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v2.5" />
      </svg>
      <h2 id="welcome-title">{$t('voice.introTitle')}</h2>
      <p>{$t('voice.introText')}</p>
      <ol>
        {#each steps as s}<li>{s}</li>{/each}
      </ol>
      <div class="actions">
        <button class="btn primary" on:click={understood}>{$t('voice.introOk')}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .welcome {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: calc(16px + env(safe-area-inset-top)) 16px calc(16px + env(safe-area-inset-bottom));
    background: rgba(0, 0, 0, 0.4);
    -webkit-backdrop-filter: blur(6px);
    backdrop-filter: blur(6px);
    overflow-y: auto;
  }
  .card {
    width: min(100%, 380px);
    max-height: 100%;
    overflow-y: auto;
    padding: 24px 20px 20px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    animation: pop-in 0.4s cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  img {
    border-radius: 14px;
    margin-bottom: 10px;
  }
  .mic {
    width: 52px;
    height: 52px;
    margin-bottom: 8px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linecap: round;
  }
  h2 {
    margin: 0 0 10px;
    font-size: 1.25rem;
  }
  p {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 0.92rem;
    line-height: 1.45;
  }
  .found {
    color: var(--text);
    font-weight: 600;
  }
  ol {
    align-self: stretch;
    margin: 4px 0 14px;
    padding: 12px 14px 12px 34px;
    border-radius: 12px;
    background: var(--surface-2);
    text-align: left;
    font-size: 0.88rem;
    line-height: 1.45;
  }
  li + li {
    margin-top: 6px;
  }
  .actions {
    align-self: stretch;
    display: flex;
    gap: 10px;
    margin-top: 6px;
  }
  .actions.column {
    flex-direction: column;
  }
  .btn {
    flex: 1;
    border: none;
    border-radius: 12px;
    padding: 12px 10px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .btn.ghost {
    background: var(--surface-2);
    color: var(--text);
  }
  .btn:disabled {
    opacity: 0.55;
  }
  .error {
    margin: 12px 0 0;
    color: var(--red);
  }
  @keyframes pop-in {
    from {
      transform: scale(0.9);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      animation: none;
    }
  }
</style>
