<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errText, fmtNum, type SessionRecord } from '$lib/api';
  import { locale, t } from '$lib/i18n';
  import { prefs } from '$lib/prefs';
  import { streak } from '$lib/stats';
  import BadgeIcon from '$lib/BadgeIcon.svelte';
  import { badgeName, badges, forgive, graceOffer, longestStreak, noteBadges } from '$lib/badges';

  let sessions: SessionRecord[] = [];
  let loaded = false;
  let errorText = '';

  $: days = streak(sessions, new Date(), $prefs.forgiven);
  $: offer = loaded ? graceOffer(sessions, $prefs.forgiven) : null;
  let forgivenNow = false;
  $: allBadges = badges(sessions, $prefs.forgiven);
  $: longest = longestStreak(sessions, $prefs.forgiven);
  $: allTotal = sessions.reduce((a, x) => a + x.count, 0);

  function dayWords(d: Date): string {
    const y = new Date();
    y.setDate(y.getDate() - 1);
    if (d.toDateString() === y.toDateString()) return $t('common.yesterday');
    const s = d.toLocaleDateString($locale, { weekday: 'long', day: 'numeric', month: 'long' });
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function useGrace() {
    if (!offer) return;
    forgive(offer.key);
    forgivenNow = true;
  }

  onMount(async () => {
    try {
      sessions = await api.listSessions();
      noteBadges(sessions);
    } catch (e) {
      errorText = errText(e);
    }
    loaded = true;
  });
</script>

<div class="page">
  <div class="tiles">
    <div class="tile glass">
      <div class="t-num">{fmtNum(allTotal, $locale)}</div>
      <div class="t-label">{$t('badge.allDaimoku')}</div>
    </div>
    <div class="tile glass">
      <div class="t-num">{days}</div>
      <div class="t-label">{days === 1 ? $t('stats.streak1') : $t('stats.streakN')}</div>
    </div>
  </div>

  {#if offer}
    <section class="card glass grace">
      <div class="c-title">{$t('grace.title')}</div>
      <p>{$t('grace.text', { day: dayWords(offer.day) })}</p>
      <button class="small primary" on:click={useGrace}>{$t('grace.button')}</button>
    </section>
  {:else if forgivenNow}
    <p class="note-ok">{$t('grace.done')}</p>
  {/if}

  <section class="card glass">
    <div class="c-sub">{$t('badge.have', { n: fmtNum(allTotal, $locale), d: fmtNum(longest, $locale) })}</div>
    {#each [['total', 'badge.totalGroup'], ['streak', 'badge.streakGroup']] as const as [kind, label]}
      <div class="b-group">{$t(label)}</div>
      <div class="b-grid">
        {#each allBadges.filter((b) => b.kind === kind) as b (b.id)}
          <div class="b-item" class:locked={!b.earned} title={badgeName(b, $t, $locale)}>
            <BadgeIcon badge={b} size={54} />
            <span class="b-name">{badgeName(b, $t, $locale)}</span>
            {#if !b.earned}
              <span class="b-bar"><span style="width:{Math.min(100, (b.have / b.n) * 100)}%"></span></span>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  </section>

  {#if errorText}<p class="error">{errorText}</p>{/if}
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
  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .tile {
    padding: 14px;
    text-align: center;
  }
  .t-num {
    font-size: 1.6rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .t-label {
    color: var(--muted);
    font-size: 0.8rem;
    margin-top: 2px;
  }
  .c-title {
    font-weight: 600;
  }
  .c-sub {
    color: var(--muted);
    font-size: 0.85rem;
    line-height: 1.4;
  }
  .grace p {
    margin: 6px 0 12px;
    line-height: 1.5;
    color: var(--muted);
  }
  .note-ok {
    color: var(--green);
    font-size: 0.8rem;
    text-align: center;
    margin: 0;
  }
  .b-group {
    margin: 14px 0 8px;
    color: var(--muted);
    font-size: 0.82rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .b-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(86px, 1fr));
    gap: 12px 8px;
  }
  .b-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }
  .b-name {
    font-size: 0.78rem;
    line-height: 1.25;
  }
  .b-item.locked .b-name {
    color: var(--faint);
  }
  .b-bar {
    width: 70%;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .b-bar span {
    display: block;
    height: 100%;
    background: var(--accent);
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
  .error {
    color: var(--red);
    font-size: 0.85rem;
    text-align: center;
  }
</style>
