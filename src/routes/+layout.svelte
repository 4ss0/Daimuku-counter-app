<script lang="ts">
  import { page } from '$app/stores';

  const tabs = [
    { href: '/', label: 'Conta', icon: 'count' },
    { href: '/stats', label: 'Statistiche', icon: 'stats' },
    { href: '/voce', label: 'La tua voce', icon: 'voice' },
  ];

  $: path = $page.url.pathname.replace(/\/$/, '') || '/';
</script>

<div class="app">
  <main>
    <slot />
  </main>

  <nav aria-label="Sezioni">
    {#each tabs as t}
      <a href={t.href} class:active={path === t.href} aria-current={path === t.href ? 'page' : undefined}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {#if t.icon === 'count'}
            <circle cx="12" cy="12" r="8.5" />
            <circle cx="12" cy="12" r="3" />
          {:else if t.icon === 'stats'}
            <path d="M5 20V11M12 20V5M19 20v-6" />
          {:else}
            <rect x="9" y="3.5" width="6" height="11" rx="3" />
            <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v2.5" />
          {/if}
        </svg>
        <span>{t.label}</span>
      </a>
    {/each}
  </nav>
</div>

<style>
  :global(:root) {
    --bg: #0e0f13;
    --surface: #171a21;
    --surface-2: #1f2330;
    --line: #2a2f3c;
    --text: #eceff4;
    --muted: #8b93a7;
    --faint: #5b6275;
    --gold: #f2b544;
    --gold-soft: rgba(242, 181, 68, 0.14);
    --gold-mark: #b98228;
    --green: #6fd39a;
    --red: #e5675f;
    --radius: 18px;
    color-scheme: dark;
  }
  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    background: var(--bg);
  }
  :global(body) {
    color: var(--text);
    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    -webkit-tap-highlight-color: transparent;
    overflow: hidden;
    overscroll-behavior: none;
  }
  :global(button) {
    font: inherit;
    color: inherit;
    touch-action: manipulation;
  }
  :global(*) {
    box-sizing: border-box;
  }

  .app {
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
    background: rgba(23, 26, 33, 0.96);
    border-top: 1px solid var(--line);
    padding: 6px 8px calc(6px + env(safe-area-inset-bottom));
  }
  nav a {
    flex: 1;
    max-width: 140px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 6px 0;
    border-radius: 12px;
    color: var(--faint);
    text-decoration: none;
    font-size: 0.72rem;
    font-weight: 500;
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
    color: var(--gold);
  }
</style>
