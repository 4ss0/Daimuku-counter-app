<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { api, errText, fmtNum } from '$lib/api';
  import { splitGoal, type GoalPeriod } from '$lib/stats';
  import { LANGS, detectLang, langName, locale, t, type Key, type Lang } from '$lib/i18n';
  import { ACCENTS, STATS_PERIODS, initPrefs, prefs, resolvedTheme, updatePrefs, type Accent, type ThemePref } from '$lib/prefs';
  import { BACKGROUNDS, backgroundUrl, customBackgroundUrl, loadCustomBackground, setCustomBackground, type Background } from '$lib/background';
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

  // ---- language: the device's ("auto") or one of the list, with a search ----
  const deviceLang = detectLang();
  let langOpen = false;
  let langQuery = '';
  /** Lower case, without accents, for searching. */
  const fold = (s: string) => s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '');
  $: langWords = fold(langQuery).split(/\s+/).filter(Boolean);
  $: langsShown = LANGS.filter((l) => {
    const text = fold(`${l.name} ${l.english} ${l.id}`);
    return langWords.every((w) => text.includes(w));
  });
  function pickLang(l: 'auto' | Lang) {
    updatePrefs({ lang: l });
    langOpen = false;
    langQuery = '';
  }
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

  const PERIOD_GOALS: { id: GoalPeriod; label: Key }[] = [
    { id: 'week', label: 'goal.weekly' },
    { id: 'month', label: 'goal.monthly' },
    { id: 'year', label: 'goal.yearly' },
  ];

  /** Weekly / monthly / yearly goal from its field (empty or 0 = none). */
  function savePeriodGoal(id: GoalPeriod, raw: string) {
    const v = parseInt(raw, 10);
    updatePrefs({ goals: { ...$prefs.goals, [id]: Number.isFinite(v) && v > 0 ? v : 0 } });
  }

  // background: one of the pictures, or the user's own
  let bgFile: HTMLInputElement;
  let customUrl: string | null = null;
  let bgError = '';
  onMount(async () => {
    if (await loadCustomBackground()) customUrl = customBackgroundUrl();
  });
  function pickBackground(bg: Background) {
    bgError = '';
    if (bg === 'custom' && !customUrl) bgFile.click();
    else updatePrefs({ background: bg });
  }
  async function chooseImage() {
    const file = bgFile.files?.[0];
    bgFile.value = '';
    if (!file) return;
    bgError = '';
    try {
      await setCustomBackground(file);
      customUrl = customBackgroundUrl();
      updatePrefs({ background: 'custom' });
    } catch {
      bgError = $t('bg.error');
    }
  }

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
    <button class="lang-now" on:click={() => (langOpen = !langOpen)} aria-expanded={langOpen}>
      <span>{$prefs.lang === 'auto' ? `${$t('set.langAuto')} · ${langName(deviceLang)}` : langName($prefs.lang)}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true" class:up={langOpen}><path d="M6 9l6 6 6-6" /></svg>
    </button>
    {#if langOpen}
      <input class="lang-search" type="search" placeholder={$t('set.langSearch')} aria-label={$t('set.langSearch')} bind:value={langQuery} />
      <ul class="lang-list">
        {#if !langQuery.trim()}
          <li>
            <button class:on={$prefs.lang === 'auto'} on:click={() => pickLang('auto')}>
              <span>{$t('set.langAuto')}</span><small>{$t('set.langAutoHint', { lang: langName(deviceLang) })}</small>
            </button>
          </li>
        {/if}
        {#each langsShown as l (l.id)}
          <li>
            <button class:on={$prefs.lang === l.id} on:click={() => pickLang(l.id)}>
              <span lang={l.id}>{l.name}</span>{#if l.english !== l.name}<small>{l.english}</small>{/if}
            </button>
          </li>
        {:else}
          <li class="none">{$t('set.langNone')}</li>
        {/each}
      </ul>
    {/if}
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
    <div class="label">{$t('bg.title')}</div>
    <div class="bgs">
      {#each BACKGROUNDS as bg}
        {@const url = bg === 'custom' ? customUrl : backgroundUrl(bg, $resolvedTheme)}
        <button class="bg-tile" class:on={$prefs.background === bg} on:click={() => pickBackground(bg)} aria-pressed={$prefs.background === bg}>
          <span class="thumb" style={url ? `background-image:url('${url}')` : ''}>
            {#if bg === 'custom' && !url}<span class="plus">+</span>{/if}
          </span>
          <span class="bg-name">{$t(`bg.${bg}`)}</span>
        </button>
      {/each}
    </div>
    {#if customUrl}
      <button class="link-btn" on:click={() => bgFile.click()}>{$t('bg.change')}</button>
    {/if}
    <p class="muted small-hint bg-hint">{$t('bg.hint')}</p>
    {#if bgError}<p class="error">{bgError}</p>{/if}
    <input class="hidden-file" type="file" accept="image/*" bind:this={bgFile} on:change={chooseImage} />
  </section>

  <section class="card glass">
    <div class="label">{$t('set.statsPeriod')}</div>
    <div class="options five">
      {#each STATS_PERIODS as sp}
        <button class:on={$prefs.statsPeriod === sp} on:click={() => updatePrefs({ statsPeriod: sp })}>{$t(`stats.${sp}` as Key)}</button>
      {/each}
    </div>
  </section>

  <section class="card glass">
    <div class="label">{$t('goal.title')}</div>
    <div class="row goal-row">
      <span>{$t('goal.daily')}</span>
      <form class="inline" on:submit|preventDefault={saveGoal}>
        <input type="number" inputmode="numeric" min="1" bind:value={goalInput} on:blur={saveGoal} aria-label={$t('goal.daily')} />
        <span class="unit">{$t('count.unit')}</span>
      </form>
    </div>
    <p class="muted small-hint goal-split">
      {$t('goal.split', { m: fmtNum(splitGoal(goal).morning, $locale), e: fmtNum(splitGoal(goal).evening, $locale) })}
    </p>
    {#each PERIOD_GOALS as g}
      <div class="row goal-row">
        <span>{$t(g.label)}</span>
        <form class="inline" on:submit|preventDefault={(e) => savePeriodGoal(g.id, new FormData(e.currentTarget).get('v')?.toString() ?? '')}>
          <input
            name="v"
            type="number"
            inputmode="numeric"
            min="0"
            placeholder={$t('goal.none')}
            value={$prefs.goals[g.id] || ''}
            on:blur={(e) => savePeriodGoal(g.id, e.currentTarget.value)}
            aria-label={$t(g.label)}
          />
          <span class="unit">{$t('count.unit')}</span>
        </form>
      </div>
    {/each}
    <p class="muted small-hint">{$t('goal.hint')}</p>
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

  <section class="card glass">
    <div class="label">{$t('set.privacy')}</div>
    <p class="muted small privacy">{$t('set.privacyText')}</p>
    <div class="options data-actions">
      <a class="wide" href="/privacy">{$t('set.privacyOpen')}</a>
    </div>
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
  .lang-now {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 11px 12px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .lang-now svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.2s;
  }
  .lang-now svg.up {
    transform: rotate(180deg);
  }
  .lang-search {
    width: 100%;
    margin-top: 10px;
    text-align: left;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .lang-list {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-height: 300px;
    overflow-y: auto;
    border-radius: 12px;
    border: 1px solid var(--line);
  }
  .lang-list li + li {
    border-top: 1px solid var(--line);
  }
  .lang-list button {
    width: 100%;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .lang-list button small {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .lang-list button.on {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }
  .lang-list button.on small {
    color: inherit;
    opacity: 0.8;
  }
  .lang-list .none {
    padding: 12px;
    color: var(--muted);
    font-size: 0.88rem;
  }
  .options {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .options.three {
    grid-template-columns: repeat(3, 1fr);
  }
  .options.five {
    grid-template-columns: repeat(auto-fit, minmax(84px, 1fr));
  }
  .options button,
  .options a {
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
  .options a {
    color: inherit;
    text-decoration: none;
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
  .privacy {
    margin: 0 0 12px;
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
  .bgs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 10px;
  }
  .bg-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
  }
  .thumb {
    width: 100%;
    aspect-ratio: 9 / 14;
    border-radius: 12px;
    border: 2px solid var(--line);
    background: var(--surface-2) center 30% / cover no-repeat;
    display: grid;
    place-items: center;
  }
  .bg-tile.on .thumb {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .plus {
    font-size: 2rem;
    color: var(--muted);
  }
  .bg-name {
    font-size: 0.78rem;
    color: var(--muted);
    text-align: center;
  }
  .bg-tile.on .bg-name {
    color: var(--accent);
    font-weight: 600;
  }
  .link-btn {
    margin-top: 10px;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 0.88rem;
    cursor: pointer;
  }
  .bg-hint {
    margin-top: 10px;
  }
  .goal-row {
    margin-top: 8px;
  }
  .goal-split {
    margin: 4px 0 6px;
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
