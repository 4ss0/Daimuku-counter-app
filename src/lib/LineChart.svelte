<script lang="ts">
  // The Stats chart: one dot per day (week, month...) joined by a line.
  // Touching a column shows its value.
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
  /** Centre of column i and height of a value, in % of the plot. */
  $: xs = buckets.map((_, i) => ((i + 0.5) / Math.max(1, buckets.length)) * 100);
  $: ys = buckets.map((b) => 100 - (b.value / nice) * 100);
  $: points = buckets.map((_, i) => `${xs[i]},${ys[i]}`).join(' ');
  $: area = buckets.length ? `${xs[0]},100 ${points} ${xs[xs.length - 1]},100` : '';

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
    <div class="area">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        {#if buckets.length > 1}
          <polygon class="fill" points={area} />
          <polyline class="line" points={points} />
        {/if}
      </svg>
      {#each buckets as b, i}
        <span
          class="dot"
          class:zero={b.value === 0}
          class:current={b.current}
          class:sel={selected === i}
          style="left:{xs[i]}%;top:{ys[i]}%"
        ></span>
      {/each}
      <div class="cols" style="--n:{buckets.length}">
        {#each buckets as b, i}
          <button
            class="col"
            class:sel={selected === i}
            on:click={() => (selected = selected === i ? null : i)}
            on:mouseenter={() => (selected = i)}
            aria-label="{b.title}: {b.value} {unit}"
          ></button>
        {/each}
      </div>
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
  /* value labels live in a right-hand gutter, never behind the line */
  .area {
    position: absolute;
    inset: 0 38px 0 0;
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .line {
    fill: none;
    stroke: var(--mark);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }
  .fill {
    fill: var(--accent-soft);
    stroke: none;
  }
  .dot {
    position: absolute;
    width: 9px;
    height: 9px;
    margin: -4.5px 0 0 -4.5px;
    border-radius: 50%;
    background: var(--mark);
    border: 2px solid var(--surface-solid);
    box-sizing: content-box;
    pointer-events: none;
    transition: top 0.35s ease, transform 0.15s, background 0.15s;
  }
  .dot.zero {
    background: var(--surface-solid);
    border-color: var(--mark);
    width: 5px;
    height: 5px;
    margin: -4.5px 0 0 -4.5px;
  }
  .dot.current {
    background: var(--mark-current);
  }
  .dot.sel {
    background: var(--text);
    transform: scale(1.35);
  }
  .cols {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
  }
  .col {
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
  }
  .col.sel {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 4px;
  }
  .labels {
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    margin: 6px 38px 0 0;
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
