<script lang="ts">
  // "Goals" in the menu: what the user chants for. Each goal has a slider
  // for how far along it is; it is closed by choosing why: reached, or a
  // better way was found. Closed goals stay below, to look back on.
  import { locale, t, type Key } from '$lib/i18n';
  import { INTENT_REASONS, MAX_INTENT_TEXT, prefs, type Intent, type IntentReason } from '$lib/prefs';
  import { addIntent, closeIntent, deleteIntent, editIntent, reopenIntent, setProgress } from '$lib/intents';

  let draft = '';
  /** The goal being edited, closed, or about to be deleted. */
  let editing: string | null = null;
  let editText = '';
  let closing: string | null = null;
  let reason: IntentReason | null = null;
  let deleting: string | null = null;
  let showClosed = false;
  /** Slider positions while a finger moves them (saved when let go). */
  let moving: Record<string, number> = {};

  $: open = $prefs.intents.filter((g) => !g.closed);
  $: closed = $prefs.intents.filter((g) => g.closed).sort((a, b) => (b.closed ?? '').localeCompare(a.closed ?? ''));

  function day(iso: string | undefined): string {
    return iso ? new Date(iso).toLocaleDateString($locale, { day: 'numeric', month: 'long', year: 'numeric' }) : '';
  }

  function add() {
    if (!draft.trim()) return;
    addIntent(draft);
    draft = '';
  }

  function startEdit(g: Intent) {
    editing = g.id;
    editText = g.text;
    closing = null;
    deleting = null;
  }

  function saveEdit() {
    if (editing && editText.trim()) editIntent(editing, editText);
    editing = null;
  }

  function remove(id: string) {
    deleteIntent(id);
    deleting = null;
    editing = null;
  }

  function startClose(g: Intent) {
    closing = g.id;
    // at 100% the goal looks reached: suggested, still to be confirmed
    reason = g.progress >= 100 ? 'achieved' : null;
    editing = null;
  }

  function confirmClose() {
    if (closing && reason) closeIntent(closing, reason);
    closing = null;
    reason = null;
  }

  function slide(id: string, v: number) {
    moving = { ...moving, [id]: v };
  }

  function letGo(id: string, v: number) {
    setProgress(id, v);
    const rest = { ...moving };
    delete rest[id];
    moving = rest;
  }
</script>

<div class="page">
  <p class="lead glass">{$t('intent.lead')}</p>

  <form class="card glass new" on:submit|preventDefault={add}>
    <textarea
      rows="2"
      maxlength={MAX_INTENT_TEXT}
      placeholder={$t('intent.placeholder')}
      aria-label={$t('intent.placeholder')}
      bind:value={draft}
    ></textarea>
    <button class="btn primary" type="submit" disabled={!draft.trim()}>{$t('common.add')}</button>
  </form>

  {#each open as g (g.id)}
    {@const value = moving[g.id] ?? g.progress}
    <section class="card glass goal">
      {#if editing === g.id}
        <textarea rows="3" maxlength={MAX_INTENT_TEXT} aria-label={$t('intent.edit')} bind:value={editText}></textarea>
        <div class="actions">
          {#if deleting === g.id}
            <span class="ask">{$t('intent.deleteConfirm')}</span>
            <button class="btn ghost small" on:click={() => (deleting = null)}>{$t('common.no')}</button>
            <button class="btn danger small" on:click={() => remove(g.id)}>{$t('common.delete')}</button>
          {:else}
            <button class="link danger-link" on:click={() => (deleting = g.id)}>{$t('common.delete')}</button>
            <span class="spacer"></span>
            <button class="btn ghost small" on:click={() => (editing = null)}>{$t('common.cancel')}</button>
            <button class="btn primary small" on:click={saveEdit} disabled={!editText.trim()}>{$t('intent.save')}</button>
          {/if}
        </div>
      {:else}
        <p class="text">{g.text}</p>
        <div class="since">{$t('intent.since', { d: day(g.created) })}</div>
        <label class="progress">
          <span class="p-head"><span>{$t('intent.progress')}</span><strong>{value}%</strong></span>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            {value}
            on:input={(e) => slide(g.id, Number(e.currentTarget.value))}
            on:change={(e) => letGo(g.id, Number(e.currentTarget.value))}
          />
        </label>
        {#if closing === g.id}
          <div class="close-box">
            <p>{$t('intent.closeAsk')}</p>
            <div class="reasons" role="radiogroup" aria-label={$t('intent.closeAsk')}>
              {#each INTENT_REASONS as r}
                <button role="radio" aria-checked={reason === r} class:on={reason === r} on:click={() => (reason = r)}>
                  <span class="dot" aria-hidden="true"></span>{$t(`intent.${r}` as Key)}
                </button>
              {/each}
            </div>
            <div class="actions end">
              <button class="btn ghost small" on:click={() => (closing = null)}>{$t('common.cancel')}</button>
              <button class="btn primary small" on:click={confirmClose} disabled={!reason}>{$t('intent.closeConfirm')}</button>
            </div>
          </div>
        {:else}
          {#if value >= 100}<p class="reached">{$t('intent.reached')}</p>{/if}
          <div class="actions end">
            <button class="btn ghost small" on:click={() => startEdit(g)}>{$t('intent.edit')}</button>
            <button class="btn small" class:primary={value >= 100} class:ghost={value < 100} on:click={() => startClose(g)}>{$t('intent.close')}</button>
          </div>
        {/if}
      {/if}
    </section>
  {:else}
    {#if !closed.length}<p class="empty">{$t('intent.empty')}</p>{/if}
  {/each}

  {#if closed.length}
    <section class="card glass closed">
      <button class="closed-head" on:click={() => (showClosed = !showClosed)} aria-expanded={showClosed}>
        <span>{$t('intent.closedTitle', { n: closed.length })}</span>
        <svg viewBox="0 0 24 24" aria-hidden="true" class:up={showClosed}><path d="M6 9l6 6 6-6" /></svg>
      </button>
      {#if showClosed}
        <ul>
          {#each closed as g (g.id)}
            <li>
              <p class="text">{g.text}</p>
              <div class="meta">
                <span class="tag" data-reason={g.reason}>{$t(`intent.${g.reason}` as Key)}</span>
                <span>{$t('intent.closedOn', { d: day(g.closed) })}</span>
              </div>
              <div class="actions">
                {#if deleting === g.id}
                  <span class="ask">{$t('intent.deleteConfirm')}</span>
                  <button class="btn ghost small" on:click={() => (deleting = null)}>{$t('common.no')}</button>
                  <button class="btn danger small" on:click={() => remove(g.id)}>{$t('common.delete')}</button>
                {:else}
                  <button class="link" on:click={() => reopenIntent(g.id)}>{$t('intent.reopen')}</button>
                  <button class="link danger-link" on:click={() => (deleting = g.id)}>{$t('common.delete')}</button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  .page {
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: 8px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .lead {
    margin: 0;
    padding: 14px 16px;
    line-height: 1.5;
  }
  .card {
    padding: 14px 16px;
  }
  textarea {
    width: 100%;
    resize: vertical;
    min-height: 3em;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    line-height: 1.4;
  }
  textarea:focus {
    outline: none;
    border-color: var(--accent);
  }
  .new {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .btn {
    border: none;
    border-radius: 12px;
    padding: 11px 14px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.small {
    padding: 8px 12px;
    font-size: 0.9rem;
    white-space: nowrap;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .btn.ghost {
    background: var(--surface-2);
    color: var(--text);
  }
  .btn.danger {
    background: var(--red);
    color: #fff;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .link {
    padding: 4px 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 0.88rem;
    cursor: pointer;
  }
  .danger-link {
    color: var(--red);
  }
  .goal {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .text {
    margin: 0;
    font-size: 1.02rem;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .since {
    color: var(--faint);
    font-size: 0.8rem;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 6px;
  }
  .p-head {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 0.86rem;
  }
  .p-head strong {
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  input[type='range'] {
    width: 100%;
    margin: 0;
    accent-color: var(--accent);
  }
  .reached {
    margin: 4px 0 0;
    color: var(--accent);
    font-size: 0.88rem;
  }
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 6px;
  }
  .actions.end {
    justify-content: flex-end;
  }
  .spacer {
    flex: 1;
  }
  .ask {
    flex: 1;
    font-size: 0.88rem;
  }
  .close-box {
    margin-top: 6px;
    padding: 12px;
    border-radius: 12px;
    background: var(--surface-2);
    border: 1px solid var(--line);
  }
  .close-box p {
    margin: 0 0 10px;
    font-size: 0.92rem;
  }
  .reasons {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .reasons button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .reasons button.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .dot {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    border-radius: 50%;
    border: 2px solid var(--muted);
  }
  .reasons button.on .dot {
    border-color: var(--accent);
    background: radial-gradient(circle, var(--accent) 45%, transparent 50%);
  }
  .empty {
    margin: 4px 4px 0;
    color: var(--muted);
    text-align: center;
    font-size: 0.9rem;
    text-shadow: var(--title-shadow);
  }
  .closed-head {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0;
    border: none;
    background: none;
    font-weight: 600;
    cursor: pointer;
  }
  .closed-head svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.2s;
  }
  .closed-head svg.up {
    transform: rotate(180deg);
  }
  .closed ul {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .closed li {
    padding: 12px 0;
    border-top: 1px solid var(--line);
  }
  .closed li .text {
    font-size: 0.95rem;
    color: var(--muted);
  }
  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 6px;
    color: var(--faint);
    font-size: 0.8rem;
  }
  .tag {
    padding: 2px 9px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text);
    font-weight: 600;
  }
  .tag[data-reason='achieved'] {
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
