<script lang="ts">
  import { onMount } from 'svelte';
  import BarChart from '$lib/BarChart.svelte';
  import { api, errText, fmtDuration, fmtNum, type SessionRecord } from '$lib/api';
  import { locale, t, type Key } from '$lib/i18n';
  import { share, sessionText, statsText } from '$lib/share';
  import { bestDay, fmtWhen, goalDaysThisMonth, periodStats, sessionsIn, streak, type Period } from '$lib/stats';

  const PERIODS: { id: Period; label: Key; prev: Key | null; title: Key | null }[] = [
    { id: 'day', label: 'stats.day', prev: 'stats.prevDay', title: 'stats.titleDay' },
    { id: 'week', label: 'stats.week', prev: 'stats.prevWeek', title: 'stats.titleWeek' },
    { id: 'month', label: 'stats.month', prev: 'stats.prevMonth', title: null },
    { id: 'year', label: 'stats.year', prev: 'stats.prevYear', title: null },
    { id: 'all', label: 'stats.all', prev: null, title: 'stats.titleAll' },
  ];

  let sessions: SessionRecord[] = [];
  let goal = 100;
  let period: Period = 'day';
  let errorText = '';
  let loaded = false;

  let editingGoal = false;
  let goalInput = '';
  let manualInput = '';
  let openId: number | null = null;
  let showAll = false;

  $: st = periodStats(sessions, period, new Date(), $locale);
  $: p = PERIODS.find((x) => x.id === period)!;
  $: heroTitle = p.title ? $t(p.title) : st.title;
  $: delta = st.prevTotal !== null && st.prevTotal > 0 ? Math.round(((st.total - st.prevTotal) / st.prevTotal) * 100) : null;
  $: days = streak(sessions);
  $: best = bestDay(sessions);
  $: goalMonth = goalDaysThisMonth(sessions, goal);
  $: list = sessionsIn(sessions, period);
  $: visible = showAll ? list : list.slice(0, 20);
  $: period, (showAll = false);
  $: words = { today: $t('common.today'), yesterday: $t('common.yesterday') };

  async function load() {
    try {
      [sessions, goal] = await Promise.all([api.listSessions(), api.getGoal()]);
    } catch (e) {
      errorText = errText(e);
    }
    loaded = true;
  }

  async function saveGoal() {
    const v = parseInt(goalInput, 10);
    if (!Number.isFinite(v) || v < 1) {
      editingGoal = false;
      return;
    }
    try {
      goal = await api.setGoal(v);
    } catch (e) {
      errorText = errText(e);
    }
    editingGoal = false;
  }

  async function addManual() {
    const v = parseInt(manualInput, 10);
    if (!Number.isFinite(v) || v < 1) return;
    try {
      await api.addManual(v);
      manualInput = '';
      await load();
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function change(s: SessionRecord, d: number) {
    const count = Math.max(0, s.count + d);
    sessions = sessions.map((x) => (x.id === s.id ? { ...x, count } : x));
    try {
      await api.updateSessionCount(s.id, count);
    } catch (e) {
      errorText = errText(e);
      await load();
    }
  }

  async function remove(s: SessionRecord) {
    try {
      await api.deleteSession(s.id);
      openId = null;
      await load();
    } catch (e) {
      errorText = errText(e);
    }
  }

  let shareMsg = '';
  async function shareStats() {
    shareMsg = await share(
      statsText({ title: heroTitle, total: st.total, sessions: st.sessions, seconds: st.seconds, streak: days }),
    );
  }
  async function shareSession(s: SessionRecord) {
    shareMsg = await share(sessionText(s));
  }

  onMount(load);
</script>

<div class="page">
  <h1>{$t('stats.title')}</h1>

  <div class="seg glass" role="tablist">
    {#each PERIODS as x}
      <button role="tab" aria-selected={period === x.id} class:on={period === x.id} on:click={() => (period = x.id)}>
        {$t(x.label)}
      </button>
    {/each}
  </div>

  <section class="hero glass">
    <div class="hero-top">
      <div class="hero-label">{heroTitle}</div>
      <button class="share-btn" on:click={shareStats} aria-label={$t('share.button')} title={$t('share.button')}>
        <svg viewBox="0 0 24 24"><path d="M12 4v11M8 8l4-4 4 4M6 13v5a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-5" /></svg>
      </button>
    </div>
    <div class="hero-num">{fmtNum(st.total, $locale)}</div>
    <div class="hero-sub">
      {$t('count.unit')}
      {#if delta !== null && p.prev}
        <span class="delta" class:up={delta >= 0} class:down={delta < 0}>
          {delta >= 0 ? '▲' : '▼'} {Math.abs(delta)}%
        </span>
        <span class="vs">{$t(p.prev)}</span>
      {/if}
    </div>
  </section>

  <section class="card glass">
    <BarChart buckets={st.buckets} unit={$t('count.unit')} />
  </section>

  <div class="tiles">
    <div class="tile glass">
      <div class="t-num">{fmtNum(st.sessions, $locale)}</div>
      <div class="t-label">{$t('stats.sessions')}</div>
    </div>
    <div class="tile glass">
      <div class="t-num">{fmtDuration(st.seconds, $t)}</div>
      <div class="t-label">{$t('stats.chanting')}</div>
    </div>
    <div class="tile glass">
      <div class="t-num">{st.perDay === null ? fmtNum(goal, $locale) : fmtNum(Math.round(st.perDay), $locale)}</div>
      <div class="t-label">{st.perDay === null ? $t('stats.goalToday') : $t('stats.perDay')}</div>
    </div>
    <div class="tile glass">
      <div class="t-num">{days}</div>
      <div class="t-label">{days === 1 ? $t('stats.streak1') : $t('stats.streakN')}</div>
    </div>
  </div>

  <section class="card glass goal">
    <div class="row">
      <div>
        <div class="c-title">{$t('stats.goalTitle')}</div>
        <div class="c-sub">
          {$t('stats.goalMet', { met: goalMonth.met, n: goalMonth.elapsed })}
          {#if best}· {$t('stats.record', { n: fmtNum(best.total, $locale) })}{/if}
        </div>
      </div>
      {#if editingGoal}
        <form class="inline" on:submit|preventDefault={saveGoal}>
          <input type="number" inputmode="numeric" min="1" bind:value={goalInput} aria-label={$t('stats.newGoal')} />
          <button class="small primary" type="submit">{$t('common.ok')}</button>
        </form>
      {:else}
        <button class="pill" on:click={() => { goalInput = String(goal); editingGoal = true; }}>{fmtNum(goal, $locale)} ✎</button>
      {/if}
    </div>
  </section>

  <section class="card glass">
    <div class="c-title">{$t('stats.sessionsTitle')}</div>
    <form class="inline add" on:submit|preventDefault={addManual}>
      <input type="number" inputmode="numeric" min="1" placeholder={$t('stats.addManual')} bind:value={manualInput} aria-label={$t('stats.addManualLabel')} />
      <button class="small primary" type="submit" disabled={!manualInput}>{$t('common.add')}</button>
    </form>

    {#if loaded && list.length === 0}
      <p class="empty">{$t('stats.empty')}</p>
    {/if}

    <ul>
      {#each visible as s (s.id)}
        <li class:open={openId === s.id}>
          <button class="item" on:click={() => (openId = openId === s.id ? null : s.id)}>
            <span class="when">
              {fmtWhen(s.started_at, new Date(), $locale, words)}
              {#if s.manual}<span class="badge">{$t('stats.manual')}</span>{/if}
            </span>
            <span class="dur">{s.manual ? '' : fmtDuration(s.duration_secs, $t)}</span>
            <span class="n">{fmtNum(s.count, $locale)}</span>
          </button>
          {#if openId === s.id}
            <div class="edit">
              <div class="stepper">
                <button on:click={() => change(s, -1)} aria-label={$t('common.less')}>−</button>
                <span>{s.count}</span>
                <button on:click={() => change(s, 1)} aria-label={$t('common.more')}>+</button>
              </div>
              {#if !s.manual && s.detected !== s.count}
                <span class="det">{$t('count.detected', { n: s.detected })}</span>
              {/if}
              <button class="link" on:click={() => shareSession(s)}>{$t('share.button')}</button>
              <button class="link danger" on:click={() => remove(s)}>{$t('common.delete')}</button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if list.length > visible.length}
      <button class="more" on:click={() => (showAll = true)}>{$t('stats.showAll', { n: list.length })}</button>
    {/if}
  </section>

  {#if shareMsg}<p class="note-ok">{shareMsg}</p>{/if}
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
    letter-spacing: -0.01em;
    text-shadow: var(--title-shadow);
  }

  .seg {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    border-radius: 12px;
    padding: 3px;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 8px 0;
    border-radius: 9px;
    font-size: 0.76rem;
    font-weight: 500;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .hero {
    padding: 14px 16px;
  }
  .hero-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .share-btn {
    background: none;
    border: none;
    padding: 4px;
    color: var(--muted);
    cursor: pointer;
    line-height: 0;
  }
  .share-btn svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .note-ok {
    color: var(--green);
    font-size: 0.8rem;
    text-align: center;
    margin: 0;
  }
  .hero-label {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .hero-num {
    font-size: 3.2rem;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.05;
    font-variant-numeric: tabular-nums;
  }
  .hero-sub {
    color: var(--muted);
    font-size: 0.85rem;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: baseline;
  }
  .delta {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .delta.up {
    color: var(--green);
  }
  .delta.down {
    color: var(--red);
  }
  .vs {
    color: var(--faint);
  }

  .card {
    padding: 16px;
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .tile {
    border-radius: 14px;
    padding: 12px 14px;
  }
  .t-num {
    font-size: 1.35rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .t-label {
    color: var(--muted);
    font-size: 0.78rem;
    margin-top: 2px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .c-title {
    font-weight: 600;
  }
  .c-sub {
    color: var(--muted);
    font-size: 0.8rem;
    margin-top: 3px;
    line-height: 1.35;
  }
  .pill {
    flex-shrink: 0;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--accent);
    border-radius: 999px;
    padding: 8px 14px;
    font-weight: 600;
    cursor: pointer;
  }

  .inline {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .inline.add {
    margin: 12px 0 4px;
  }
  input {
    flex: 1;
    min-width: 0;
    width: 7rem;
    background: var(--surface-2);
    border: 1px solid var(--line);
    border-radius: 10px;
    color: var(--text);
    padding: 10px 12px;
    font-size: 1rem;
  }
  input:focus {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .small {
    border: none;
    border-radius: 10px;
    padding: 10px 14px;
    cursor: pointer;
    font-weight: 600;
  }
  .primary {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .primary:disabled {
    opacity: 0.4;
  }

  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
  }
  li {
    border-top: 1px solid var(--line);
  }
  li:first-child {
    border-top: none;
  }
  .item {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto 4.5rem;
    gap: 10px;
    align-items: center;
    background: none;
    border: none;
    padding: 12px 2px;
    text-align: left;
    cursor: pointer;
  }
  .when {
    font-size: 0.9rem;
  }
  .badge {
    font-size: 0.68rem;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 1px 6px;
    margin-left: 6px;
  }
  .dur {
    color: var(--faint);
    font-size: 0.8rem;
  }
  .n {
    text-align: right;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .edit {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 2px 12px;
  }
  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 3px;
  }
  .stepper button {
    width: 36px;
    height: 36px;
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
    font-variant-numeric: tabular-nums;
  }
  .det {
    color: var(--faint);
    font-size: 0.8rem;
  }
  .link {
    margin-left: auto;
    background: none;
    border: none;
    color: var(--muted);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .link.danger {
    color: var(--red);
    margin-left: 4px;
  }
  .more {
    width: 100%;
    margin-top: 6px;
    padding: 10px;
    border: none;
    background: var(--surface-2);
    border-radius: 10px;
    color: var(--muted);
    cursor: pointer;
  }
  .empty {
    color: var(--faint);
    font-size: 0.85rem;
    margin: 12px 0 0;
  }
  .error {
    color: var(--red);
    font-size: 0.85rem;
  }
</style>
