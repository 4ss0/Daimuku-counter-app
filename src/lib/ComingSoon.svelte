<script lang="ts">
  import { t } from '$lib/i18n';

  /** Text under the title; the generic "being prepared" one if missing. */
  export let text = '';
</script>

<div class="page">
  <section class="card glass">
    <svg viewBox="0 0 64 64" aria-hidden="true">
      <circle cx="32" cy="32" r="22" />
      <path d="M32 20v13l8 5" />
    </svg>
    <h2>{$t('soon.title')}</h2>
    <p>{text || $t('soon.text')}</p>
  </section>
</div>

<style>
  .page {
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: 18px 16px 28px;
  }
  .card {
    padding: 28px 22px;
    text-align: center;
  }
  svg {
    width: 56px;
    height: 56px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 2.5;
    stroke-linecap: round;
  }
  h2 {
    margin: 10px 0 6px;
    font-size: 1.2rem;
  }
  p {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
</style>
