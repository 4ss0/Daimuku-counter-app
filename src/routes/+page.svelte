<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, errText, fmtClock, fmtNum, type LiveSessionSummary, type LiveView, type SessionRecord } from '$lib/api';
  import { locale, t } from '$lib/i18n';
  import { todayTotal } from '$lib/stats';
  import { hasNative, shareFile, startCounting, stopCounting, updateCounting } from '$lib/native';
  import { share, sessionText } from '$lib/share';
  import { prefs } from '$lib/prefs';

  const POLL_MS = 150;
  const R = 118;
  const CIRC = 2 * Math.PI * R;

  let view: LiveView | null = null;
  let running = false;
  let busy = false;
  let summary: LiveSessionSummary | null = null;
  let corrected = 0;
  let errorText = '';
  let savedPath = '';
  // count animation: re-created on every increment, stronger at milestones
  let pulseKey = 0;
  let pulseTier: '' | 'p1' | 'p10' | 'p100' | 'p1000' = '';
  let lastCount = 0;
  // "goal reached" screen
  let celebrate = false;
  let celebrateTimer: ReturnType<typeof setTimeout> | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  // Daimoku found later by the checks wait in a small box under the ring
  // and fly into the counter one by one, like coins
  const COIN_GAP_MS = 320;
  const COIN_SPREAD_MS = 2400;
  let pending = 0;
  let lastRecovered = 0;
  let coins: { id: number; delay: number }[] = [];
  let coinSeq = 0;

  let sessions: SessionRecord[] = [];
  let goal = 100;
  let userTakes = -1;

  $: liveCount = running ? Math.max(0, (view?.count ?? 0) - pending) : summary ? corrected : 0;
  $: baseToday = todayTotal(sessions);
  $: today = running ? baseToday + liveCount : baseToday;
  $: progress = Math.min(1, today / Math.max(1, goal));
  $: goalReached = today >= goal;
  $: elapsed = view?.elapsed_secs ?? 0;
  $: statusLabel = !running
    ? summary
      ? $t('count.saved')
      : $t('count.ready')
    : view?.state === 'locked'
      ? $t('count.counting')
      : view?.speaking
        ? $t('count.listening')
        : $t('count.waiting');
  $: dateLabel = new Date().toLocaleDateString($locale, { weekday: 'long', day: 'numeric', month: 'long' });

  async function refresh() {
    try {
      [sessions, goal] = await Promise.all([api.listSessions(), api.getGoal()]);
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function poll() {
    try {
      const v = await api.liveView();
      const fresh = (v.recovered ?? 0) - lastRecovered;
      // with the app hidden (screen off) animations do not run: recovered
      // Daimoku go straight into the counter instead of queuing as coins
      if (fresh > 0 && running && !document.hidden) launchCoins(fresh);
      lastRecovered = v.recovered ?? 0;
      view = v;
      sync();
      if (running) updateCounting($t('notif.count', { n: fmtNum(liveCountOf(v), $locale) }));
    } catch (e) {
      errorText = errText(e);
    }
  }

  function liveCountOf(v: LiveView) {
    return Math.max(0, (v.count ?? 0) - pending);
  }

  // coins still in flight when the app is hidden would all land on return
  function onVisibility() {
    if (document.hidden) clearCoins();
  }

  /** Pulses for every Daimoku that reaches the counter. */
  function sync() {
    const shown = Math.max(0, (view?.count ?? 0) - pending);
    if (shown > lastCount) onIncrement(lastCount, shown);
    lastCount = shown;
  }

  // coins leave the box one after the other, also across batches
  let nextCoinAt = 0;
  function launchCoins(n: number) {
    pending += n;
    const gap = Math.min(COIN_GAP_MS, COIN_SPREAD_MS / n);
    const now = performance.now();
    const first = Math.max(now, nextCoinAt);
    for (let i = 0; i < n; i++) coins.push({ id: coinSeq++, delay: Math.round(first - now + i * gap) });
    nextCoinAt = first + n * gap;
    coins = coins;
  }

  function land(id: number) {
    coins = coins.filter((c) => c.id !== id);
    pending = Math.max(0, pending - 1);
    sync();
  }

  function clearCoins() {
    coins = [];
    pending = 0;
    nextCoinAt = 0;
  }

  /** Highest milestone (1000, 100, 10) crossed going from `a` to `b`. */
  function milestone(a: number, b: number): number {
    for (const m of [1000, 100, 10]) if (Math.floor(b / m) > Math.floor(a / m)) return m;
    return 1;
  }

  function onIncrement(from: number, to: number) {
    const m = milestone(from, to);
    pulseTier = m === 1000 ? 'p1000' : m === 100 ? 'p100' : m === 10 ? 'p10' : 'p1';
    pulseKey += 1;
    if (navigator.vibrate) {
      navigator.vibrate(m === 1000 ? [40, 60, 40, 60, 90] : m === 100 ? [30, 60, 30] : m === 10 ? 25 : 12);
    }
    // daily goal crossed during this session
    const before = baseToday + from;
    const after = baseToday + to;
    if (before < goal && after >= goal) showCelebration(after);
  }

  let celebrateCount = 0;
  function showCelebration(n: number) {
    celebrateCount = n;
    celebrate = true;
    if (navigator.vibrate) navigator.vibrate([60, 80, 60, 80, 120]);
    if (celebrateTimer) clearTimeout(celebrateTimer);
    celebrateTimer = setTimeout(() => (celebrate = false), 4000);
  }

  function closeCelebration() {
    celebrate = false;
    if (celebrateTimer) clearTimeout(celebrateTimer);
    celebrateTimer = null;
  }

  async function start() {
    if (busy || running) return;
    busy = true;
    errorText = '';
    savedPath = '';
    summary = null;
    shareMsg = '';
    lastCount = 0;
    lastRecovered = 0;
    clearCoins();
    view = null;
    try {
      await api.startLive();
      running = true;
      timer = setInterval(poll, POLL_MS);
      // Android: keeps counting with the screen off (shows a notification)
      startCounting($t('notif.title'), $t('notif.text'), $prefs.keepAwake);
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    if (busy || !running) return;
    busy = true;
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    stopCounting();
    try {
      summary = await api.stopLive();
      corrected = summary.count;
      clearCoins();
      await poll();
      await refresh();
    } catch (e) {
      errorText = errText(e);
    } finally {
      running = false;
      busy = false;
    }
  }

  function adjust(delta: number) {
    if (!summary?.session) return;
    corrected = Math.max(0, corrected + delta);
    if (saveTimer) clearTimeout(saveTimer);
    const id = summary.session.id;
    const value = corrected;
    saveTimer = setTimeout(async () => {
      try {
        await api.updateSessionCount(id, value);
        await refresh();
      } catch (e) {
        errorText = errText(e);
      }
    }, 500);
  }

  async function discard() {
    if (!summary?.session) return;
    try {
      await api.deleteSession(summary.session.id);
      summary = null;
      await refresh();
    } catch (e) {
      errorText = errText(e);
    }
  }

  let shareMsg = '';
  async function shareSession() {
    if (!summary?.session) return;
    shareMsg = await share(sessionText({ ...summary.session, count: corrected }));
  }

  async function saveWav() {
    try {
      const f = await api.exportLiveWav();
      if (hasNative()) await shareFile(f, 'audio/wav', $t('count.saveAudio'));
      else savedPath = f.path;
    } catch (e) {
      errorText = errText(e);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.code === 'Space' && !(e.target instanceof HTMLInputElement)) {
      e.preventDefault();
      running ? stop() : start();
    }
  }

  onMount(async () => {
    window.addEventListener('keydown', onKey);
    document.addEventListener('visibilitychange', onVisibility);
    await refresh();
    try {
      userTakes = (await api.profile()).takes.length;
    } catch {
      userTakes = -1;
    }
  });

  onDestroy(() => {
    window.removeEventListener('keydown', onKey);
    document.removeEventListener('visibilitychange', onVisibility);
    if (timer) clearInterval(timer);
    if (celebrateTimer) clearTimeout(celebrateTimer);
    if (running) {
      stopCounting();
      api.stopLive().catch(() => {});
    }
  });
</script>

<div class="page">
  <header>
    <div class="date">{dateLabel}</div>
    <div class="today">
      {$t('count.today')} <strong>{fmtNum(today, $locale)}</strong> <span class="of">/ {fmtNum(goal, $locale)}</span>
      {#if goalReached}<span class="done" aria-label={$t('count.goalReached')}>✓</span>{/if}
    </div>
  </header>

  <div class="ring-wrap" class:speaking={running && view?.speaking}>
    {#key pulseKey}
      {#if pulseTier === 'p10' || pulseTier === 'p100' || pulseTier === 'p1000'}
        <div class="halo {pulseTier}" aria-hidden="true"></div>
      {/if}
      {#if pulseTier === 'p100' || pulseTier === 'p1000'}
        <div class="ripple {pulseTier}" aria-hidden="true"></div>
      {/if}
      {#if pulseTier === 'p1000'}
        <div class="burst" aria-hidden="true">
          {#each Array(12) as _, i}
            <span style="--a:{i * 30}deg"></span>
          {/each}
        </div>
      {/if}
    {/key}
    <div class="ring-glass"></div>
    <div class="coins" aria-hidden="true">
      {#each coins as c (c.id)}
        <span class="coin" style="animation-delay:{c.delay}ms" on:animationend={() => land(c.id)}></span>
      {/each}
    </div>
    <svg class="ring" viewBox="0 0 280 280" aria-hidden="true">
      <circle class="track" cx="140" cy="140" r={R} />
      <circle
        class="bar"
        class:full={goalReached}
        cx="140"
        cy="140"
        r={R}
        stroke-dasharray={CIRC}
        stroke-dashoffset={CIRC * (1 - progress)}
      />
    </svg>
    <div class="center">
      {#key pulseKey}
        <div class="count {pulseTier}" class:d4={liveCount >= 1000} class:d5={liveCount >= 10000} aria-live="polite">
          {liveCount}
        </div>
      {/key}
      <div class="unit">{$t('count.unit')}</div>
    </div>
  </div>

  {#if running && (pending > 0 || lastRecovered > 0)}
    <div class="coinbox" class:empty={pending === 0} title={$t('count.recoveredHint')} aria-label={$t('count.recoveredHint')}>
      <span class="coin-icon" aria-hidden="true"></span>
      <span class="coin-n">{pending}</span>
      <span class="coin-label">{$t('count.recovered')}</span>
    </div>
  {/if}

  <div class="status">
    <span class="dot" class:on={running && view?.speaking} class:live={running}></span>
    <span>{statusLabel}</span>
    {#if running || summary}
      <span class="clock">{fmtClock(running ? elapsed : summary?.duration_secs ?? 0)}</span>
    {/if}
  </div>

  <button
    class="go"
    class:stop={running}
    on:click={running ? stop : start}
    disabled={busy}
    aria-label={running ? $t('count.stop') : $t('count.start')}
  >
    {#if running}
      <svg viewBox="0 0 24 24"><rect x="6.5" y="6.5" width="11" height="11" rx="2.5" /></svg>
    {:else}
      <svg viewBox="0 0 24 24"><path d="M8.5 5.8v12.4a1 1 0 0 0 1.5.86l10-6.2a1 1 0 0 0 0-1.72l-10-6.2a1 1 0 0 0-1.5.86z" /></svg>
    {/if}
  </button>
  <div class="go-label">{running ? $t('count.stop') : $t('count.start')}</div>

  {#if summary && !running}
    <section class="sheet glass">
      {#if summary.session}
        <div class="sheet-row">
          <div>
            <div class="sheet-title">{$t('count.saved')}</div>
            <div class="sheet-sub">
              {fmtClock(summary.duration_secs)}
              {#if corrected !== summary.count}· {$t('count.detected', { n: summary.count })}{/if}
            </div>
          </div>
          <div class="stepper" role="group">
            <button on:click={() => adjust(-1)} aria-label={$t('common.less')}>−</button>
            <span>{corrected}</span>
            <button on:click={() => adjust(1)} aria-label={$t('common.more')}>+</button>
          </div>
        </div>
        <p class="hint">{$t('count.correctHint')}</p>
        <div class="sheet-actions">
          <button class="link" on:click={shareSession}>{$t('share.button')}</button>
          <button class="link" on:click={saveWav}>{$t('count.saveAudio')}</button>
          <button class="link danger" on:click={discard}>{$t('count.deleteSession')}</button>
        </div>
        {#if savedPath}<p class="saved">{$t('count.savedIn', { p: savedPath })}</p>{/if}
        {#if shareMsg}<p class="saved">{shareMsg}</p>{/if}
      {:else}
        <div class="sheet-title">{$t('count.noneTitle')}</div>
        <p class="hint">{$t('count.noneHint')}</p>
      {/if}
    </section>
  {:else if !running && userTakes === 0}
    <a class="tip glass" href="/voice">
      <strong>{$t('count.tipTitle')}</strong>
      <span>{$t('count.tipText')}</span>
    </a>
  {/if}

  {#if errorText}
    <p class="error">{errorText}</p>
  {/if}
</div>

{#if celebrate}
  <button class="celebrate" on:click={closeCelebration} aria-live="assertive">
    <div class="celebrate-card glass">
      <svg class="lotus" viewBox="0 0 64 64" aria-hidden="true">
        <path d="M32 10c5 6 7.5 12 7.5 18 0 1.6-.2 3.1-.5 4.6 4-3.4 9.2-5.4 15-5.9-.9 7.3-4 13.3-9.1 17.5 4.9.6 9.4 2.6 13.1 6-5.4 5.4-12.8 8.4-21 8.4H32h-5c-8.2 0-15.6-3-21-8.4 3.7-3.4 8.2-5.4 13.1-6-5.1-4.2-8.2-10.2-9.1-17.5 5.8.5 11 2.5 15 5.9-.3-1.5-.5-3-.5-4.6C24.5 22 27 16 32 10z" />
      </svg>
      <div class="celebrate-title">{$t('count.goalTitle')}</div>
      <div class="celebrate-text">{$t('count.goalText', { n: fmtNum(celebrateCount, $locale) })}</div>
      <div class="celebrate-hint">{$t('count.goalTap')}</div>
    </div>
  </button>
{/if}

<style>
  .page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 18px 20px 24px;
    gap: 14px;
    user-select: none;
    -webkit-user-select: none;
  }

  header {
    width: 100%;
    max-width: 420px;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    color: var(--muted);
    font-size: 0.9rem;
    padding: 7px 14px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--line);
    -webkit-backdrop-filter: blur(14px);
    backdrop-filter: blur(14px);
  }
  .date::first-letter {
    text-transform: uppercase;
  }
  .today {
    white-space: nowrap;
  }
  .today strong {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .today .of {
    color: var(--faint);
  }
  .done {
    color: var(--green);
    margin-left: 4px;
  }

  .ring-wrap {
    position: relative;
    width: min(78vw, 320px);
    aspect-ratio: 1;
    margin-top: 8px;
    border-radius: 50%;
    transition: box-shadow 0.25s;
  }
  .ring-glass {
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    background: var(--surface);
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .coins {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 3;
  }
  .coin {
    position: absolute;
    left: 50%;
    top: calc(100% + 30px);
    width: 22px;
    height: 22px;
    margin: -11px 0 0 -11px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #fff7cf, #f2c94c 45%, #b9861a 100%);
    box-shadow: 0 0 12px rgba(242, 201, 76, 0.75);
    opacity: 0;
    animation: coin-fly 0.75s cubic-bezier(0.3, 0.7, 0.4, 1) both;
  }
  @keyframes coin-fly {
    0% {
      top: calc(100% + 30px);
      transform: scale(0.6);
      opacity: 0;
    }
    15% {
      transform: scale(1.05);
      opacity: 1;
    }
    80% {
      opacity: 1;
    }
    100% {
      top: 50%;
      transform: scale(0.35);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .coin {
      animation-duration: 0.01s;
    }
  }
  .coinbox {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: -2px;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--line);
    color: var(--muted);
    font-size: 0.85rem;
    transition: opacity 0.4s;
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .coinbox.empty {
    opacity: 0.5;
  }
  .coin-icon {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: radial-gradient(circle at 35% 30%, #fff7cf, #f2c94c 45%, #b9861a 100%);
  }
  .coin-n {
    color: var(--text);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .ring-wrap.speaking {
    box-shadow: 0 0 60px var(--accent-soft);
  }
  .ring {
    position: relative;
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  .ring circle {
    fill: none;
    stroke-width: 10;
  }
  .ring .track {
    stroke: var(--surface-2);
  }
  .ring .bar {
    stroke: var(--accent);
    stroke-linecap: round;
    transition: stroke-dashoffset 0.4s ease;
  }
  .ring .bar.full {
    stroke: var(--green);
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .count {
    font-size: clamp(4.5rem, 24vw, 7.5rem);
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.03em;
  }
  .count.d4 {
    font-size: clamp(3.4rem, 18vw, 5.6rem);
  }
  .count.d5 {
    font-size: clamp(2.8rem, 15vw, 4.6rem);
  }
  /* one pulse per Daimoku; stronger at every 10, 100 and 1000 */
  .count.p1 {
    animation: pulse1 0.45s ease-out;
  }
  .count.p10 {
    animation: pulse10 0.65s ease-out;
  }
  .count.p100 {
    animation: pulse100 0.9s cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  .count.p1000 {
    animation: pulse1000 1.4s cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  @keyframes pulse1 {
    0% {
      transform: scale(1.1);
      color: var(--accent);
    }
    100% {
      transform: scale(1);
    }
  }
  @keyframes pulse10 {
    0% {
      transform: scale(1.22);
      color: var(--accent);
    }
    60% {
      color: var(--accent);
    }
    100% {
      transform: scale(1);
    }
  }
  @keyframes pulse100 {
    0% {
      transform: scale(0.9);
      color: var(--accent);
    }
    35% {
      transform: scale(1.35);
      color: var(--accent);
    }
    100% {
      transform: scale(1);
    }
  }
  @keyframes pulse1000 {
    0% {
      transform: scale(0.85) rotate(-4deg);
      color: var(--accent);
    }
    30% {
      transform: scale(1.3) rotate(3deg);
      color: var(--accent);
    }
    55% {
      transform: scale(1.15) rotate(0);
      color: var(--accent);
    }
    100% {
      transform: scale(1);
    }
  }
  .halo {
    position: absolute;
    inset: 2%;
    border-radius: 50%;
    pointer-events: none;
    box-shadow: 0 0 0 0 var(--accent);
    animation: halo 0.7s ease-out forwards;
  }
  .halo.p100 {
    animation-duration: 1.1s;
  }
  .halo.p1000 {
    animation-duration: 1.6s;
  }
  @keyframes halo {
    0% {
      box-shadow: 0 0 0 0 var(--accent-soft), inset 0 0 0 0 var(--accent-soft);
    }
    30% {
      box-shadow: 0 0 50px 14px var(--accent-soft), inset 0 0 40px 6px var(--accent-soft);
    }
    100% {
      box-shadow: 0 0 0 0 transparent, inset 0 0 0 0 transparent;
    }
  }
  .ripple {
    position: absolute;
    inset: 4%;
    border-radius: 50%;
    border: 3px solid var(--accent);
    pointer-events: none;
    animation: ripple 1s ease-out forwards;
  }
  .ripple.p1000 {
    border-width: 4px;
    animation-duration: 1.5s;
  }
  @keyframes ripple {
    0% {
      transform: scale(0.9);
      opacity: 0.9;
    }
    100% {
      transform: scale(1.35);
      opacity: 0;
    }
  }
  .burst {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .burst span {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 12px;
    height: 22px;
    margin: -11px 0 0 -6px;
    border-radius: 50% 50% 50% 50% / 70% 70% 30% 30%;
    background: var(--accent);
    opacity: 0;
    transform: rotate(var(--a)) translateY(-60px);
    animation: petal 1.5s ease-out forwards;
  }
  @keyframes petal {
    0% {
      opacity: 0;
      transform: rotate(var(--a)) translateY(-70px) scale(0.4);
    }
    20% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: rotate(var(--a)) translateY(-190px) scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .count.p10,
    .count.p100,
    .count.p1000 {
      animation: pulse1 0.45s ease-out;
    }
    .ripple,
    .burst {
      display: none;
    }
  }

  /* daily goal reached */
  .celebrate {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    border: 0;
    background: rgba(0, 0, 0, 0.35);
    -webkit-backdrop-filter: blur(6px);
    backdrop-filter: blur(6px);
    cursor: pointer;
    animation: fade-in 0.3s ease-out;
  }
  .celebrate-card {
    width: min(100%, 340px);
    padding: 30px 24px 22px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
    animation: pop-in 0.55s cubic-bezier(0.2, 0.9, 0.3, 1.25);
  }
  .lotus {
    width: 76px;
    height: 76px;
    fill: var(--accent);
    margin-bottom: 6px;
    animation: bloom 1.2s ease-out;
  }
  .celebrate-title {
    font-size: 1.45rem;
    font-weight: 700;
  }
  .celebrate-text {
    color: var(--muted);
    font-size: 1rem;
  }
  .celebrate-hint {
    margin-top: 10px;
    color: var(--faint);
    font-size: 0.78rem;
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
  @keyframes pop-in {
    0% {
      transform: scale(0.8);
      opacity: 0;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
  @keyframes bloom {
    0% {
      transform: scale(0.3) rotate(-20deg);
      opacity: 0;
    }
    60% {
      transform: scale(1.12) rotate(4deg);
      opacity: 1;
    }
    100% {
      transform: scale(1) rotate(0);
    }
  }
  .unit {
    margin-top: 6px;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.9rem;
    min-height: 1.4rem;
    padding: 4px 12px;
    border-radius: 999px;
    background: var(--surface);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--line);
    transition: background 0.15s, box-shadow 0.15s;
  }
  .dot.live {
    background: var(--faint);
  }
  .dot.on {
    background: var(--green);
    box-shadow: 0 0 10px var(--green);
  }
  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--faint);
  }

  button.go {
    margin-top: 6px;
    width: 84px;
    height: 84px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: var(--accent-ink);
    display: grid;
    place-items: center;
    cursor: pointer;
    box-shadow: 0 8px 28px var(--accent-soft), var(--shadow);
    transition: transform 0.1s, background 0.2s, box-shadow 0.2s;
  }
  button.go:active {
    transform: scale(0.95);
  }
  button.go.stop {
    background: var(--red);
    color: #fff;
  }
  button.go:disabled {
    opacity: 0.6;
  }
  button.go svg {
    width: 34px;
    height: 34px;
    fill: currentColor;
  }
  .go-label {
    margin-top: -6px;
    font-size: 0.8rem;
    color: var(--muted);
    text-shadow: var(--title-shadow);
  }

  .sheet {
    width: 100%;
    max-width: 420px;
    padding: 16px 18px;
    margin-top: 4px;
  }
  .sheet-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .sheet-title {
    font-weight: 600;
  }
  .sheet-sub {
    color: var(--muted);
    font-size: 0.85rem;
    margin-top: 2px;
    font-variant-numeric: tabular-nums;
  }
  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 4px;
  }
  .stepper button {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: transparent;
    font-size: 1.3rem;
    cursor: pointer;
  }
  .stepper button:active {
    background: var(--line);
  }
  .stepper span {
    min-width: 3ch;
    text-align: center;
    font-weight: 700;
    font-size: 1.15rem;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.4;
    margin: 10px 0 4px;
  }
  .sheet-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px;
    margin-top: 6px;
  }
  button.link {
    background: none;
    border: none;
    padding: 6px 0;
    color: var(--muted);
    font-size: 0.82rem;
    text-decoration: underline;
    text-underline-offset: 3px;
    cursor: pointer;
  }
  button.link.danger {
    color: var(--red);
  }
  .saved {
    color: var(--green);
    font-size: 0.78rem;
    word-break: break-all;
    margin: 6px 0 0;
  }

  .tip {
    width: 100%;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 14px 16px;
    color: var(--text);
    text-decoration: none;
    font-size: 0.88rem;
    border-color: var(--accent-soft);
  }
  .tip strong {
    color: var(--accent);
  }
  .tip span {
    color: var(--muted);
  }

  .error {
    color: var(--red);
    font-size: 0.85rem;
    text-align: center;
    max-width: 420px;
    background: var(--surface);
    padding: 8px 12px;
    border-radius: 10px;
  }
</style>
