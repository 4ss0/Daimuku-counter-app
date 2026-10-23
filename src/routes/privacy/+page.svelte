<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { errText } from '$lib/api';
  import { t, type Key } from '$lib/i18n';

  const PRIVACY_URL = 'https://4ss0.github.io/Daimuku-counter-app/privacy-policy.html';

  /** Every permission in the Android manifest, and why. */
  const PERMS: { name: Key; why: Key }[] = [
    { name: 'priv.micName', why: 'priv.mic' },
    { name: 'priv.notifName', why: 'priv.notif' },
    { name: 'priv.fgsName', why: 'priv.fgs' },
    { name: 'priv.wakeName', why: 'priv.wake' },
    { name: 'priv.bootName', why: 'priv.boot' },
    { name: 'priv.billingName', why: 'priv.billing' },
  ];

  let errorText = '';

  async function openPolicy() {
    try {
      await openUrl(PRIVACY_URL);
    } catch (e) {
      errorText = errText(e);
    }
  }
</script>

<div class="page">
  <section class="card glass hero">
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 3l7 3v5.2c0 4.4-3 8.1-7 9.8-4-1.7-7-5.4-7-9.8V6z" />
      <path d="M9 12l2.2 2.2L15.5 10" />
    </svg>
    <h2>{$t('priv.shortTitle')}</h2>
    <p>{$t('priv.short')}</p>
  </section>

  <section class="card glass">
    <h2>{$t('priv.permsTitle')}</h2>
    <p class="muted">{$t('priv.permsLead')}</p>
    <dl>
      {#each PERMS as perm}
        <dt>{$t(perm.name)}</dt>
        <dd>{$t(perm.why)}</dd>
      {/each}
    </dl>
    <p class="muted">{$t('priv.notAsked')}</p>
  </section>

  <section class="card glass">
    <h2>{$t('priv.dataTitle')}</h2>
    <p>{$t('priv.data')}</p>
  </section>

  <section class="card glass">
    <h2>{$t('priv.donateTitle')}</h2>
    <p>{$t('priv.donate')}</p>
  </section>

  <button class="link" on:click={openPolicy}>{$t('set.privacyLink')}</button>
  {#if errorText}<p class="error">{errorText}</p>{/if}
</div>

<style>
  .page {
    width: 100%;
    max-width: 560px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .card {
    padding: 16px 18px;
  }
  .hero {
    text-align: center;
  }
  .hero svg {
    width: 44px;
    height: 44px;
    fill: var(--accent-soft);
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  h2 {
    margin: 4px 0 8px;
    font-size: 1.05rem;
  }
  p {
    margin: 0;
    line-height: 1.55;
  }
  .muted {
    color: var(--muted);
  }
  dl {
    margin: 12px 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  dt {
    font-weight: 600;
  }
  dd {
    margin: 2px 0 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .link {
    align-self: center;
    padding: 11px 18px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--accent);
    font-weight: 600;
    cursor: pointer;
  }
  .error {
    color: var(--red);
    text-align: center;
  }
</style>
