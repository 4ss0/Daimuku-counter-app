<script lang="ts">
  // Gongyo, to recite and to understand.
  //
  // Recite: the whole text in a column rolling up like film credits, one
  // beat per kanji (see gongyo.ts). In the middle are the two lines to read,
  // whose letters fill with the accent colour as they are recited; above,
  // the two just recited; below, the ones to come. The pace is a slider, and
  // moving it plays a preview at that pace.
  //
  // Translation: both chapters line by line (pronunciation, original text,
  // translation) in Italian, English or Japanese.
  import { onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import { LANGS, lang, t, type Key, type Lang } from '$lib/i18n';
  import { prefs, updatePrefs } from '$lib/prefs';
  import {
    BEATS_BEFORE,
    GONGYO,
    PACE,
    PART_START,
    TOTAL_BEATS,
    pieces,
    type GongyoLine,
    type GongyoPart,
  } from '$lib/gongyo';
  import { HOBEN_TR, JURYO_TR, TEN_FACTORS_TR, type TrLine } from '$lib/gongyoTranslation';
  import { keepScreen } from '$lib/native';
  import KaraokeLine from '$lib/KaraokeLine.svelte';

  const PREVIEW = pieces('Myo ho ren ge kyo.');
  const PREVIEW_BEATS = PREVIEW.filter((p) => p.beat).length;
  const PREVIEW_MS = 3500;
  /** Silent beats before the first line, to get ready. */
  const LEAD_IN = 2;
  /** How quickly the column catches up with where it should be (s). */
  const FOLLOW = 0.16;

  let tab: 'recite' | 'translation' = 'recite';
  let index = 0;
  let beat = 0; // beats recited in the current line (negative: lead-in)
  let playing = false;
  let finished = false;
  let sliding = false;
  let pace = $prefs.gongyoPace;
  $: if (!sliding) pace = $prefs.gongyoPace;

  $: line = GONGYO[index];
  $: done = BEATS_BEFORE[index] + Math.max(0, beat);
  $: progress = done / TOTAL_BEATS;

  function clock(secs: number): string {
    const s = Math.max(0, Math.round(secs));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  }

  // ---- the rolling column ----
  let view: HTMLElement;
  let rows: HTMLElement[] = [];
  let mids: number[] = []; // per line: halfway down it and the next one
  let speeds: number[] = []; // per line: how fast the column rolls through it (px a beat)
  let viewHeight = 0;
  let y = 0; // the point of the column shown in the middle of the view

  /**
   * The point of the column for the middle of the view: halfway down the two
   * lines to read, sliding on to the next two as the first is recited. Lines
   * have different lengths, so the roll goes through the halfway points on a
   * smooth curve rather than straight from one to the next: its speed
   * changes gradually, without a jolt at each line.
   */
  function aim(k: number, b: number): number {
    if (mids.length !== GONGYO.length) return 0;
    const last = GONGYO.length - 1;
    if (k >= last) return mids[last];
    const n = GONGYO[k].beats;
    const t = Math.min(1, Math.max(0, b) / n);
    // cubic Hermite from mids[k] to mids[k + 1]
    const t2 = t * t;
    const t3 = t2 * t;
    return (
      (2 * t3 - 3 * t2 + 1) * mids[k] +
      (t3 - 2 * t2 + t) * n * speeds[k] +
      (-2 * t3 + 3 * t2) * mids[k + 1] +
      (t3 - t2) * n * speeds[k + 1]
    );
  }
  function measure() {
    if (!view || rows.some((r) => !r)) return;
    const last = GONGYO.length - 1;
    const tops = rows.map((r) => r.offsetTop);
    const bottoms = rows.map((r) => r.offsetTop + r.offsetHeight);
    mids = tops.map((top, i) => (top + bottoms[Math.min(i + 1, last)]) / 2);
    // the average of the slopes either side, kept low enough that the roll
    // never goes back (Fritsch-Carlson)
    const slope = GONGYO.map((l, i) => (i < last ? (mids[i + 1] - mids[i]) / l.beats : 0));
    speeds = slope.map((s, i) => {
      const before = slope[i - 1] ?? s;
      const after = i < last ? s : before;
      return Math.min((before + after) / 2, 3 * Math.min(before, after));
    });
    viewHeight = view.clientHeight;
    y = aim(index, beat);
  }
  /** Measure the lines again whenever the layout changes (width, font size). */
  function roll(node: HTMLElement) {
    view = node;
    const ro = new ResizeObserver(measure);
    ro.observe(node);
    if (node.firstElementChild) ro.observe(node.firstElementChild);
    return { destroy: () => ro.disconnect() };
  }

  // ---- the beat ----
  let raf = 0;
  let last = 0;
  function frame(now: number) {
    const dt = Math.min(0.25, (now - last) / 1000);
    last = now;
    if (playing) {
      beat += dt * pace;
      // no pause between lines: what is left over goes into the next one
      while (beat >= GONGYO[index].beats) {
        if (index + 1 >= GONGYO.length) {
          beat = GONGYO[index].beats;
          finish();
          break;
        }
        beat -= GONGYO[index].beats;
        index += 1;
      }
    }
    const goal = aim(index, beat);
    y += (goal - y) * (1 - Math.exp(-dt / FOLLOW));
    if (Math.abs(goal - y) < 0.25) y = goal;
    if (previewOn) {
      if (now > previewUntil) previewOn = false;
      else previewBeat = (previewBeat + dt * pace) % (PREVIEW_BEATS + 1);
    }
    raf = playing || previewOn || y !== goal ? requestAnimationFrame(frame) : 0;
  }
  function run() {
    if (raf) return;
    last = performance.now();
    raf = requestAnimationFrame(frame);
  }

  function play() {
    if (finished) goTo(0);
    if (index === 0 && beat === 0) beat = -LEAD_IN;
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
    beat = 0;
    finished = false;
    run();
  }
  /** A tap on a line goes to it; on the line being recited, pauses or goes on. */
  function tap(i: number) {
    if (i !== index) goTo(i);
    else if (playing) pause();
    else play();
  }
  function show(t: typeof tab) {
    if (t !== 'recite' && playing) pause();
    tab = t;
  }

  // ---- the pace slider and its preview ----
  let previewOn = false;
  let previewUntil = 0;
  let previewBeat = 0;
  function onSlide(e: Event) {
    sliding = true;
    pace = parseFloat((e.currentTarget as HTMLInputElement).value);
    if (!previewOn) previewBeat = 0;
    previewOn = true;
    previewUntil = performance.now() + PREVIEW_MS;
    run();
  }
  function onSlideEnd() {
    sliding = false;
    updatePrefs({ gongyoPace: pace });
  }
  const MARKS: { at: number; key: Key }[] = [
    { at: PACE.slow, key: 'gongyo.slow' },
    { at: PACE.medium, key: 'gongyo.medium' },
    { at: PACE.fast, key: 'gongyo.fast' },
  ];
  const pct = (v: number) => ((v - PACE.min) / (PACE.max - PACE.min)) * 100;

  function onKey(e: KeyboardEvent) {
    if (tab !== 'recite' || e.target instanceof HTMLInputElement) return;
    if (e.code === 'Space') {
      e.preventDefault();
      playing ? pause() : play();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      goTo(index + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      goTo(index - 1);
    }
  }

  // ---- the translation ----
  // the app's language until one is picked here
  let picked: Lang | null = null;
  $: trLang = picked ?? $lang;
  const COLUMN: Record<Lang, 1 | 2 | 3> = { it: 1, en: 2, ja: 3 };
  /** The titles need no translation into Japanese. */
  const same = (a: string, b: string) => a.replace(/\s/g, '') === b.replace(/\s/g, '');
  const zip = (ls: GongyoLine[], tr: TrLine[]) => ls.map((l, i) => ({ text: l.text, tr: tr[i] }));
  const CHAPTERS: { title: Key; blocks: { label?: Key; lines: { text: string; tr: TrLine }[] }[] }[] = [
    {
      title: 'gongyo.hoben',
      blocks: [
        { lines: zip(GONGYO.filter((l) => l.part === 'hoben' && !l.repeat), HOBEN_TR) },
        { label: 'gongyo.tenFactors', lines: zip(GONGYO.filter((l) => l.repeat === 1), TEN_FACTORS_TR) },
      ],
    },
    { title: 'gongyo.juryo', blocks: [{ lines: zip(GONGYO.filter((l) => l.part === 'juryo'), JURYO_TR) }] },
  ];

  onDestroy(() => {
    if (raf) cancelAnimationFrame(raf);
    if (playing) keepScreen(false);
  });
</script>

<svelte:window on:keydown={onKey} />

<div class="page">
  <div class="tabs" role="tablist">
    {#each [['recite', 'gongyo.recite'], ['translation', 'gongyo.translation']] as const as [id, key]}
      <button role="tab" aria-selected={tab === id} class:on={tab === id} on:click={() => show(id)}>{$t(key)}</button>
    {/each}
  </div>

  {#if tab === 'recite'}
    <section class="reader glass">
      <header>
        <div class="parts" role="group">
          {#each ['hoben', 'juryo'] as const as p}
            <button class:on={line.part === p} on:click={() => goTo(PART_START[p as GongyoPart])}>{$t(`gongyo.${p}` as Key)}</button>
          {/each}
        </div>
        {#if line.repeat}
          <span class="rep" aria-label={$t('gongyo.repeat', { n: line.repeat })} title={$t('gongyo.repeat', { n: line.repeat })}>
            {#each [1, 2, 3] as r}<span class="dot" class:on={r <= line.repeat}></span>{/each}
          </span>
        {/if}
        <span class="time">{clock(done / pace)} / {clock(TOTAL_BEATS / pace)}</span>
      </header>

      <div class="view" use:roll>
        <div class="col" style="transform: translate3d(0, {(viewHeight / 2 - y).toFixed(2)}px, 0)">
          {#each GONGYO as l, i}
            <button
              class="row"
              class:cur={i === index || i === index + 1}
              tabindex="-1"
              bind:this={rows[i]}
              on:click={() => tap(i)}
            >
              <KaraokeLine pieces={l.pieces} beat={i < index ? l.beats : i === index ? Math.max(0, beat) : 0} />
            </button>
          {/each}
        </div>
      </div>

      <div class="bar"><span style="width:{progress * 100}%"></span></div>
    </section>

    {#if finished}<p class="end">{$t('gongyo.end')}</p>{/if}

    <div class="controls">
      <button class="ctl glass" on:click={() => goTo(0)} aria-label={$t('gongyo.restart')} title={$t('gongyo.restart')}>
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
      <div class="pace-head">
        <span class="title">{$t('gongyo.speed')}</span>
        <span class="muted">{$t('gongyo.duration', { t: clock(TOTAL_BEATS / pace) })}</span>
      </div>
      <div class="slider">
        <input
          type="range"
          min={PACE.min}
          max={PACE.max}
          step="0.05"
          value={pace}
          on:input={onSlide}
          on:change={onSlideEnd}
          aria-label={$t('gongyo.speed')}
        />
        <div class="marks" aria-hidden="true">
          {#each MARKS as m}
            <span class:near={Math.abs(pace - m.at) < 0.2} style="left:{pct(m.at)}%">{$t(m.key)}</span>
          {/each}
        </div>
      </div>
      {#if previewOn}
        <div class="preview" transition:fly={{ y: -6, duration: 160 }}>
          <KaraokeLine pieces={PREVIEW} beat={Math.min(PREVIEW_BEATS, previewBeat)} />
        </div>
      {/if}
      <p class="hint">{$t('gongyo.hint')}</p>
    </section>
  {:else}
    <div class="langs" role="group" aria-label={$t('gongyo.trLang')}>
      {#each LANGS as l}
        <button class:on={trLang === l.id} lang={l.id} on:click={() => (picked = l.id)}>{l.name}</button>
      {/each}
    </div>
    <p class="note">{$t('gongyo.trNote')}</p>

    {#each CHAPTERS as ch}
      <section class="chapter glass">
        <h2>{$t(ch.title)}</h2>
        {#each ch.blocks as b}
          {#if b.label}<h3>{$t(b.label)}</h3>{/if}
          <ol>
            {#each b.lines as l}
              <li>
                <p class="ro">{l.text}</p>
                <p class="kj" lang="ja">{l.tr[0]}</p>
                {#if !same(l.tr[0], l.tr[COLUMN[trLang]])}<p class="tx" lang={trLang}>{l.tr[COLUMN[trLang]]}</p>{/if}
              </li>
            {/each}
          </ol>
        {/each}
      </section>
    {/each}
  {/if}
</div>

<style>
  .page {
    width: 100%;
    max-width: 600px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .tabs,
  .langs {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 4px;
    padding: 4px;
    border-radius: 14px;
    background: var(--surface);
    border: 1px solid var(--line);
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .tabs button,
  .langs button {
    padding: 9px;
    border-radius: 10px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    cursor: pointer;
  }
  .tabs button.on,
  .langs button.on {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .reader {
    padding: 12px 14px 14px;
    container-type: inline-size;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }
  .parts {
    display: flex;
    gap: 4px;
  }
  .parts button {
    padding: 4px 9px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    cursor: pointer;
  }
  .parts button.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .rep {
    display: inline-flex;
    gap: 4px;
  }
  .rep .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1.5px solid var(--accent);
  }
  .rep .dot.on {
    background: var(--accent);
  }
  .time {
    margin-left: auto;
    color: var(--faint);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }

  /* six lines high: two recited, two to read, two to come; they fade in at
     the bottom and out at the top */
  .view {
    --k-text: var(--text);
    position: relative;
    height: 13.2em;
    margin: 6px -14px 10px;
    overflow: hidden;
    font-size: clamp(1.02rem, 5cqw, 1.45rem);
    font-weight: 700;
    line-height: 1.4;
    -webkit-mask-image: linear-gradient(to bottom, transparent, #000 16%, #000 84%, transparent);
    mask-image: linear-gradient(to bottom, transparent, #000 16%, #000 84%, transparent);
    user-select: none;
    -webkit-user-select: none;
  }
  .col {
    position: relative;
    will-change: transform;
  }
  .row {
    display: block;
    width: 100%;
    padding: 0.3em 14px;
    border: none;
    background: none;
    font: inherit;
    color: var(--text);
    text-align: center;
    text-wrap: balance;
    opacity: 0.34;
    transition: opacity 0.45s ease;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
  }
  .row.cur {
    opacity: 1;
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
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
  .pace-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
  }
  .title {
    font-weight: 600;
  }
  .muted {
    color: var(--muted);
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
  }
  .slider {
    position: relative;
    margin: 10px 0 0;
    padding-bottom: 22px;
  }
  input[type='range'] {
    width: 100%;
    margin: 0;
    accent-color: var(--accent);
  }
  .marks span {
    position: absolute;
    bottom: 0;
    transform: translateX(-50%);
    color: var(--faint);
    font-size: 0.74rem;
    white-space: nowrap;
  }
  .marks span::before {
    content: '';
    position: absolute;
    left: 50%;
    top: -7px;
    width: 1px;
    height: 5px;
    background: var(--faint);
  }
  .marks span.near {
    color: var(--accent);
    font-weight: 600;
  }
  .preview {
    --k-text: var(--text);
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--surface-2);
    text-align: center;
    font-size: 1.35rem;
    font-weight: 700;
  }
  .hint {
    margin: 12px 0 0;
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .note {
    margin: 0 4px;
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.45;
  }
  .chapter {
    padding: 16px 16px 6px;
  }
  h2 {
    margin: 0 0 4px;
    color: var(--accent);
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  h3 {
    margin: 14px 0 0;
    padding-top: 12px;
    border-top: 1px solid var(--accent-soft);
    color: var(--accent);
    font-size: 0.78rem;
    font-weight: 600;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: none;
  }
  li p {
    margin: 0;
  }
  .ro {
    color: var(--muted);
    font-size: 0.8rem;
    font-style: italic;
  }
  .kj {
    margin-top: 2px;
    font-family: 'Hiragino Mincho ProN', 'Yu Mincho', 'Noto Serif CJK JP', 'Noto Serif JP', serif;
    font-size: 1.02rem;
    letter-spacing: 0.08em;
    color: var(--text);
  }
  .tx {
    margin-top: 4px;
    font-size: 0.96rem;
    line-height: 1.5;
    color: var(--text);
  }
  .tx:lang(ja) {
    line-height: 1.7;
  }
</style>
