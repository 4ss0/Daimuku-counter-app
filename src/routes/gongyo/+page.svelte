<script lang="ts">
  // Gongyo as a karaoke: the line before, the line being recited with a
  // cursor running through it (in the accent colour), and the line after.
  // The speed is a slider; moving it shows a preview at that speed.
  import { onDestroy } from 'svelte';
  import { t, type Key } from '$lib/i18n';
  import { prefs, updatePrefs, GONGYO_SPEED } from '$lib/prefs';
  import { GONGYO, PART_START, cursorAt, lineSeconds, type GongyoPart } from '$lib/gongyo';
  import { keepScreen } from '$lib/native';

  /** Breath between two lines, in syllables. */
  const LINE_GAP = 0.6;
  const PREVIEW_TEXT = 'Nam-myoho-renge-kyo.';
  const PREVIEW_MS = 3000;

  let index = 0;
  let syl = 0; // syllables recited in the current line
  let playing = false;
  let finished = false;
  let speed = $prefs.gongyoSpeed;
  $: if (!sliding) speed = $prefs.gongyoSpeed;

  $: line = GONGYO[index];
  $: prev = GONGYO[index - 1];
  $: next = GONGYO[index + 1];
  $: length = lineSeconds(line.text, 1);
  $: cursor = cursorAt(line.text, 1, syl);
  $: done = line.text.slice(0, Math.floor(cursor));
  $: rest = line.text.slice(Math.floor(cursor));
  $: partLabel = $t(`gongyo.${line.part}` as Key) + (line.repeat ? ` · ${$t('gongyo.repeat', { n: line.repeat })}` : '');
  $: progress = (index + Math.min(1, syl / Math.max(0.1, length))) / GONGYO.length;

  let raf = 0;
  let last = 0;
  function frame(now: number) {
    const dt = Math.min(0.25, (now - last) / 1000);
    last = now;
    if (playing) {
      syl += dt * speed;
      if (syl >= length + LINE_GAP) {
        if (index + 1 >= GONGYO.length) {
          syl = length;
          finish();
        } else {
          index += 1;
          syl = 0;
        }
      }
    }
    if (previewUntil > now) previewSyl = (previewSyl + dt * speed) % (previewLength + LINE_GAP);
    else if (previewOn) previewOn = false;
    if (playing || previewOn) raf = requestAnimationFrame(frame);
    else raf = 0;
  }
  function run() {
    if (raf) return;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  }

  function play() {
    if (finished) restart();
    playing = true;
    keepScreen(true);
    run();
  }
  function pause() {
    playing = false;
    keepScreen(false);
  }
  function finish() {
    playing = false;
    finished = true;
    keepScreen(false);
  }
  function goTo(i: number) {
    index = Math.max(0, Math.min(GONGYO.length - 1, i));
    syl = 0;
    finished = false;
  }
  function restart() {
    goTo(0);
  }

  // the speed slider and its preview
  let sliding = false;
  let previewOn = false;
  let previewUntil = 0;
  let previewSyl = 0;
  const previewLength = lineSeconds(PREVIEW_TEXT, 1);
  $: previewCursor = cursorAt(PREVIEW_TEXT, 1, previewSyl);
  function onSlide(e: Event) {
    sliding = true;
    speed = parseFloat((e.currentTarget as HTMLInputElement).value);
    if (!previewOn) previewSyl = 0;
    previewOn = true;
    previewUntil = performance.now() + PREVIEW_MS;
    run();
  }
  function onSlideEnd() {
    sliding = false;
    updatePrefs({ gongyoSpeed: speed });
  }

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement) return;
    if (e.code === 'Space') {
      e.preventDefault();
      playing ? pause() : play();
    } else if (e.key === 'ArrowDown') goTo(index + 1);
    else if (e.key === 'ArrowUp') goTo(index - 1);
  }

  onDestroy(() => {
    if (raf) cancelAnimationFrame(raf);
    if (playing) keepScreen(false);
  });
</script>

<svelte:window on:keydown={onKey} />

<div class="page">
  <div class="parts" role="group">
    {#each ['hoben', 'juryo'] as const as p}
      <button class:on={line.part === p} on:click={() => goTo(PART_START[p as GongyoPart])}>{$t(`gongyo.${p}` as Key)}</button>
    {/each}
  </div>

  <section class="reader glass" aria-live="off">
    <div class="part">{partLabel}</div>
    <button class="line prev" on:click={() => goTo(index - 1)} disabled={!prev} aria-label={$t('gongyo.prev')}>
      {prev ? prev.text : ''}
    </button>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="line now" on:click={() => (playing ? pause() : play())}>
      <span class="done">{done}</span><span class="caret" class:blink={!playing} aria-hidden="true"></span><span>{rest}</span>
    </div>
    <button class="line next" on:click={() => goTo(index + 1)} disabled={!next} aria-label={$t('gongyo.next')}>
      {next ? next.text : ''}
    </button>
    <div class="progress"><span style="width:{progress * 100}%"></span></div>
  </section>

  {#if finished}<p class="end">{$t('gongyo.end')}</p>{/if}

  <div class="controls">
    <button class="ctl glass" on:click={restart} aria-label={$t('gongyo.restart')} title={$t('gongyo.restart')}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12a8 8 0 1 0 2.4-5.7M4 4v4.5h4.5" /></svg>
    </button>
    <button class="ctl glass" on:click={() => goTo(index - 1)} aria-label={$t('gongyo.prev')} title={$t('gongyo.prev')}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 15l6-6 6 6" /></svg>
    </button>
    <button class="go" on:click={() => (playing ? pause() : play())} aria-label={playing ? $t('gongyo.pause') : $t('gongyo.play')}>
      {#if playing}
        <svg viewBox="0 0 24 24"><rect x="6.5" y="5.5" width="4" height="13" rx="1.2" /><rect x="13.5" y="5.5" width="4" height="13" rx="1.2" /></svg>
      {:else}
        <svg viewBox="0 0 24 24"><path d="M8.5 5.8v12.4a1 1 0 0 0 1.5.86l10-6.2a1 1 0 0 0 0-1.72l-10-6.2a1 1 0 0 0-1.5.86z" /></svg>
      {/if}
    </button>
    <button class="ctl glass" on:click={() => goTo(index + 1)} aria-label={$t('gongyo.next')} title={$t('gongyo.next')}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </button>
  </div>

  <section class="card glass">
    <label class="speed">
      <span class="s-title">{$t('gongyo.speed')}</span>
      <span class="s-row">
        <span class="s-end">{$t('gongyo.slow')}</span>
        <input
          type="range"
          min={GONGYO_SPEED.min}
          max={GONGYO_SPEED.max}
          step="0.1"
          value={speed}
          on:input={onSlide}
          on:change={onSlideEnd}
        />
        <span class="s-end">{$t('gongyo.fast')}</span>
      </span>
    </label>
    {#if previewOn}
      <div class="preview">
        <span class="p-label">{$t('gongyo.preview')}</span>
        <span class="p-text"
          ><span class="done">{PREVIEW_TEXT.slice(0, Math.floor(previewCursor))}</span><span class="caret" aria-hidden="true"></span><span
            >{PREVIEW_TEXT.slice(Math.floor(previewCursor))}</span
          ></span
        >
      </div>
    {/if}
    <p class="hint">{$t('gongyo.hint')}</p>
  </section>
</div>

<style>
  .page {
    width: 100%;
    max-width: 560px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .parts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .parts button {
    padding: 9px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    cursor: pointer;
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .parts button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .reader {
    padding: 14px 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    text-align: center;
  }
  .part {
    color: var(--accent);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .line {
    border: none;
    background: none;
    padding: 4px 0;
    color: inherit;
    font: inherit;
    line-height: 1.35;
    min-height: 1.35em;
  }
  .prev,
  .next {
    color: var(--faint);
    font-size: 1.05rem;
    cursor: pointer;
  }
  .prev:disabled,
  .next:disabled {
    cursor: default;
  }
  .now {
    font-size: clamp(1.45rem, 6.4vw, 1.9rem);
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
  }
  .done {
    color: var(--accent);
  }
  .caret {
    display: inline-block;
    width: 3px;
    height: 1.05em;
    margin: 0 1px -0.15em;
    border-radius: 2px;
    background: var(--accent);
  }
  .caret.blink {
    animation: blink 1s steps(2, start) infinite;
  }
  @keyframes blink {
    to {
      visibility: hidden;
    }
  }
  .progress {
    height: 4px;
    border-radius: 2px;
    background: var(--surface-2);
    overflow: hidden;
    margin-top: 4px;
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .end {
    margin: 0;
    text-align: center;
    color: var(--green);
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }
  .ctl {
    width: 48px;
    height: 48px;
    padding: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--text);
    cursor: pointer;
  }
  .ctl svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.1;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .go {
    width: 76px;
    height: 76px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: var(--accent-ink);
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 8px 28px var(--accent-soft), var(--shadow);
  }
  .go svg {
    width: 32px;
    height: 32px;
    fill: currentColor;
  }
  .card {
    padding: 14px 16px;
  }
  .speed {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .s-title {
    font-weight: 600;
  }
  .s-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .s-end {
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  input[type='range'] {
    flex: 1;
    min-width: 0;
    accent-color: var(--accent);
  }
  .preview {
    margin-top: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--surface-2);
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: center;
  }
  .p-label {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .p-text {
    font-size: 1.2rem;
    font-weight: 600;
  }
  .hint {
    margin: 12px 0 0;
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.45;
  }
</style>
