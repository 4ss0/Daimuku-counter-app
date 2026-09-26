<script lang="ts">
  import { goto } from '$app/navigation';
</script>

<div class="welcome">
  <h1>Daimuku Counter</h1>
  <p>Application for counting Daimoku recited in front of the Gohonzon.</p>

  <div class="actions">
    <button on:click={() => goto('/samples')}>Add samples</button>
    <button on:click={() => goto('/test')}>Count Daimoku</button>
  </div>
</div>

<style>
  .welcome {
    flex: 1;                /* occupa tutto lo spazio del main */
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 1rem;
  }

  h1 {
    margin: 0;
    font-size: 2.5rem;
  }

  p {
    margin: 0;
    opacity: 0.75;
    max-width: 32rem;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 8px;
    background-color: #3b6ea5;
    color: #fff;
    font-size: 1rem;
    cursor: pointer;
  }

  button:hover {
    background-color: #4a7fb8;
  }
</style>