<script lang="ts">
  import { onMount } from 'svelte';
  import BarChart from '$lib/BarChart.svelte';
  import { api, errText, fmtDuration, fmtNum, type SessionRecord } from '$lib/api';
  import { bestDay, fmtWhen, goalDaysThisMonth, periodStats, sessionsIn, streak, type Period } from '$lib/stats';

  const PERIODS: { id: Period; label: string; prev: string }[] = [
    { id: 'day', label: 'Giorno', prev: 'rispetto a ieri alla stessa ora' },
    { id: 'week', label: 'Settimana', prev: 'rispetto alla settimana scorsa' },
    { id: 'month', label: 'Mese', prev: 'rispetto al mese scorso' },
    { id: 'year', label: 'Anno', prev: "rispetto all'anno scorso" },
    { id: 'all', label: 'Totale', prev: '' },
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

  $: st = periodStats(sessions, period);
  $: p = PERIODS.find((x) => x.id === period)!;
  $: delta = st.prevTotal !== null && st.prevTotal > 0 ? Math.round(((st.total - st.prevTotal) / st.prevTotal) * 100) : null;
  $: days = streak(sessions);
  $: best = bestDay(sessions);
  $: goalMonth = goalDaysThisMonth(sessions, goal);
  $: list = sessionsIn(sessions, period);
  $: visible = showAll ? list : list.slice(0, 20);
  $: period, (showAll = false);

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
    // optimistic update, then persist
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

  onMount(load);
</script>

<div class="page">
  <h1>Statistiche</h1>

  <div class="seg" role="tablist">
    {#each PERIODS as x}
      <button role="tab" aria-selected={period === x.id} class:on={period === x.id} on:click={() => (period = x.id)}>
        {x.label}
      </button>
    {/each}
  </div>

  <section class="hero">
    <div class="hero-label">{st.title}</div>
    <div class="hero-num">{fmtNum(st.total)}</div>
    <div class="hero-sub">
      daimoku
      {#if delta !== null}
        <span class="delta" class:up={delta >= 0} class:down={delta < 0}>
          {delta >= 0 ? '▲' : '▼'} {Math.abs(delta)}%
        </span>
        <span class="vs">{p.prev}</span>
      {/if}
    </div>
  </section>

  <section class="card">
    <BarChart buckets={st.buckets} />
  </section>

  <div class="tiles">
    <div class="tile">
      <div class="t-num">{fmtNum(st.sessions)}</div>
      <div class="t-label">sessioni</div>
    </div>
    <div class="tile">
      <div class="t-num">{fmtDuration(st.seconds)}</div>
      <div class="t-label">di recitazione</div>
    </div>
    <div class="tile">
      <div class="t-num">{st.perDay === null ? fmtNum(goal) : fmtNum(Math.round(st.perDay))}</div>
      <div class="t-label">{st.perDay === null ? 'obiettivo del giorno' : 'media al giorno'}</div>
    </div>
    <div class="tile">
      <div class="t-num">{days}</div>
      <div class="t-label">{days === 1 ? 'giorno di fila' : 'giorni di fila'}</div>
    </div>
  </div>

  <section class="card goal">
    <div class="row">
      <div>
        <div class="c-title">Obiettivo giornaliero</div>
        <div class="c-sub">
          Raggiunto {goalMonth.met} {goalMonth.met === 1 ? 'giorno' : 'giorni'} su {goalMonth.elapsed} questo mese
          {#if best}· record {fmtNum(best.total)} in un giorno{/if}
        </div>
      </div>
      {#if editingGoal}
        <form class="inline" on:submit|preventDefault={saveGoal}>
          <input type="number" inputmode="numeric" min="1" bind:value={goalInput} aria-label="Nuovo obiettivo" />
          <button class="small primary" type="submit">OK</button>
        </form>
      {:else}
        <button class="pill" on:click={() => { goalInput = String(goal); editingGoal = true; }}>{fmtNum(goal)} ✎</button>
      {/if}
    </div>
  </section>

  <section class="card">
    <div class="c-title">Sessioni</div>
    <form class="inline add" on:submit|preventDefault={addManual}>
      <input type="number" inputmode="numeric" min="1" placeholder="Aggiungi a mano…" bind:value={manualInput} aria-label="Daimoku da aggiungere" />
      <button class="small primary" type="submit" disabled={!manualInput}>Aggiungi</button>
    </form>

    {#if loaded && list.length === 0}
      <p class="empty">Nessuna sessione in questo periodo.</p>
    {/if}

    <ul>
      {#each visible as s (s.id)}
        <li class:open={openId === s.id}>
          <button class="item" on:click={() => (openId = openId === s.id ? null : s.id)}>
            <span class="when">
              {fmtWhen(s.started_at)}
              {#if s.manual}<span class="badge">a mano</span>{/if}
            </span>
            <span class="dur">{s.manual ? '' : fmtDuration(s.duration_secs)}</span>
            <span class="n">{fmtNum(s.count)}</span>
          </button>
          {#if openId === s.id}
            <div class="edit">
              <div class="stepper">
                <button on:click={() => change(s, -1)} aria-label="Uno in meno">−</button>
                <span>{s.count}</span>
                <button on:click={() => change(s, 1)} aria-label="Uno in più">+</button>
              </div>
              {#if !s.manual && s.detected !== s.count}
                <span class="det">rilevati {s.detected}</span>
              {/if}
              <button class="link danger" on:click={() => remove(s)}>Elimina</button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if list.length > visible.length}
      <button class="more" on:click={() => (showAll = true)}>Mostra tutte ({list.length})</button>
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
    letter-spacing: -0.01em;
  }

  .seg {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 3px;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 8px 0;
    border-radius: 9px;
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-2);
    color: var(--text);
  }

  .hero {
    padding: 6px 2px 0;
  }
  .hero-label {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .hero-num {
    font-size: 3.4rem;
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
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 16px;
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .tile {
    background: var(--surface);
    border: 1px solid var(--line);
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
    color: var(--gold);
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
    outline: 2px solid var(--gold);
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
    background: var(--gold);
    color: #1b1406;
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
