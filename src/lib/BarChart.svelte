<script lang="ts">
  import type { Bucket } from './stats';
  import { fmtNum } from './api';
  import { locale, t } from './i18n';

  export let buckets: Bucket[] = [];
  export let unit = 'daimoku';

  let selected: number | null = null;

  $: max = Math.max(1, ...buckets.map((b) => b.value));
  $: nice = niceMax(max);
  $: shown = selected !== null && selected < buckets.length ? buckets[selected] : null;
  // reset the selection when the period changes
  $: buckets, (selected = null);

  function niceMax(v: number): number {
    const p = Math.pow(10, Math.floor(Math.log10(v)));
    for (const m of [1, 2, 2.5, 5, 10]) if (m * p >= v) return m * p;
    return v;
  }
</script>

<div class="chart">
  <div class="readout" aria-live="polite">
    {#if shown}
      <strong>{fmtNum(shown.value, $locale)}</strong> {unit} · {shown.title}
    {:else}
      <span class="muted">{$t('stats.tapBar')}</span>
    {/if}
  </div>

  <div class="plot">
    <div class="grid top"><span>{fmtNum(nice, $locale)}</span></div>
    <div class="grid mid"><span>{fmtNum(nice / 2, $locale)}</span></div>
    <div class="bars" style="--n:{buckets.length}">
      {#each buckets as b, i}
        <button
          class="col"
          class:sel={selected === i}
          on:click={() => (selected = selected === i ? null : i)}
          on:mouseenter={() => (selected = i)}
          aria-label="{b.title}: {b.value} {unit}"
        >
          <span
            class="bar"
            class:zero={b.value === 0}
            class:current={b.current}
            style="height:{b.value === 0 ? 0 : Math.max(2, (b.value / nice) * 100)}%"
          ></span>
        </button>
      {/each}
    </div>
  </div>
  <div class="labels" style="--n:{buckets.length}">
    {#each buckets as b}
      <span class:cur={b.current}>{b.label}</span>
    {/each}
  </div>
</div>

<style>
  .chart {
    width: 100%;
  }
  .readout {
    font-size: 0.85rem;
    min-height: 1.3rem;
    margin-bottom: 10px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--faint);
  }
  .plot {
    position: relative;
    height: 150px;
    border-bottom: 1px solid var(--line);
  }
  /* value labels live in a right-hand gutter, never behind a bar */
  .bars {
    right: 38px !important;
  }
  .labels {
    margin-right: 38px;
  }
  .grid {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px dashed rgba(139, 147, 167, 0.18);
    pointer-events: none;
  }
  .grid span {
    position: absolute;
    right: 0;
    top: -0.45rem;
    width: 34px;
    text-align: right;
    background: var(--surface-solid);
    border-radius: 3px;
    font-size: 0.68rem;
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .grid.top {
    top: 0;
  }
  .grid.mid {
    top: 50%;
  }
  .bars,
  .labels {
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    column-gap: 2px;
  }
  .bars {
    position: absolute;
    inset: 0;
  }
  .col {
    height: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
  }
  .bar {
    width: 100%;
    max-width: 26px;
    background: var(--mark);
    border-radius: 4px 4px 0 0;
    transition: height 0.35s ease, background 0.15s;
  }
  .bar.current {
    background: var(--mark-current);
  }
  .col.sel .bar {
    background: var(--text);
  }
  .col.sel {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 4px;
  }
  .labels {
    margin-top: 6px;
  }
  .labels span {
    text-align: center;
    font-size: 0.68rem;
    color: var(--faint);
    white-space: nowrap;
    overflow: visible;
  }
  .labels span.cur {
    color: var(--muted);
  }
</style>
