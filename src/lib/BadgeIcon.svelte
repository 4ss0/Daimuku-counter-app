<script lang="ts">
  import { badgeShort, streakSpan, STREAK_BADGES, TOTAL_BADGES, type Badge } from '$lib/badges';
  import { locale } from '$lib/i18n';

  export let badge: Badge;
  export let size = 64;

  // bronze, silver, gold, jade, sapphire, ruby, amethyst, pearl, lotus
  const TIERS = ['#c98b52', '#b9c2cc', '#e7b83e', '#4fbf96', '#4f7fd9', '#d9505f', '#9b6bd6', '#e9e1cf', '#f29bbd'];
  $: list = badge.kind === 'total' ? TOTAL_BADGES : STREAK_BADGES;
  $: tier = TIERS[Math.min(TIERS.length - 1, list.indexOf(badge.n) + (badge.kind === 'total' ? 0 : 1))];
  $: span = streakSpan(badge.n);
</script>

<svg class:locked={!badge.earned} width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" style="--tier:{tier}">
  <!-- ribbon -->
  <path class="ribbon" d="M22 38l-6 22 9-5 5 8 4-20zM42 38l6 22-9-5-5 8-4-20z" />
  <!-- medal -->
  <circle class="rim" cx="32" cy="27" r="22" />
  <circle class="face" cx="32" cy="27" r="17" />
  {#if badge.kind === 'total'}
    <!-- a lotus petal behind the number -->
    <path class="mark" d="M32 15c4 4 4 9 0 13-4-4-4-9 0-13z" />
    <text x="32" y="38" text-anchor="middle">{badgeShort(badge, $locale)}</text>
  {:else}
    <!-- a calendar page with the span -->
    <rect class="mark" x="22" y="15" width="20" height="5" rx="1.5" />
    <text x="32" y="38" text-anchor="middle">{span[1]}</text>
  {/if}
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
  }
  .ribbon {
    fill: var(--tier);
    opacity: 0.55;
  }
  .rim {
    fill: var(--tier);
  }
  .face {
    fill: color-mix(in srgb, var(--tier) 35%, var(--surface-solid));
    stroke: rgba(255, 255, 255, 0.35);
    stroke-width: 1;
  }
  .mark {
    fill: var(--tier);
    opacity: 0.6;
  }
  text {
    font-family: inherit;
    font-weight: 800;
    font-size: 13px;
    fill: var(--text);
  }
  .locked .ribbon,
  .locked .rim,
  .locked .mark {
    fill: var(--faint);
    opacity: 0.35;
  }
  .locked .face {
    fill: var(--surface-2);
  }
  .locked text {
    fill: var(--faint);
  }
</style>
