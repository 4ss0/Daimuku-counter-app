<script lang="ts">
  // SvelteKit requires a script block even if empty for typing purposes.
</script>

<main class="welcome">
  <h1>Daimuku Counter</h1>
  <p>Application for counting Daimoku recited in front of the Gohonzon.</p>
</main>

<style>
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    text-align: center;
    padding: 2rem;
  }

  h1 {
    margin: 0 0 1rem;
    font-size: 2.5rem;
  }

  p {
    margin: 0;
    opacity: 0.75;
    max-width: 32rem;
    line-height: 1.5;
  }
</style>