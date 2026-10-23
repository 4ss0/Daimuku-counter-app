<script lang="ts">
  import { tick } from 'svelte';
  import { CATS, TERMS, haystack, view, type Cat } from '$lib/dictionary';
  import { lang, t, type Key } from '$lib/i18n';

  let query = '';
  let cat: Cat | 'all' = 'all';
  let openId: string | null = null;

  /** Lower case, without accents and with plain apostrophes, for searching. */
  function norm(s: string): string {
    return s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[’‘`]/g, "'");
  }

  const index = TERMS.map((e) => ({ e, text: norm(haystack(e)) }));
  const byId = new Map(TERMS.map((e) => [e.id, e]));

  $: words = norm(query).split(/\s+/).filter(Boolean);
  $: list = index
    .filter(({ e, text }) => (cat === 'all' || e.cat === cat) && words.every((w) => text.includes(w)))
    .map(({ e }) => ({ e, v: view(e, $lang) }))
    .sort((a, b) => ($lang === 'ja' ? a.e.kana.localeCompare(b.e.kana, 'ja') : a.v.title.localeCompare(b.v.title, $lang)));

  function toggle(id: string) {
    openId = openId === id ? null : id;
  }

  /** "See also": shows that entry, even if the search or the filter hid it. */
  async function jump(id: string) {
    const e = byId.get(id);
    if (!e) return;
    if (!list.some((x) => x.e.id === id)) {
      query = '';
      cat = 'all';
    }
    openId = id;
    await tick();
    document.getElementById(`term-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }
</script>

<div class="page">
  <div class="search glass">
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6.5" /><path d="M16 16l4.5 4.5" /></svg>
    <input type="search" bind:value={query} placeholder={$t('dict.search')} aria-label={$t('dict.search')} autocomplete="off" />
    {#if query}
      <button class="clear" on:click={() => (query = '')} aria-label={$t('dict.clear')}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7L7 17" /></svg>
      </button>
    {/if}
  </div>

  <div class="cats" role="tablist">
    {#each ['all', ...CATS] as const as c}
      <button role="tab" aria-selected={cat === c} class:on={cat === c} on:click={() => (cat = c)}>
        {$t(c === 'all' ? 'dict.all' : (`dict.cat.${c}` as Key))}
      </button>
    {/each}
  </div>

  <div class="n">{$t('dict.count', { n: list.length })}</div>

  {#each list as { e, v } (e.id)}
    <section class="term glass" id="term-{e.id}">
      <button class="head" on:click={() => toggle(e.id)} aria-expanded={openId === e.id}>
        <span class="titles">
          <span class="title" lang={$lang === 'ja' ? 'ja' : undefined}>{v.title}</span>
          <span class="sub">{v.sub}{#if v.jp}{' · '}<span class="jp" lang="ja">{v.jp}</span>{/if}</span>
        </span>
        <svg class="chev" class:open={openId === e.id} viewBox="0 0 24 24" aria-hidden="true"><path d="M8 10l4 4 4-4" /></svg>
      </button>
      {#if openId === e.id}
        <div class="body">
          <p>{v.text}</p>
          {#if e.see?.length}
            <div class="see">
              <span class="see-label">{$t('dict.see')}</span>
              {#each e.see as id}
                {@const other = byId.get(id)}
                {#if other}
                  <button class="chip" on:click={() => jump(id)}>{view(other, $lang).title}</button>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </section>
  {:else}
    <p class="none">{$t('dict.none')}</p>
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
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px 4px 14px;
    border-radius: 999px;
  }
  .search svg {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    fill: none;
    stroke: var(--muted);
    stroke-width: 2;
    stroke-linecap: round;
  }
  .search input {
    flex: 1;
    min-width: 0;
    padding: 9px 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 1rem;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--surface-2);
    cursor: pointer;
  }
  .clear svg {
    width: 16px;
    height: 16px;
  }
  .cats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .cats button {
    padding: 7px 13px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--muted);
    font-size: 0.88rem;
    font-weight: 500;
    cursor: pointer;
  }
  .cats button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }
  .n {
    color: var(--faint);
    font-size: 0.8rem;
    margin: 0 4px;
  }
  .term {
    padding: 0;
    overflow: hidden;
    scroll-margin-top: 12px;
  }
  .head {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px 12px 16px;
    border: none;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    font-weight: 600;
    font-size: 1.02rem;
  }
  .sub {
    color: var(--muted);
    font-size: 0.85rem;
  }
  .jp {
    white-space: nowrap;
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
    padding: 0 16px 14px;
  }
  .body p {
    margin: 0 0 10px;
    line-height: 1.55;
    user-select: text;
    -webkit-user-select: text;
  }
  .see {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .see-label {
    color: var(--faint);
    font-size: 0.8rem;
    margin-right: 2px;
  }
  .chip {
    padding: 5px 11px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 0.84rem;
    cursor: pointer;
  }
  .none {
    text-align: center;
    color: var(--muted);
    margin: 24px 0;
  }
</style>
