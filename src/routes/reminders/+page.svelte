<script lang="ts">
  import { t } from '$lib/i18n';
  import { canRemind } from '$lib/native';
  import { prefs, updatePrefs } from '$lib/prefs';
  import { KINDS } from '$lib/reminders';

  const native = canRemind();

  function set(kind: (typeof KINDS)[number], patch: Partial<{ on: boolean; time: string }>) {
    const r = { ...$prefs.reminders[kind], ...patch };
    updatePrefs({ reminders: { ...$prefs.reminders, [kind]: r } });
  }
</script>

<div class="page">
  <p class="lead glass">{$t('rem.lead')}</p>

  {#each KINDS as kind}
    {@const r = $prefs.reminders[kind]}
    <section class="card glass" class:off={!r.on}>
      <label class="toggle">
        <span class="name">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            {#if kind === 'morning'}
              <circle cx="12" cy="12" r="4" /><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4" />
            {:else}
              <path d="M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5z" />
            {/if}
          </svg>
          {$t(kind === 'morning' ? 'rem.morning' : 'rem.evening')}
        </span>
        <input type="checkbox" role="switch" class="switch" checked={r.on} disabled={!native} on:change={(e) => set(kind, { on: e.currentTarget.checked })} />
      </label>
      <label class="time">
        <span>{$t('rem.at')}</span>
        <input type="time" value={r.time} disabled={!native || !r.on} on:change={(e) => e.currentTarget.value && set(kind, { time: e.currentTarget.value })} />
      </label>
    </section>
  {/each}

  <p class="note">{$t(native ? 'rem.note' : 'rem.noApp')}</p>
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
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .toggle,
  .time {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 600;
  }
  .name svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .time span {
    color: var(--muted);
  }
  .off .time {
    opacity: 0.5;
  }
  input[type='time'] {
    padding: 8px 10px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
  }
  .note {
    margin: 0 4px;
    color: var(--muted);
    font-size: 0.86rem;
    line-height: 1.5;
    text-shadow: var(--title-shadow);
  }
</style>
