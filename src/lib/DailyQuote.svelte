<script lang="ts">
  // The page shown when the app opens: the quote of the day (src/lib/quotes)
  // in the app's language. The arrows (or a swipe, or the arrow keys) go
  // through the other days' quotes; "Enter" goes into the app.
  import { fade } from 'svelte/transition';
  import { lang, locale, t, texts } from '$lib/i18n';
  import { DAYS, dayDate, dayIndex, quoteFor } from '$lib/quotes';

  export let onenter: () => void;

  const today = dayIndex(new Date());
  let offset = 0;
  $: day = (((today + offset) % DAYS) + DAYS) % DAYS;
  $: quote = quoteFor(day, $texts);
  $: dateLabel = dayDate(day).toLocaleDateString($locale, { day: 'numeric', month: 'long' });

  function move(by: number) {
    offset += by;
  }

  // a horizontal swipe browses, like the arrows
  const SWIPE_MIN = 50;
  let sx = 0;
  let sy = 0;
  function onTouchStart(e: TouchEvent) {
    sx = e.touches[0].clientX;
    sy = e.touches[0].clientY;
  }
  function onTouchEnd(e: TouchEvent) {
    const dx = e.changedTouches[0].clientX - sx;
    const dy = e.changedTouches[0].clientY - sy;
    if (Math.abs(dx) >= SWIPE_MIN && Math.abs(dx) > Math.abs(dy) * 1.5) move(dx < 0 ? 1 : -1);
  }

  // the page is in front of everything: the keys are for it alone (the
  // Count page starts counting on Space)
  function onKey(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'ArrowLeft') move(-1);
    else if (e.key === 'ArrowRight') move(1);
    else if (e.key === 'Enter' || e.key === 'Escape') {
      e.preventDefault();
      onenter();
    }
  }
</script>

<svelte:window on:keydown|capture={onKey} />

<div class="quote-page" role="dialog" aria-modal="true" aria-labelledby="quote-title" transition:fade={{ duration: 250 }}>
  <div class="top">
    <h1 id="quote-title">{offset === 0 ? $t('quote.title') : dateLabel}</h1>
    {#if offset === 0}
      <span class="date">{dateLabel}</span>
    {:else}
      <button class="today" on:click={() => (offset = 0)}>{$t('quote.today')}</button>
    {/if}
  </div>

  <div class="middle" role="region" aria-live="polite" aria-label={$t('quote.title')} on:touchstart|passive={onTouchStart} on:touchend|passive={onTouchEnd}>
    {#key day}
      <figure class="card glass" in:fade={{ duration: 220 }}>
        <svg class="lotus" viewBox="0 0 64 64" aria-hidden="true">
          <path d="M32 10c5 6 7.5 12 7.5 18 0 1.6-.2 3.1-.5 4.6 4-3.4 9.2-5.4 15-5.9-.9 7.3-4 13.3-9.1 17.5 4.9.6 9.4 2.6 13.1 6-5.4 5.4-12.8 8.4-21 8.4H32h-5c-8.2 0-15.6-3-21-8.4 3.7-3.4 8.2-5.4 13.1-6-5.1-4.2-8.2-10.2-9.1-17.5 5.8.5 11 2.5 15 5.9-.3-1.5-.5-3-.5-4.6C24.5 22 27 16 32 10z" />
        </svg>
        <blockquote lang={$lang}>{quote.text}</blockquote>
        {#if quote.source}<figcaption>{quote.source}</figcaption>{/if}
      </figure>
    {/key}

    <div class="arrows">
      <button class="arrow glass" on:click={() => move(-1)} aria-label={$t('quote.prev')} title={$t('quote.prev')}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
      </button>
      <button class="arrow glass" on:click={() => move(1)} aria-label={$t('quote.next')} title={$t('quote.next')}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
      </button>
    </div>
  </div>

  <button class="enter" on:click={onenter}>{$t('quote.enter')}</button>
</div>

<style>
  .quote-page {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    padding: calc(28px + env(safe-area-inset-top)) 20px calc(28px + env(safe-area-inset-bottom));
    background: var(--bg-overlay), var(--bg-image) center 30% / cover no-repeat, var(--bg);
    overflow-y: auto;
    user-select: none;
    -webkit-user-select: none;
  }
  .top {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-height: 52px;
    text-shadow: var(--title-shadow);
  }
  h1 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--accent);
  }
  .date {
    color: var(--muted);
    font-size: 0.9rem;
  }
  .today {
    border: none;
    background: none;
    padding: 2px 6px;
    color: var(--muted);
    font-size: 0.88rem;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .middle {
    flex: 1;
    width: 100%;
    max-width: 520px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 18px;
    touch-action: pan-y;
  }
  .card {
    margin: 0;
    padding: 26px 24px 22px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
  }
  .lotus {
    width: 34px;
    height: 34px;
    fill: var(--accent);
    opacity: 0.85;
  }
  blockquote {
    margin: 0;
    font-family: Georgia, 'Times New Roman', 'Hiragino Mincho ProN', 'Yu Mincho', 'Noto Serif CJK JP', serif;
    font-size: clamp(1.15rem, 4.6vw, 1.45rem);
    line-height: 1.55;
    color: var(--text);
    user-select: text;
    -webkit-user-select: text;
  }
  blockquote:lang(ja),
  blockquote:lang(zh),
  blockquote:lang(ko) {
    line-height: 1.75;
  }
  figcaption {
    color: var(--muted);
    font-size: 0.86rem;
    font-style: italic;
    line-height: 1.4;
  }
  figcaption::before {
    content: '— ';
  }
  .arrows {
    display: flex;
    justify-content: center;
    gap: 28px;
  }
  .arrow {
    width: 48px;
    height: 48px;
    padding: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--text);
    cursor: pointer;
    touch-action: manipulation;
  }
  .arrow:active {
    transform: scale(0.94);
  }
  .arrow svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .enter {
    width: min(100%, 320px);
    border: none;
    border-radius: 999px;
    padding: 15px 22px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 1.05rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    box-shadow: 0 8px 28px var(--accent-soft), var(--shadow);
    cursor: pointer;
  }
  .enter:active {
    transform: scale(0.98);
  }
</style>
