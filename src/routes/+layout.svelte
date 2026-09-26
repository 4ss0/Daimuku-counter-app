<script lang="ts">
  import { page } from '$app/stores';
</script>

<div class="app">
  <nav>
    <a href="/" class:active={$page.url.pathname === '/'}>Home</a>
    <a href="/samples" class:active={$page.url.pathname === '/samples'}>Samples</a>
    <a href="/test" class:active={$page.url.pathname === '/test'}>Test</a>
    <a href="/stats" class:active={$page.url.pathname === '/stats'}>Stats</a>
  </nav>

  <main>
    <slot />
  </main>
</div>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
  }

  :global(body) {
    background-color: #1a1a1a;
    color: #eaeaea;
    font-family: system-ui, -apple-system, sans-serif;
    overflow: hidden; /* no scrollbar on body itself */
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  nav {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background-color: #242424;
    border-bottom: 1px solid #333;
    flex-shrink: 0;
  }

  nav a {
    padding: 0.5rem 0.9rem;
    border-radius: 6px;
    color: #b8b8b8;
    text-decoration: none;
    font-size: 0.9rem;
  }

  nav a:hover {
    background-color: #333;
    color: #fff;
  }

  nav a.active {
    background-color: #3b6ea5;
    color: #fff;
  }

  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: auto;
    padding: 2rem;
  }
</style>