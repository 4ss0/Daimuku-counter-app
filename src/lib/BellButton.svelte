<script lang="ts">
  import { prefs } from './prefs';
  import { t } from './i18n';
  import { prepareBell, strike } from './bell';
  $: prepareBell($prefs.bell.kind);
  function ring() { void strike($prefs.bell.kind, $prefs.bell.volume).catch(() => {}); }
</script>

<button class="glass" aria-label={$t('bell.strike')} title={$t('bell.strike')}
  on:pointerdown|preventDefault={ring} on:click={(event) => { if (event.detail === 0) ring(); }}>
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <path d="M5 16.5h14l-1.6-2.4V10a5.4 5.4 0 0 0-10.8 0v4.1zM10 19.2a2.2 2.2 0 0 0 4 0M12 4.6V3" />
  </svg>
</button>

<style>
  button { width: min(100%, var(--bell-size, 58px)); height: auto; aspect-ratio: 1; padding: 0; border-radius: 50%; display: grid; place-items: center; color: var(--accent); cursor: pointer; touch-action: manipulation; }
  button:active { transform: scale(0.94); }
  svg { width: 28px; height: 28px; fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }
</style>
