<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { getVersion } from '@tauri-apps/api/app';
  import { api, fmtNum } from '$lib/api';
  import { locale, t, type Key } from '$lib/i18n';
  import { liveCount, liveRunning } from '$lib/live';
  import { initPrefs, prefs } from '$lib/prefs';
  import { syncReminders } from '$lib/reminders';
  import BadgeIcon from '$lib/BadgeIcon.svelte';
  import DailyQuote from '$lib/DailyQuote.svelte';
  import { badgeName, newBadges, type Badge } from '$lib/badges';

  const tabs = [
    { href: '/', key: 'tab.count', icon: 'count' },
    { href: '/stats', key: 'tab.stats', icon: 'stats' },
    { href: '/voice', key: 'tab.voice', icon: 'voice' },
    { href: '/settings', key: 'tab.settings', icon: 'settings' },
  ] as const;

  /** Sections of the side menu; the counter is the four tabs above. */
  const sections: { href: string; key: Key; icon: string; soon?: boolean }[] = [
    { href: '/', key: 'menu.counter', icon: 'count' },
    { href: '/gongyo', key: 'menu.gongyo', icon: 'scroll' },
    { href: '/badges', key: 'menu.badges', icon: 'badge' },
    { href: '/reminders', key: 'menu.reminders', icon: 'alarm' },
    { href: '/bell', key: 'menu.bell', icon: 'bell' },
    { href: '/dictionary', key: 'menu.dictionary', icon: 'book' },
    { href: '/social', key: 'menu.social', icon: 'people', soon: true },
    { href: '/donate', key: 'menu.donate', icon: 'heart' },
    { href: '/help', key: 'menu.help', icon: 'help' },
    { href: '/privacy', key: 'menu.privacy', icon: 'shield' },
  ];

  $: path = $page.url.pathname.replace(/\/$/, '') || '/';
  $: inCounter = tabs.some((tab) => tab.href === path);
  $: section = inCounter ? sections[0] : sections.find((s) => s.href === path);

  let version = '';

  // new badges: a banner at the top, closed by a tap or after a while
  const BANNER_MS = 7000;
  let banner: Badge[] = [];
  let bannerTimer: ReturnType<typeof setTimeout> | null = null;
  $: if ($newBadges.length && !banner.length) {
    banner = $newBadges;
    newBadges.set([]);
    if (bannerTimer) clearTimeout(bannerTimer);
    bannerTimer = setTimeout(closeBanner, BANNER_MS);
  }
  function closeBanner() {
    banner = [];
    if (bannerTimer) clearTimeout(bannerTimer);
    bannerTimer = null;
  }
  let prefsReady = false;
  // the quote of the day, every time the app starts (the language comes
  // with the preferences, so it waits for them)
  let showQuote = true;
  // the Android side keeps its own copy of the reminders (and their text)
  $: if (prefsReady) syncReminders($prefs), $t;

  // ---------- side menu ----------
  // Opens with the button or with a swipe to the right that starts on the
  // left half of the screen (not right at the edge: with gesture
  // navigation Android keeps the edge for "back"); closes with a swipe to
  // the left, a tap outside, Escape or by choosing a section.
  const DRAWER_W = 300;
  const SWIPE_START = 12; // px before a move counts as horizontal
  const SWIPE_ZONE = 0.5; // share of the screen width where an opening swipe may start
  let open = false;
  let drag: number | null = null; // drawer offset while following the finger, 0 = open
  let sx = 0;
  let sy = 0;
  let st = 0;
  let tracking = false;
  let horizontal = false;

  function drawerWidth() {
    return Math.min(DRAWER_W, window.innerWidth * 0.84);
  }

  function noSwipe(el: EventTarget | null): boolean {
    return el instanceof Element && !!el.closest('input, textarea, select, [data-noswipe]');
  }

  function onTouchStart(e: TouchEvent) {
    tracking = false;
    horizontal = false;
    if (e.touches.length !== 1 || noSwipe(e.target)) return;
    const p = e.touches[0];
    sx = p.clientX;
    sy = p.clientY;
    st = performance.now();
    tracking = open || sx < window.innerWidth * SWIPE_ZONE;
  }

  function onTouchMove(e: TouchEvent) {
    if (!tracking) return;
    const p = e.touches[0];
    const dx = p.clientX - sx;
    const dy = p.clientY - sy;
    if (!horizontal) {
      if (Math.abs(dy) > SWIPE_START && Math.abs(dy) > Math.abs(dx)) {
        tracking = false; // a vertical scroll
        return;
      }
      if (Math.abs(dx) < SWIPE_START || (open ? dx > 0 : dx < 0)) return;
      horizontal = true;
    }
    const w = drawerWidth();
    drag = open ? Math.min(0, dx) : Math.min(0, -w + dx);
  }

  function onTouchEnd(e: TouchEvent) {
    if (!tracking) return;
    tracking = false;
    if (!horizontal || drag === null) return;
    const w = drawerWidth();
    const dx = e.changedTouches[0].clientX - sx;
    const fast = Math.abs(dx) / Math.max(1, performance.now() - st) > 0.5; // px/ms: a flick
    open = open ? !(drag < -w * 0.35 || (fast && dx < 0)) : drag > -w * 0.65 || (fast && dx > 0);
    drag = null;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) open = false;
  }

  /** Inline style of the drawer and the scrim while a finger moves them. */
  $: drawerStyle = drag === null ? '' : `transform: translateX(${drag}px); transition: none;`;
  $: scrimStyle = drag === null ? '' : `opacity: ${1 + drag / DRAWER_W}; transition: none;`;

  /** The splash in app.html stays at least this long, and at most MAX. */
  const SPLASH_MIN_MS = 700;
  const SPLASH_MAX_MS = 8000;

  function hideSplash() {
    const el = document.getElementById('splash');
    if (!el || el.classList.contains('hide')) return;
    const wait = Math.max(0, SPLASH_MIN_MS - performance.now());
    setTimeout(() => {
      el.classList.add('hide');
      setTimeout(() => el.remove(), 400);
    }, wait);
  }

  onMount(async () => {
    const safety = setTimeout(hideSplash, SPLASH_MAX_MS);
    getVersion().then((v) => (version = v), () => {});
    await initPrefs();
    prefsReady = true;
    // First start (no voice recordings yet): open the voice page, so the
    // app learns the user's voice before counting.
    try {
      const profile = await api.profile();
      if (profile.takes.length === 0 && path === '/') await goto('/voice', { replaceState: true });
    } catch {
      /* keep the counter page */
    }
    clearTimeout(safety);
    hideSplash();
  });
</script>

<svelte:window on:keydown={onKey} />

<div class="bg" aria-hidden="true"></div>

<!-- the swipe that opens the menu: touch only, the button does the same -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="app"
  inert={showQuote && prefsReady}
  on:touchstart|passive={onTouchStart}
  on:touchmove|passive={onTouchMove}
  on:touchend={onTouchEnd}
  on:touchcancel={() => {
    tracking = false;
    drag = null;
  }}
>
  <header class="topbar">
    <button class="menu-btn" on:click={() => (open = true)} aria-label={$t('menu.open')} aria-expanded={open} aria-controls="side-menu">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
    </button>
    <span class="section">{section ? $t(section.key) : ''}</span>
    {#if $liveRunning && path !== '/'}
      <a class="live" href="/" aria-label={$t('menu.live')} title={$t('menu.live')}>
        <span class="dot" aria-hidden="true"></span>{fmtNum($liveCount, $locale)}
      </a>
    {/if}
  </header>

  <main>
    <slot />
  </main>

  {#if inCounter}
    <nav aria-label="Menu">
      {#each tabs as tab}
        <a href={tab.href} class:active={path === tab.href} aria-current={path === tab.href ? 'page' : undefined}>
          <svg viewBox="0 0 24 24" aria-hidden="true">
            {#if tab.icon === 'count'}
              <circle cx="12" cy="12" r="8.5" />
              <circle cx="12" cy="12" r="3" />
            {:else if tab.icon === 'stats'}
              <path d="M5 20V11M12 20V5M19 20v-6" />
            {:else if tab.icon === 'voice'}
              <rect x="9" y="3.5" width="6" height="11" rx="3" />
              <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v2.5" />
            {:else}
              <circle cx="12" cy="12" r="3" />
              <path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8" />
            {/if}
          </svg>
          <span>{$t(tab.key)}</span>
        </a>
      {/each}
    </nav>
  {/if}
</div>

<div role="status">
{#if banner.length}
  <a class="badge-banner glass" href="/badges" on:click={closeBanner}>
    <BadgeIcon badge={banner[banner.length - 1]} size={46} />
    <span>
      <strong>{banner.length > 1 ? $t('badge.newMany', { n: banner.length }) : $t('badge.new')}</strong>
      <span class="b-names">{banner.map((b) => badgeName(b, $t, $locale)).join(' · ')}</span>
    </span>
  </a>
{/if}
</div>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="scrim"
  class:open={open || drag !== null}
  style={scrimStyle}
  on:click={() => (open = false)}
  on:touchstart|passive={onTouchStart}
  on:touchmove|passive={onTouchMove}
  on:touchend={onTouchEnd}
></div>

<aside
  id="side-menu"
  class="drawer"
  class:open
  style={drawerStyle}
  inert={!open && drag === null}
  aria-label="Menu"
  on:touchstart|passive={onTouchStart}
  on:touchmove|passive={onTouchMove}
  on:touchend={onTouchEnd}
>
  <div class="drawer-head">
    <img src="/app-icon.png" alt="" />
    <div>
      <div class="name">Daimoku Counter</div>
      {#if version}<div class="ver">v{version}</div>{/if}
    </div>
    <button class="close" on:click={() => (open = false)} aria-label={$t('menu.close')}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </div>
  <ul>
    {#each sections as s, i}
      {@const active = section === s}
      <li>
        <a href={s.href} class:active aria-current={active ? 'page' : undefined} on:click={() => (open = false)}>
          <span class="num" aria-hidden="true">{i + 1}</span>
          <svg viewBox="0 0 24 24" aria-hidden="true">
            {#if s.icon === 'count'}
              <circle cx="12" cy="12" r="8.5" />
              <circle cx="12" cy="12" r="3" />
            {:else if s.icon === 'scroll'}
              <path d="M7 3.5h10a2 2 0 0 1 2 2V20l-2.5-1.5L14 20l-2-1.5L10 20l-2.5-1.5L5 20V5.5a2 2 0 0 1 2-2z" />
              <path d="M8.5 8h7M8.5 11.5h7M8.5 15h4" />
            {:else if s.icon === 'badge'}
              <circle cx="12" cy="9.5" r="5.5" />
              <path d="M8.5 14l-1.5 7 5-2.5 5 2.5-1.5-7" />
            {:else if s.icon === 'alarm'}
              <circle cx="12" cy="13" r="7.5" />
              <path d="M12 9.5V13l2.5 2M5 4.5L3 6.5M19 4.5l2 2" />
            {:else if s.icon === 'bell'}
              <path d="M5 16.5h14l-1.6-2.4V10a5.4 5.4 0 0 0-10.8 0v4.1z" />
              <path d="M10 19.2a2.2 2.2 0 0 0 4 0M12 4.6V3" />
            {:else if s.icon === 'book'}
              <path d="M5 5.5A2.5 2.5 0 0 1 7.5 3H19v15H7.5A2.5 2.5 0 0 0 5 20.5zM5 20.5A2.5 2.5 0 0 0 7.5 23H19v-5" />
            {:else if s.icon === 'people'}
              <circle cx="9" cy="8" r="3" />
              <path d="M3.5 19.5a5.5 5.5 0 0 1 11 0" />
              <circle cx="17" cy="9" r="2.4" />
              <path d="M15.6 14.3A4.5 4.5 0 0 1 20.5 19" />
            {:else if s.icon === 'heart'}
              <path d="M12 20s-7.5-4.6-7.5-10.2A4.1 4.1 0 0 1 12 7.3a4.1 4.1 0 0 1 7.5 2.5C19.5 15.4 12 20 12 20z" />
            {:else if s.icon === 'help'}
              <circle cx="12" cy="12" r="9" />
              <path d="M9.6 9.4a2.5 2.5 0 1 1 3.4 2.4c-.6.3-1 .8-1 1.5v.6M12 17.2v.1" />
            {:else}
              <path d="M12 3l7 3v5.2c0 4.4-3 8.1-7 9.8-4-1.7-7-5.4-7-9.8V6z" />
              <path d="M9 12l2.2 2.2L15.5 10" />
            {/if}
          </svg>
          <span class="label">{$t(s.key)}</span>
          {#if s.soon}
            <span class="badge">{$t('menu.soon')}</span>
          {:else if s.href === '/' && $liveRunning}
            <span class="dot" aria-hidden="true"></span>
          {/if}
        </a>
      </li>
    {/each}
  </ul>
</aside>


{#if showQuote && prefsReady}
  <DailyQuote onenter={() => (showQuote = false)} />
{/if}

<style>
  /* ---------- theme tokens ---------- */
  :global(:root) {
    color-scheme: dark;
    --radius: 18px;
    --green: #6fd39a;
    --red: #e5675f;
    /* dark (default) */
    --bg: #0c0d0f;
    --bg-image: url('/bg-dark.jpg');
    --bg-overlay: linear-gradient(180deg, rgba(8, 9, 11, 0.3) 0%, rgba(8, 9, 11, 0.62) 45%, rgba(8, 9, 11, 0.9) 100%);
    --surface: rgba(24, 25, 28, 0.72);
    --surface-2: rgba(255, 255, 255, 0.08);
    --surface-solid: #1a1b1e;
    --line: rgba(255, 255, 255, 0.1);
    --text: #eeeeec;
    --muted: #a6a9b0;
    --faint: #737780;
    --nav: rgba(14, 15, 17, 0.82);
    --shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    --title-shadow: 0 1px 10px rgba(0, 0, 0, 0.6);
  }
  :global(:root[data-theme='light']) {
    color-scheme: light;
    --green: #23965a;
    --red: #c8453c;
    --bg: #eef2ea;
    --bg-image: url('/bg-light.jpg');
    --bg-overlay: linear-gradient(180deg, rgba(240, 244, 236, 0.62) 0%, rgba(240, 244, 236, 0.3) 22%, rgba(240, 244, 236, 0.5) 55%, rgba(240, 244, 236, 0.9) 100%);
    --title-shadow: none;
    --surface: rgba(255, 255, 255, 0.76);
    --surface-2: rgba(20, 40, 20, 0.06);
    --surface-solid: #fbfcf9;
    --line: rgba(20, 40, 20, 0.1);
    --text: #172117;
    --muted: #4c5a4d;
    --faint: #7b877c;
    --nav: rgba(250, 252, 248, 0.86);
    --shadow: 0 10px 30px rgba(30, 50, 30, 0.12);
  }

  /* backgrounds (src/lib/background.ts): the picture of each theme, in
     its light and dark version; a picture of the user's own */
  :global(:root[data-bg='sutra']) {
    --bg-image: url('/backgrounds/sutra-dark.jpg');
  }
  :global(:root[data-theme='light'][data-bg='sutra']) {
    --bg-image: url('/backgrounds/sutra-light.jpg');
  }
  :global(:root[data-bg='buddha']) {
    --bg-image: url('/backgrounds/buddha-dark.jpg');
  }
  :global(:root[data-theme='light'][data-bg='buddha']) {
    --bg-image: url('/backgrounds/buddha-light.jpg');
  }
  :global(:root[data-bg='temple']) {
    --bg-image: url('/backgrounds/temple-dark.jpg');
  }
  :global(:root[data-theme='light'][data-bg='temple']) {
    --bg-image: url('/backgrounds/temple-light.jpg');
  }
  :global(:root[data-bg='nature']) {
    --bg-image: url('/backgrounds/nature-dark.jpg');
  }
  :global(:root[data-theme='light'][data-bg='nature']) {
    --bg-image: url('/backgrounds/nature-light.jpg');
  }
  :global(:root[data-bg='sakura']) {
    --bg-image: url('/backgrounds/sakura-dark.jpg');
  }
  :global(:root[data-theme='light'][data-bg='sakura']) {
    --bg-image: url('/backgrounds/sakura-light.jpg');
  }
  :global(:root[data-bg='custom']),
  :global(:root[data-theme='light'][data-bg='custom']) {
    --bg-image: var(--bg-custom, none);
  }

  /* accent colours: [theme] x [accent] */
  :global(:root[data-accent='lotus']),
  :global(:root:not([data-accent])) {
    --accent: #5cc9a7;
    --accent-ink: #062016;
    --accent-soft: rgba(92, 201, 167, 0.16);
    --mark: #2f9e80;
    --mark-current: #5cc9a7;
  }
  :global(:root[data-accent='gold']) {
    --accent: #f2b544;
    --accent-ink: #1b1406;
    --accent-soft: rgba(242, 181, 68, 0.16);
    --mark: #b98228;
    --mark-current: #f2b544;
  }
  :global(:root[data-accent='sakura']) {
    --accent: #f29bbd;
    --accent-ink: #2a0a17;
    --accent-soft: rgba(242, 155, 189, 0.16);
    --mark: #c4668a;
    --mark-current: #f29bbd;
  }
  :global(:root[data-accent='indigo']) {
    --accent: #93aaf2;
    --accent-ink: #0b1330;
    --accent-soft: rgba(147, 170, 242, 0.16);
    --mark: #6f86d0;
    --mark-current: #93aaf2;
  }
  :global(:root[data-theme='light'][data-accent='lotus']),
  :global(:root[data-theme='light']:not([data-accent])) {
    --accent: #1f8a70;
    --accent-ink: #ffffff;
    --accent-soft: rgba(31, 138, 112, 0.13);
    --mark: #1f8a70;
    --mark-current: #146b56;
  }
  :global(:root[data-theme='light'][data-accent='gold']) {
    --accent: #b07a1a;
    --accent-ink: #ffffff;
    --accent-soft: rgba(176, 122, 26, 0.13);
    --mark: #b07a1a;
    --mark-current: #86590f;
  }
  :global(:root[data-theme='light'][data-accent='sakura']) {
    --accent: #c0507a;
    --accent-ink: #ffffff;
    --accent-soft: rgba(192, 80, 122, 0.13);
    --mark: #c0507a;
    --mark-current: #963a5d;
  }
  :global(:root[data-theme='light'][data-accent='indigo']) {
    --accent: #3c5aa8;
    --accent-ink: #ffffff;
    --accent-soft: rgba(60, 90, 168, 0.13);
    --mark: #3c5aa8;
    --mark-current: #2b4383;
  }

  /* ---------- base ---------- */
  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    background: var(--bg);
    /* no browser "swipe back" under the swipe that opens the menu */
    overscroll-behavior: none;
  }
  :global(body) {
    color: var(--text);
    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, 'Hiragino Sans', 'Noto Sans JP', sans-serif;
    -webkit-font-smoothing: antialiased;
    -webkit-tap-highlight-color: transparent;
    overflow: hidden;
    overscroll-behavior: none;
    transition: background-color 0.3s;
  }
  :global(button) {
    font: inherit;
    color: inherit;
    touch-action: manipulation;
  }
  :global(*) {
    box-sizing: border-box;
  }
  /* frosted cards over the background picture */
  :global(.glass) {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    -webkit-backdrop-filter: blur(16px) saturate(1.1);
    backdrop-filter: blur(16px) saturate(1.1);
    box-shadow: var(--shadow);
  }

  /* on/off switch: <input type="checkbox" role="switch" class="switch"> */
  :global(input.switch) {
    appearance: none;
    -webkit-appearance: none;
    flex-shrink: 0;
    width: 46px;
    height: 28px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--line);
    position: relative;
    cursor: pointer;
    transition: background 0.2s;
  }
  :global(input.switch::after) {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--muted);
    transition: transform 0.2s, background 0.2s;
  }
  :global(input.switch:checked) {
    background: var(--accent);
    border-color: var(--accent);
  }
  :global(input.switch:checked::after) {
    transform: translateX(18px);
    background: var(--accent-ink);
  }
  :global(input.switch:disabled) {
    opacity: 0.5;
    cursor: default;
  }

  .bg {
    position: fixed;
    inset: 0;
    z-index: 0;
    background: var(--bg-overlay), var(--bg-image) center 30% / cover no-repeat, var(--bg);
    transition: background 0.3s;
  }

  .app {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    height: 100vh;
    height: 100dvh;
    padding-top: env(safe-area-inset-top);
  }

  main {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    display: flex;
    flex-direction: column;
    -webkit-overflow-scrolling: touch;
  }

  nav {
    flex-shrink: 0;
    display: flex;
    justify-content: space-around;
    background: var(--nav);
    -webkit-backdrop-filter: blur(16px);
    backdrop-filter: blur(16px);
    border-top: 1px solid var(--line);
    padding: 6px 6px calc(6px + env(safe-area-inset-bottom));
  }
  nav a {
    flex: 1;
    max-width: 120px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 6px 0;
    border-radius: 12px;
    color: var(--faint);
    text-decoration: none;
    font-size: 0.68rem;
    font-weight: 500;
    text-align: center;
    transition: color 0.15s;
  }
  nav a svg {
    width: 24px;
    height: 24px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  nav a.active {
    color: var(--accent);
  }
  /* ---------- new badge banner ---------- */
  .badge-banner {
    position: fixed;
    z-index: 30;
    top: calc(10px + env(safe-area-inset-top));
    left: 50%;
    transform: translateX(-50%);
    width: min(420px, calc(100vw - 24px));
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px 10px 12px;
    color: var(--text);
    text-decoration: none;
    animation: banner-in 0.45s cubic-bezier(0.2, 1.4, 0.4, 1);
  }
  .badge-banner > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .b-names {
    color: var(--muted);
    font-size: 0.88rem;
  }
  @keyframes banner-in {
    from {
      transform: translate(-50%, -120%);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .badge-banner {
      animation: none;
    }
  }

  /* ---------- top bar and side menu ---------- */
  .topbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 48px;
    padding: 4px 12px 0 6px;
  }
  .menu-btn,
  .close {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
  }
  .menu-btn:active,
  .close:active {
    background: var(--surface-2);
  }
  .menu-btn svg,
  .close svg {
    width: 24px;
    height: 24px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .section {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: var(--title-shadow);
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 12px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid var(--line);
    color: var(--text);
    text-decoration: none;
    font-weight: 600;
    font-size: 0.9rem;
    font-variant-numeric: tabular-nums;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: live-dot 1.6s ease-in-out infinite;
  }
  @keyframes live-dot {
    50% {
      opacity: 0.35;
    }
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(0, 0, 0, 0.45);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.25s ease;
  }
  .scrim.open {
    opacity: 1;
    pointer-events: auto;
  }
  .drawer {
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 21;
    width: min(300px, 84vw);
    display: flex;
    flex-direction: column;
    padding: calc(10px + env(safe-area-inset-top)) 10px calc(14px + env(safe-area-inset-bottom));
    background: var(--surface-solid);
    border-right: 1px solid var(--line);
    box-shadow: var(--shadow);
    transform: translateX(-100%);
    transition: transform 0.25s ease;
    overflow-y: auto;
  }
  .drawer.open {
    transform: translateX(0);
  }
  .drawer-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 4px 14px 8px;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--line);
  }
  .drawer-head img {
    width: 44px;
    height: 44px;
    border-radius: 12px;
  }
  .drawer-head > div {
    flex: 1;
    min-width: 0;
  }
  .drawer-head .name {
    font-weight: 700;
  }
  .drawer-head .ver {
    color: var(--faint);
    font-size: 0.8rem;
  }
  .drawer ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .drawer li a {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px;
    border-radius: 14px;
    color: var(--text);
    text-decoration: none;
    font-weight: 500;
  }
  .drawer li a:active {
    background: var(--surface-2);
  }
  .drawer li a.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .drawer .num {
    width: 14px;
    color: var(--faint);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .drawer li a svg {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .drawer .label {
    flex: 1;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 600;
  }
  @media (prefers-reduced-motion: reduce) {
    .drawer,
    .scrim {
      transition: none;
    }
    .dot {
      animation: none;
    }
  }
</style>
