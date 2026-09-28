<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  // ---------------------------------------------------------------------
  // Types (mirror the Rust side)
  // ---------------------------------------------------------------------

  interface TakeRecord {
    id: number;
    n_daimoku: number;
    duration_secs: number;
    period_ms: number | null;
    created_at: string;
  }

  interface PersonalProfile {
    version: number;
    takes: TakeRecord[];
    base_clip_count: number;
    ref_llr: number;
    min_cycle_ms: number;
    max_cycle_ms: number;
  }

  interface TrainingTakeMeta {
    id: number;
    expected_daimoku_count: number;
    duration_ms: number;
    sample_rate: number;
    created_at: string;
  }

  interface DaimokuCountResult {
    count: number;
    confidence: number;
    phrase_count: number;
    mean_period_ms: number;
    method: string;
    period_ms: number;
    segment_count: number;
    duration_secs: number;
    active_duration_secs: number;
  }

  interface ValidationResult {
    expected: number;
    detected: number;
    ok: boolean;
    profile_used: boolean;
    analysis: DaimokuCountResult;
  }

  // ---------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------

  let profile: PersonalProfile | null = null;
  let recordings: TrainingTakeMeta[] = [];
  let validations: Record<number, ValidationResult | 'error'> = {};
  let errorText: Record<number, string> = {};

  let isRecording = false;
  let expectedCount = 1;
  let recordingLabel = 'medio';
  let elapsedMs = 0;
  let elapsedTimer: ReturnType<typeof setInterval> | null = null;
  let recordingStartedAt = 0;

  let busy = false;
  let statusMsg = '';

  const SPEED_PRESETS: { label: string; hint: string }[] = [
    { label: 'lento', hint: 'Recita un solo Nam-myoho-renge-kyo, il più lento possibile.' },
    { label: 'medio', hint: 'Recita un solo Nam-myoho-renge-kyo, al tuo ritmo naturale.' },
    { label: 'veloce', hint: 'Recita un solo Nam-myoho-renge-kyo, il più veloce possibile.' },
  ];

  // ---------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------

  onMount(refreshAll);
  onDestroy(() => {
    if (elapsedTimer) clearInterval(elapsedTimer);
  });

  async function refreshAll() {
    await Promise.all([refreshProfile(), refreshRecordings()]);
  }

  async function refreshProfile() {
    profile = await invoke<PersonalProfile>('get_personal_profile');
  }

  async function refreshRecordings() {
    recordings = await invoke<TrainingTakeMeta[]>('list_training_takes');
  }

  // ---------------------------------------------------------------------
  // Recording
  // ---------------------------------------------------------------------

  async function startRecording() {
    statusMsg = '';
    try {
      await invoke('start_recording');
      isRecording = true;
      recordingStartedAt = Date.now();
      elapsedMs = 0;
      elapsedTimer = setInterval(() => {
        elapsedMs = Date.now() - recordingStartedAt;
      }, 100);
    } catch (e) {
      statusMsg = `Impossibile avviare la registrazione: ${e}`;
    }
  }

  async function stopRecording() {
    if (elapsedTimer) {
      clearInterval(elapsedTimer);
      elapsedTimer = null;
    }
    isRecording = false;
    try {
      await invoke('stop_recording', { expectedDaimokuCount: expectedCount });
      await refreshRecordings();
    } catch (e) {
      statusMsg = `Errore durante il salvataggio: ${e}`;
    }
  }

  function pickSpeed(label: string) {
    recordingLabel = label;
    expectedCount = 1;
  }

  // ---------------------------------------------------------------------
  // Per-recording actions
  // ---------------------------------------------------------------------

  async function validate(index: number) {
    busy = true;
    try {
      const result = await invoke<ValidationResult>('validate_training_take', { index });
      validations = { ...validations, [index]: result };
    } catch (e) {
      validations = { ...validations, [index]: 'error' };
      errorText = { ...errorText, [index]: String(e) };
    } finally {
      busy = false;
    }
  }

  async function addToProfile(index: number) {
    busy = true;
    try {
      profile = await invoke<PersonalProfile>('add_take_to_profile', { index });
      statusMsg = 'Registrazione aggiunta al tuo profilo personale.';
    } catch (e) {
      statusMsg = `Impossibile aggiungere al profilo: ${e}`;
    } finally {
      busy = false;
    }
  }

  async function exportWav(index: number) {
    try {
      const path = await invoke<string>('export_training_wav', { index });
      statusMsg = `WAV esportato in: ${path}`;
    } catch (e) {
      statusMsg = `Esportazione fallita: ${e}`;
    }
  }

  async function clearRecordings() {
    await invoke('clear_training_takes');
    validations = {};
    errorText = {};
    await refreshRecordings();
  }

  async function clearProfile() {
    await invoke('clear_personal_profile');
    await refreshProfile();
  }

  // ---------------------------------------------------------------------
  // Formatting helpers
  // ---------------------------------------------------------------------

  function fmtSecs(ms: number): string {
    return (ms / 1000).toFixed(1) + ' s';
  }
  function fmtMs(ms: number | null): string {
    if (ms === null || ms === undefined) return '—';
    return (ms / 1000).toFixed(2) + ' s';
  }
  function fmtPct(x: number): string {
    return Math.round(x * 100) + '%';
  }
</script>

<div class="page">
  <header>
    <h1>Campioni</h1>
    <p class="subtitle">
      L'app riconosce "Nam-myoho-renge-kyo" per intero, non solo il ritmo. Parte già
      pronta con esempi generici: registra il tuo daimoku (uno lento, uno medio, uno
      veloce) per affinarla sulla tua voce.
    </p>
  </header>

  {#if profile}
    <section class="card profile-card">
      <div class="card-header">
        <h2>Il tuo profilo</h2>
        <button class="danger" on:click={clearProfile} disabled={busy}>Azzera profilo</button>
      </div>
      <div class="profile-stats">
        <div class="stat">
          <span class="stat-value">{profile.base_clip_count}</span>
          <span class="stat-label">esempi di base</span>
        </div>
        <div class="stat">
          <span class="stat-value">{profile.takes.length}</span>
          <span class="stat-label">tue registrazioni</span>
        </div>
        <div class="stat">
          <span class="stat-value">{fmtMs(profile.min_cycle_ms)}–{fmtMs(profile.max_cycle_ms)}</span>
          <span class="stat-label">intervallo di velocità</span>
        </div>
        <div class="stat">
          <span class="stat-value">{profile.ref_llr.toFixed(1)}</span>
          <span class="stat-label">nitidezza del modello</span>
        </div>
      </div>
      {#if profile.takes.length > 0}
        <ul class="take-list">
          {#each profile.takes as t}
            <li>
              #{t.id} · {t.n_daimoku} daimoku · {fmtSecs(t.duration_secs * 1000)}
              {#if t.period_ms !== null} · ~{fmtMs(t.period_ms)}/daimoku{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="hint">
          Nessuna registrazione ancora promossa. Registra sotto un daimoku lento, uno
          medio e uno veloce, poi premi "Aggiungi al profilo" su ciascuno.
        </p>
      {/if}
    </section>
  {/if}

  <section class="card">
    <h2>Nuova registrazione</h2>
    <div class="speed-picker">
      {#each SPEED_PRESETS as p}
        <button
          class:selected={recordingLabel === p.label}
          on:click={() => pickSpeed(p.label)}
          disabled={isRecording}
        >
          {p.label}
        </button>
      {/each}
    </div>
    <p class="hint">
      {SPEED_PRESETS.find((p) => p.label === recordingLabel)?.hint ?? ''}
    </p>

    <div class="count-row">
      <label for="count">Daimoku recitati in questa presa</label>
      <input id="count" type="number" min="1" bind:value={expectedCount} disabled={isRecording} />
    </div>

    <div class="record-row">
      {#if !isRecording}
        <button class="primary" on:click={startRecording}>Avvia registrazione</button>
      {:else}
        <button class="danger" on:click={stopRecording}>Ferma · {fmtSecs(elapsedMs)}</button>
      {/if}
    </div>
    {#if statusMsg}<p class="status">{statusMsg}</p>{/if}
  </section>

  <section class="card">
    <div class="card-header">
      <h2>Registrazioni di questa sessione</h2>
      {#if recordings.length > 0}
        <button class="ghost" on:click={clearRecordings}>Svuota lista</button>
      {/if}
    </div>

    {#if recordings.length === 0}
      <p class="hint">Nessuna registrazione in questa sessione.</p>
    {:else}
      <ul class="recording-list">
        {#each recordings as r}
          <li class="recording-row">
            <div class="recording-main">
              <span class="rec-title">{r.expected_daimoku_count} daimoku</span>
              <span class="rec-sub">{fmtSecs(r.duration_ms)}</span>
            </div>
            <div class="recording-actions">
              <button on:click={() => validate(r.id)} disabled={busy}>Verifica</button>
              <button on:click={() => addToProfile(r.id)} disabled={busy}>+ Profilo</button>
              <button class="ghost" on:click={() => exportWav(r.id)}>WAV</button>
            </div>

            {#if validations[r.id] === 'error'}
              <div class="result error">
                <strong>Verifica non riuscita</strong>
                <p>{errorText[r.id]}</p>
              </div>
            {:else if validations[r.id]}
              {@const v = validations[r.id] as ValidationResult}
              <div class="result" class:ok={v.ok} class:bad={!v.ok}>
                <strong>{v.ok ? '✓ Corrisponde' : '✗ Non corrisponde'} — presa #{r.id}</strong>
                <p>
                  Recitati: <b>{v.expected}</b> · Rilevati: <b>{v.detected}</b>
                </p>
                <p class="detail">
                  Periodo: {v.analysis.period_ms.toFixed(0)} ms · Confidenza:
                  {fmtPct(v.analysis.confidence)} · Segmenti: {v.analysis.segment_count} ·
                  Attivo: {v.analysis.active_duration_secs.toFixed(1)} s
                </p>
                {#if !v.ok}
                  <p class="detail hint">
                    Recita di nuovo con sillabe chiare e un ritmo costante, oppure
                    verifica di aver dichiarato il numero corretto di daimoku.
                  </p>
                {/if}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    max-width: 46rem;
    margin: 0 auto;
    width: 100%;
  }

  header h1 {
    margin: 0 0 0.4rem;
    font-size: 1.8rem;
  }
  .subtitle {
    margin: 0;
    opacity: 0.7;
    line-height: 1.5;
  }

  .card {
    background: #232323;
    border: 1px solid #333;
    border-radius: 12px;
    padding: 1.25rem 1.5rem;
  }
  .card h2 {
    margin: 0 0 0.75rem;
    font-size: 1.1rem;
  }
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .card-header h2 {
    margin: 0;
  }

  .profile-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .stat {
    background: #1a1a1a;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    display: flex;
    flex-direction: column;
  }
  .stat-value {
    font-size: 1.15rem;
    font-weight: 600;
  }
  .stat-label {
    font-size: 0.78rem;
    opacity: 0.65;
  }

  .take-list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 0.85rem;
    opacity: 0.85;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .hint {
    font-size: 0.85rem;
    opacity: 0.65;
    line-height: 1.4;
    margin: 0.4rem 0;
  }

  .speed-picker {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .speed-picker button {
    flex: 1;
    padding: 0.55rem 0.75rem;
    border-radius: 8px;
    border: 1px solid #3a3a3a;
    background: #1e1e1e;
    color: #ddd;
    text-transform: capitalize;
    cursor: pointer;
  }
  .speed-picker button.selected {
    background: #3b6ea5;
    border-color: #3b6ea5;
    color: #fff;
  }
  .speed-picker button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .count-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin: 0.75rem 0;
  }
  .count-row label {
    font-size: 0.85rem;
    opacity: 0.8;
  }
  .count-row input {
    width: 4.5rem;
    padding: 0.35rem 0.5rem;
    border-radius: 6px;
    border: 1px solid #3a3a3a;
    background: #1a1a1a;
    color: #eee;
  }

  .record-row {
    margin-top: 0.5rem;
  }

  button.primary {
    background: #3b6ea5;
    color: #fff;
    border: none;
    padding: 0.6rem 1.2rem;
    border-radius: 8px;
    cursor: pointer;
    font-size: 0.95rem;
  }
  button.danger {
    background: #a53b3b;
    color: #fff;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 8px;
    cursor: pointer;
  }
  button.ghost {
    background: transparent;
    border: 1px solid #444;
    color: #ccc;
    padding: 0.45rem 0.9rem;
    border-radius: 8px;
    cursor: pointer;
  }

  .status {
    font-size: 0.85rem;
    opacity: 0.8;
    margin-top: 0.5rem;
  }

  .recording-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .recording-row {
    background: #1a1a1a;
    border-radius: 10px;
    padding: 0.75rem 1rem;
  }
  .recording-main {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .rec-title {
    font-weight: 600;
  }
  .rec-sub {
    opacity: 0.6;
    font-size: 0.85rem;
  }
  .recording-actions {
    display: flex;
    gap: 0.5rem;
  }
  .recording-actions button {
    flex: 1;
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
    border: 1px solid #3a3a3a;
    background: #262626;
    color: #ddd;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .result {
    margin-top: 0.6rem;
    padding: 0.6rem 0.8rem;
    border-radius: 8px;
    font-size: 0.85rem;
    background: #262626;
    border: 1px solid #3a3a3a;
  }
  .result.ok {
    background: #16321f;
    border-color: #2a6b41;
  }
  .result.bad,
  .result.error {
    background: #33191b;
    border-color: #7a3038;
  }
  .result p {
    margin: 0.25rem 0 0;
  }
  .result .detail {
    opacity: 0.75;
  }
</style>
