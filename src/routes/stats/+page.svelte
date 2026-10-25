<script lang="ts">
  import { onMount } from 'svelte';
  import LineChart from '$lib/LineChart.svelte';
  import { api, errText, fmtDuration, fmtNum, type SessionRecord } from '$lib/api';
  import { locale, t, type Key } from '$lib/i18n';
  import { share, sessionText } from '$lib/share';
  import { shareCard } from '$lib/card';
  import { bestDay, fmtWhen, goalDaysThisMonth, halfOf, periodStats, periodTotal, sessionsIn, splitGoal, streak, todayHalves, type GoalPeriod, type Half, type Period } from '$lib/stats';
  import { prefs, prefsLoaded } from '$lib/prefs';
  import { noteBadges } from '$lib/badges';

  const PERIODS: { id: Period; label: Key; prev: Key | null; title: Key | null }[] = [
    { id: 'day', label: 'stats.day', prev: 'stats.prevDay', title: 'stats.titleDay' },
    { id: 'week', label: 'stats.week', prev: 'stats.prevWeek', title: 'stats.titleWeek' },
    { id: 'month', label: 'stats.month', prev: 'stats.prevMonth', title: null },
    { id: 'year', label: 'stats.year', prev: 'stats.prevYear', title: null },
    { id: 'all', label: 'stats.all', prev: null, title: 'stats.titleAll' },
  ];

  let sessions: SessionRecord[] = [];
  let goal = 100;
  // the period the page opens on is chosen in Settings
  let period: Period = 'week';
  let periodSet = false;
  $: if ($prefsLoaded && !periodSet) {
    period = $prefs.statsPeriod;
    periodSet = true;
  }
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
  $: days = streak(sessions, new Date(), $prefs.forgiven);
  $: best = bestDay(sessions);
  $: goalMonth = goalDaysThisMonth(sessions, goal);
  $: halfGoal = splitGoal(goal);
  $: halves = todayHalves(sessions);
  const PERIOD_GOALS: { id: GoalPeriod; label: Key }[] = [
    { id: 'week', label: 'goal.thisWeek' },
    { id: 'month', label: 'goal.thisMonth' },
    { id: 'year', label: 'goal.thisYear' },
  ];
  $: periodGoals = PERIOD_GOALS.filter((g) => $prefs.goals[g.id] > 0).map((g) => ({
    ...g,
    goal: $prefs.goals[g.id],
    done: periodTotal(sessions, g.id),
  }));
  $: list = sessionsIn(sessions, period);
  $: visible = showAll ? list : list.slice(0, 20);
  $: period, (showAll = false);
  $: words = { today: $t('common.today'), yesterday: $t('common.yesterday') };

  async function load() {
    try {
      [sessions, goal] = await Promise.all([api.listSessions(), api.getGoal()]);
      noteBadges(sessions);
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

  // a session added by hand: the day, its session (morning or evening)
  // and the Daimoku chanted. Today's goes in now; an earlier day's at 8:00
  // or 20:00, so that the list shows it in its place.
  const pad = (n: number) => String(n).padStart(2, '0');
  const dayInput = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  let addDay = dayInput(new Date());
  let addHalf: Half = halfOf(new Date());
  let pastMsg = '';
  const HALF_HOUR: Record<Half, number> = { morning: 8, evening: 20 };

  async function addManual() {
    const v = parseInt(manualInput, 10);
    if (!Number.isFinite(v) || v < 1) return;
    errorText = '';
    pastMsg = '';
    const today = dayInput(new Date());
    let at: string | undefined;
    if (addDay && addDay !== today) {
      if (addDay > today) {
        errorText = $t('past.future');
        return;
      }
      const when = new Date(`${addDay}T${pad(HALF_HOUR[addHalf])}:00`);
      if (isNaN(when.getTime())) return;
      at = when.toISOString();
    }
    try {
      await api.addManual(v, at, at ? 0 : undefined, addHalf);
      manualInput = '';
      pastMsg = $t('past.added');
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
  let sharing = false;
  async function withCard(fn: () => Promise<string>) {
    if (sharing) return;
    sharing = true;
    shareMsg = '';
    try {
      shareMsg = await fn();
    } catch (e) {
      errorText = errText(e);
    } finally {
      sharing = false;
    }
  }

  // the period shown, with its bar chart
  function shareStats() {
    const lines: string[] = [];
    if (st.sessions > 0) lines.push($t('share.statsSessions', { n: fmtNum(st.sessions, $locale), d: fmtDuration(st.seconds, $t) }));
    if (st.perDay !== null && period !== 'day') lines.push($t('share.perDay', { n: fmtNum(Math.round(st.perDay), $locale) }));
    if (days > 1) lines.push($t('share.statsStreak', { n: days }));
    return withCard(() =>
      shareCard(
        { kicker: heroTitle, big: fmtNum(st.total, $locale), unit: $t('count.unit'), lines, bars: st.buckets },
        `daimoku-${period}`,
      ),
    );
  }

  // today's progress towards the daily goal
  function shareGoal() {
    const today = periodStats(sessions, 'day', new Date(), $locale).total;
    const lines = [$t('share.goalLine', { n: fmtNum(goal, $locale) }), $t('stats.goalMet', { met: goalMonth.met, n: goalMonth.elapsed })];
    if (days > 1) lines.push($t('share.statsStreak', { n: days }));
    return withCard(() =>
      shareCard(
        {
          kicker: today >= goal ? $t('count.goalTitle') : $t('stats.titleDay'),
          big: fmtNum(today, $locale),
          unit: $t('count.unit'),
          lines,
          progress: today / Math.max(1, goal),
        },
        'daimoku-goal',
      ),
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
      <button class="share-btn" on:click={shareStats} disabled={sharing} aria-label={$t('share.button')} title={$t('share.button')}>
        <svg class="share-ico" viewBox="0 0 24 24" aria-hidden="true"><circle cx="18" cy="5" r="2.6" /><circle cx="6" cy="12" r="2.6" /><circle cx="18" cy="19" r="2.6" /><path d="M8.3 10.8l7.4-4.3M8.3 13.2l7.4 4.3" /></svg>
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
    <LineChart buckets={st.buckets} unit={$t('count.unit')} />
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
      <div class="t-label">{$t(days === 1 ? 'stats.streak1' : 'stats.streakN', { n: days })}</div>
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
        <span class="goal-actions">
          <button class="share-btn" on:click={shareGoal} disabled={sharing} aria-label={$t('share.button')} title={$t('share.button')}>
            <svg class="share-ico" viewBox="0 0 24 24" aria-hidden="true"><circle cx="18" cy="5" r="2.6" /><circle cx="6" cy="12" r="2.6" /><circle cx="18" cy="19" r="2.6" /><path d="M8.3 10.8l7.4-4.3M8.3 13.2l7.4 4.3" /></svg>
          </button>
          <button class="pill" on:click={() => { goalInput = String(goal); editingGoal = true; }}>{fmtNum(goal, $locale)} ✎</button>
        </span>
      {/if}
    </div>
    {#if goal >= 2}
      <div class="halves">
        <span>{$t('goal.morning')} <strong>{fmtNum(halves.morning, $locale)}</strong> / {fmtNum(halfGoal.morning, $locale)}{halves.morning >= halfGoal.morning ? ' ✓' : ''}</span>
        <span>{$t('goal.evening')} <strong>{fmtNum(halves.evening, $locale)}</strong> / {fmtNum(halfGoal.evening, $locale)}{halves.evening >= halfGoal.evening ? ' ✓' : ''}</span>
      </div>
    {/if}
    {#each periodGoals as g}
      <div class="pgoal">
        <div class="pg-top">
          <span>{$t(g.label)}</span>
          <span><strong>{fmtNum(g.done, $locale)}</strong> / {fmtNum(g.goal, $locale)}{g.done >= g.goal ? ' ✓' : ''}</span>
        </div>
        <div class="bar"><div class="fill" class:full={g.done >= g.goal} style="width:{Math.min(100, (g.done / g.goal) * 100)}%"></div></div>
      </div>
    {/each}
  </section>

  <section class="card glass">
    <div class="c-title">{$t('stats.sessionsTitle')}</div>
    <form class="add-form" on:submit|preventDefault={addManual}>
      <div class="past">
        <label>
          <span>{$t('past.day')}</span>
          <input type="date" bind:value={addDay} max={dayInput(new Date())} />
        </label>
        <div class="past-half" role="group" aria-label={$t('past.session')}>
          <span>{$t('past.session')}</span>
          <div class="seg glass">
            {#each ['morning', 'evening'] as const as h}
              <button type="button" class:on={addHalf === h} aria-pressed={addHalf === h} on:click={() => (addHalf = h)}>{$t(`goal.${h}`)}</button>
            {/each}
          </div>
        </div>
      </div>
      <div class="inline add">
        <input type="number" inputmode="numeric" min="1" placeholder={$t('stats.addManual')} bind:value={manualInput} aria-label={$t('stats.addManualLabel')} />
        <button class="small primary" type="submit" disabled={!manualInput}>{$t('common.add')}</button>
      </div>
    </form>
    {#if pastMsg}<p class="note-ok">{pastMsg}</p>{/if}

    {#if loaded && list.length === 0}
      <p class="empty">{$t('stats.empty')}</p>
    {/if}

    <ul>
      {#each visible as s (s.id)}
        <li class:open={openId === s.id}>
          <button class="item" on:click={() => (openId = openId === s.id ? null : s.id)}>
            <span class="when">
              {#if (s.half ?? halfOf(new Date(s.started_at))) === 'morning'}
                <svg class="h-ico" viewBox="0 0 24 24" role="img" aria-label={$t('goal.morning')}><circle cx="12" cy="12" r="4" /><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4" /></svg>
              {:else}
                <svg class="h-ico" viewBox="0 0 24 24" role="img" aria-label={$t('goal.evening')}><path d="M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5z" /></svg>
              {/if}
              {fmtWhen(s.started_at, new Date(), $locale, words)}
              {#if s.manual}<span class="badge">{$t('stats.manual')}</span>{/if}
            </span>
            <span class="dur">{s.manual && !(s.duration_secs > 0) ? '' : fmtDuration(s.duration_secs, $t)}</span>
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
              <button class="link share-link" on:click={() => shareSession(s)}><svg class="share-ico" viewBox="0 0 24 24" aria-hidden="true"><circle cx="18" cy="5" r="2.6" /><circle cx="6" cy="12" r="2.6" /><circle cx="18" cy="19" r="2.6" /><path d="M8.3 10.8l7.4-4.3M8.3 13.2l7.4 4.3" /></svg>{$t('share.button')}</button>
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
  .share-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  :global(.share-ico) {
    width: 1.15em;
    height: 1.15em;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    flex: none;
  }
  .goal-actions {
    display: flex;
    align-items: center;
    gap: 6px;
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
  .h-ico {
    width: 14px;
    height: 14px;
    margin-right: 4px;
    vertical-align: -2px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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
  .halves {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-top: 10px;
    color: var(--muted);
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
  }
  .halves strong,
  .pg-top strong {
    color: var(--text);
  }
  .pgoal {
    margin-top: 12px;
  }
  .pg-top {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
    margin-bottom: 5px;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
  }
  .fill.full {
    background: var(--green);
  }
  .past {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin-top: 10px;
  }
  .past label,
  .past-half {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    color: var(--muted);
    font-size: 0.78rem;
  }
  .past input {
    width: 100%;
    min-width: 0;
  }
  .past-half .seg {
    flex: 1;
    grid-template-columns: repeat(2, 1fr);
    align-items: stretch;
  }
  .past-half .seg button {
    font-size: 0.86rem;
  }
</style>
