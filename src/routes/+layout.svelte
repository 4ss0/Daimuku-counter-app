<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { t } from '$lib/i18n';
  import { initPrefs } from '$lib/prefs';

  const tabs = [
    { href: '/', key: 'tab.count', icon: 'count' },
    { href: '/stats', key: 'tab.stats', icon: 'stats' },
    { href: '/voce', key: 'tab.voice', icon: 'voice' },
    { href: '/settings', key: 'tab.settings', icon: 'settings' },
  ] as const;

  $: path = $page.url.pathname.replace(/\/$/, '') || '/';

  onMount(initPrefs);
</script>

<div class="bg" aria-hidden="true"></div>

<div class="app">
  <main>
    <slot />
  </main>

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
</div>

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
</style>
