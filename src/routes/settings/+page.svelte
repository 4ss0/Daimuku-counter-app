<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { api, errText } from '$lib/api';
  import { LANGS, t, type Key } from '$lib/i18n';
  import { ACCENTS, prefs, updatePrefs, type Accent, type ThemePref } from '$lib/prefs';

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

  onMount(async () => {
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
</style>
