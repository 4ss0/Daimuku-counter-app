<script lang="ts">
  import { onMount } from 'svelte';
  import { BELLS, prepareBell, strike, type BellKind } from '$lib/bell';
  import { t, type Key } from '$lib/i18n';
  import { prefs, updatePrefs } from '$lib/prefs';

  function set(patch: Partial<{ on: boolean; kind: BellKind; volume: number }>) {
    updatePrefs({ bell: { ...$prefs.bell, ...patch } });
  }

  /** Choosing a bell also rings it. */
  function choose(kind: BellKind) {
    set({ kind });
    strike(kind, $prefs.bell.volume);
  }

  onMount(() => BELLS.forEach(prepareBell));
</script>

<div class="page">
  <p class="lead glass">{$t('bell.lead')}</p>

  <section class="card glass bells">
    {#each BELLS as kind, i}
      <button class="bell" class:on={$prefs.bell.kind === kind} on:pointerdown={() => choose(kind)} aria-pressed={$prefs.bell.kind === kind}>
        <svg viewBox="0 0 48 48" aria-hidden="true" style="--s:{0.64 + i * 0.18}">
          <g class="cup">
            <path d="M8 22h32c0 9-7 16-16 16S8 31 8 22z" />
            <ellipse cx="24" cy="22" rx="16" ry="3.5" />
          </g>
          <rect x="14" y="38" width="20" height="4" rx="2" class="cushion" />
        </svg>
        <span class="text">
          <strong>{$t(`bell.${kind}` as Key)}</strong>
          <span>{$t(`bell.${kind}Text` as Key)}</span>
        </span>
        {#if $prefs.bell.kind === kind}<span class="check" aria-hidden="true">✓</span>{/if}
      </button>
    {/each}
  </section>

  <section class="card glass">
    <label class="row">
      <span>{$t('bell.volume')}</span>
      <input type="range" min="0" max="1" step="0.05" value={$prefs.bell.volume} on:change={(e) => set({ volume: parseFloat(e.currentTarget.value) })} />
    </label>
  </section>

  <section class="card glass">
    <label class="row">
      <span class="col">
        <strong>{$t('bell.show')}</strong>
        <span class="hint">{$t('bell.showHint')}</span>
      </span>
      <input type="checkbox" role="switch" class="switch" checked={$prefs.bell.on} on:change={(e) => set({ on: e.currentTarget.checked })} />
    </label>
  </section>
</div>

<style>
  .page {
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .lead {
    margin: 0;
    padding: 14px 16px;
    line-height: 1.5;
  }
  .card {
    padding: 14px 16px;
  }
  .bells {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
  }
  .bell {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    text-align: left;
    cursor: pointer;
    touch-action: manipulation;
  }
  .bell.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .bell svg {
    width: 52px;
    height: 52px;
    flex-shrink: 0;
  }
  .cup {
    fill: #d8b25a;
    stroke: #a07a2c;
    stroke-width: 1.2;
    transform: scale(var(--s));
    transform-origin: 24px 40px;
  }
  .cushion {
    fill: var(--muted);
    opacity: 0.6;
  }
  .bell:active .cup {
    animation: ring 0.5s ease-out;
  }
  @keyframes ring {
    20% {
      transform: scale(var(--s)) rotate(-6deg);
    }
    50% {
      transform: scale(var(--s)) rotate(4deg);
    }
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .text span {
    color: var(--muted);
    font-size: 0.86rem;
  }
  .check {
    color: var(--accent);
    font-weight: 700;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .hint {
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.4;
  }
  input[type='range'] {
    flex: 1;
    max-width: 240px;
    accent-color: var(--accent);
  }
</style>
