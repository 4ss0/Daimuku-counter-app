<script lang="ts">
  import { HELP, blocks } from '$lib/help';
  import { lang } from '$lib/i18n';

  let openId: string | null = HELP[0].id;
</script>

<div class="page">
  {#each HELP as s (s.id)}
    {@const [title, body] = s.text[$lang]}
    <section class="glass">
      <button class="head" on:click={() => (openId = openId === s.id ? null : s.id)} aria-expanded={openId === s.id}>
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          {#if s.icon === 'voice'}
            <rect x="9" y="3.5" width="6" height="11" rx="3" />
            <path d="M5.5 11.5a6.5 6.5 0 0 0 13 0M12 18v2.5" />
          {:else if s.icon === 'count'}
            <circle cx="12" cy="12" r="8.5" />
            <circle cx="12" cy="12" r="3" />
          {:else if s.icon === 'coin'}
            <circle cx="12" cy="12" r="8.5" />
            <path d="M12 7.5v9M9.5 10a2.5 2 0 0 1 5 0M9.5 14a2.5 2 0 0 0 5 0" />
          {:else if s.icon === 'moon'}
            <path d="M19.5 14.5A8 8 0 0 1 9.5 4.5a8 8 0 1 0 10 10z" />
          {:else if s.icon === 'tips'}
            <path d="M9 18h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.4 1 1.1 1 1.9V16h5v-.2c0-.8.4-1.5 1-1.9A6 6 0 0 0 12 3z" />
          {:else if s.icon === 'stats'}
            <path d="M5 20V11M12 20V5M19 20v-6" />
          {:else if s.icon === 'goal'}
            <circle cx="12" cy="12" r="8.5" /><circle cx="12" cy="12" r="4.5" /><circle cx="12" cy="12" r="1" />
          {:else if s.icon === 'badge'}
            <circle cx="12" cy="9.5" r="5.5" /><path d="M8.5 14l-1.5 7 5-2.5 5 2.5-1.5-7" />
          {:else if s.icon === 'bell'}
            <path d="M5 16.5h14l-1.6-2.4V10a5.4 5.4 0 0 0-10.8 0v4.1z" /><path d="M10 19.2a2.2 2.2 0 0 0 4 0M12 4.6V3" />
          {:else if s.icon === 'backup'}
            <path d="M12 15V4M8 8l4-4 4 4M5 14v4.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V14" />
          {:else if s.icon === 'menu'}
            <path d="M4 7h16M4 12h16M4 17h16" />
          {:else}
            <circle cx="12" cy="12" r="9" />
            <path d="M9.6 9.4a2.5 2.5 0 1 1 3.4 2.4c-.6.3-1 .8-1 1.5v.6M12 17.2v.1" />
          {/if}
        </svg>
        <span class="title">{title}</span>
        <svg class="chev" class:open={openId === s.id} viewBox="0 0 24 24" aria-hidden="true"><path d="M8 10l4 4 4-4" /></svg>
      </button>
      {#if openId === s.id}
        <div class="body">
          {#each blocks(body) as b}
            {#if 'list' in b}
              <ul>
                {#each b.list as item}<li>{item}</li>{/each}
              </ul>
            {:else}
              <p>{b.p}</p>
            {/if}
          {/each}
        </div>
      {/if}
    </section>
  {/each}
</div>

<style>
  .page {
    width: 100%;
    max-width: 560px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  section {
    overflow: hidden;
  }
  .head {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 14px 14px 16px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .icon {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .title {
    flex: 1;
    font-weight: 600;
  }
  .chev {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    fill: none;
    stroke: var(--faint);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.2s;
  }
  .chev.open {
    transform: rotate(180deg);
  }
  .body {
    padding: 0 18px 14px 50px;
    line-height: 1.55;
    user-select: text;
    -webkit-user-select: text;
  }
  .body p {
    margin: 0 0 10px;
  }
  .body ul {
    margin: 0 0 10px;
    padding-left: 18px;
  }
  .body li {
    margin-bottom: 6px;
  }
  .body > :last-child {
    margin-bottom: 0;
  }
</style>
