<script lang="ts">
  import { onMount } from 'svelte';
  import { t, type Key } from '$lib/i18n';
  import { canDonate, donate, donationProducts, type DonationProduct, type DonationResult } from '$lib/native';

  /** One-time products to create in the Play Console (consumable, any price). */
  const TIERS: { id: string; key: Key }[] = [
    { id: 'donation_small', key: 'donate.small' },
    { id: 'donation_medium', key: 'donate.medium' },
    { id: 'donation_large', key: 'donate.large' },
  ];

  const native = canDonate();
  let state: 'loading' | 'ready' | 'unavailable' = native ? 'loading' : 'unavailable';
  let prices: Record<string, string> = {};
  let busy = '';
  let result: DonationResult | null = null;

  async function load() {
    state = 'loading';
    const list: DonationProduct[] | null = await donationProducts(TIERS.map((x) => x.id));
    prices = Object.fromEntries((list ?? []).map((p) => [p.id, p.price]));
    state = list && list.length > 0 ? 'ready' : 'unavailable';
  }

  async function give(id: string) {
    if (busy) return;
    busy = id;
    result = null;
    try {
      result = await donate(id);
    } finally {
      busy = '';
    }
  }

  onMount(() => {
    if (native) load();
  });
</script>

<div class="page">
  <section class="card glass hero">
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 20s-7.5-4.6-7.5-10.2A4.1 4.1 0 0 1 12 7.3a4.1 4.1 0 0 1 7.5 2.5C19.5 15.4 12 20 12 20z" />
    </svg>
    <p>{$t('donate.intro')}</p>
  </section>

  {#if result === 'thanks' || result === 'pending'}
    <div class="msg ok" role="status">{$t(result === 'thanks' ? 'donate.thanks' : 'donate.pending')}</div>
  {:else if result === 'error' || result === 'unavailable'}
    <div class="msg err" role="alert">{$t('donate.error')}</div>
  {/if}

  <section class="card glass">
    {#if state === 'loading'}
      <p class="muted">{$t('donate.loading')}</p>
    {:else if state === 'unavailable'}
      <p class="muted">{$t(native ? 'donate.unavailable' : 'donate.noApp')}</p>
      {#if native}
        <button class="btn ghost" on:click={load}>{$t('donate.retry')}</button>
      {/if}
    {:else}
      <div class="tiers">
        {#each TIERS.filter((x) => prices[x.id]) as tier}
          <button class="tier" on:click={() => give(tier.id)} disabled={!!busy}>
            <span class="name">{$t(tier.key)}</span>
            <span class="price">{busy === tier.id ? '…' : prices[tier.id]}</span>
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <p class="note">{$t('donate.free')}</p>
  <p class="note">{$t('donate.how')}</p>
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
  .card {
    padding: 16px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 22px 18px;
  }
  .hero svg {
    width: 48px;
    height: 48px;
    fill: var(--accent-soft);
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linejoin: round;
  }
  .hero p {
    margin: 0;
    line-height: 1.55;
  }
  .tiers {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .tier {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 14px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    font-weight: 500;
    cursor: pointer;
  }
  .tier:active {
    background: var(--accent-soft);
  }
  .tier:disabled {
    opacity: 0.6;
  }
  .price {
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .muted {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .btn {
    margin-top: 12px;
    padding: 9px 16px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    cursor: pointer;
  }
  .msg {
    padding: 12px 16px;
    border-radius: 14px;
    line-height: 1.45;
  }
  .msg.ok {
    background: var(--accent-soft);
    color: var(--text);
    border: 1px solid var(--accent);
  }
  .msg.err {
    background: rgba(229, 103, 95, 0.14);
    border: 1px solid var(--red);
  }
  .note {
    margin: 0 4px;
    color: var(--muted);
    font-size: 0.86rem;
    line-height: 1.5;
    text-shadow: var(--title-shadow);
  }
</style>
