<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { api, errText, fmtClock, type PersonalProfile, type ValidationResult, type TrainingNoise } from '$lib/api';
  import { t, type Key } from '$lib/i18n';
  import { hasNative, keepScreen, shareFile, stopCounting } from '$lib/native';
  import { liveRunning } from '$lib/live';
  import KaraokeLine from '$lib/KaraokeLine.svelte';

  type Speed = 'slow' | 'medium' | 'fast';
  const SPEEDS: { id: Speed; label: Key; n: number; how: Key }[] = [
    { id: 'slow', label: 'voice.slow', n: 3, how: 'voice.howSlow' },
    { id: 'medium', label: 'voice.medium', n: 5, how: 'voice.howMedium' },
    { id: 'fast', label: 'voice.fast', n: 10, how: 'voice.howFast' },
  ];

  let profile: PersonalProfile | null = null;
  let speed: Speed = 'medium';
  // speed selected when the current take was recorded
  let recordedSpeed: Speed = 'medium';
  const NOISES: { id: TrainingNoise; label: Key }[] = [
    { id: 'juzu', label: 'voice.juzu' },
    { id: 'bell', label: 'voice.bell' },
    { id: 'both', label: 'voice.both' },
  ];
  let noisy = false;
  let noise: TrainingNoise = 'juzu';
  let recordedNoise: TrainingNoise | null = null;
  let n = 5;
  let recording = false;
  let busy = false;
  let secs = 0;
  let speaking = false;
  let tick: ReturnType<typeof setInterval> | null = null;
  const NOISE_SECONDS = 3;

  let takeIndex: number | null = null;
  let result: ValidationResult | null = null;
  let added = false;
  let confirmReset = false;
  let confirmDelete: number | null = null;
  let errorText = '';
  let voiceMsg = '';
  let voiceFile: HTMLInputElement;
  let pendingVoice: { content: string; n: number } | null = null;
  /** A counting session holds the microphone (started on the Count page). */
  let liveBusy = false;
  $: micTaken = !recording && ($liveRunning || liveBusy);

  async function shareVoice() {
    errorText = '';
    voiceMsg = '';
    try {
      const f = await api.exportVoice();
      if (hasNative()) await shareFile(f, 'application/json', $t('voice.share'));
      else voiceMsg = $t('set.fileSavedIn', { p: f.path });
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function shareTake(id: number) {
    errorText = '';
    voiceMsg = '';
    try {
      const f = await api.exportTake(id);
      if (hasNative()) await shareFile(f, 'audio/wav', $t('share.button'));
      else voiceMsg = $t('set.fileSavedIn', { p: f.path });
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function pickVoice() {
    errorText = '';
    voiceMsg = '';
    const file = voiceFile.files?.[0];
    voiceFile.value = '';
    if (!file) return;
    try {
      const content = await file.text();
      const head = JSON.parse(content);
      const ok = head?.format === 'daimoku-counter-voice' || head?.format === 'daimoku-counter-backup';
      const n = Array.isArray(head?.takes) ? head.takes.length : 0;
      if (!ok || n === 0) throw new Error('invalid');
      pendingVoice = { content, n };
    } catch {
      errorText = $t('voice.importInvalid');
    }
  }

  async function confirmImport() {
    if (!pendingVoice || busy) return;
    busy = true;
    try {
      const r = await api.importVoice(pendingVoice.content);
      voiceMsg = $t('voice.imported', { n: r.takes });
      await loadProfile();
    } catch (e) {
      errorText = String(e).includes('voice-invalid') ? $t('voice.importInvalid') : errText(e);
    } finally {
      pendingVoice = null;
      busy = false;
    }
  }

  $: sp = SPEEDS.find((s) => s.id === speed)!;
  $: done = coverage(profile, noisy ? noise : null);

  function speedOf(ms: number): Speed {
    if (ms >= 2600) return 'slow';
    if (ms >= 1150) return 'medium';
    return 'fast';
  }

  function takeMs(tk: PersonalProfile['takes'][number]): number {
    return tk.period_ms ?? (tk.duration_secs * 1000) / Math.max(1, tk.n_daimoku);
  }

  /** The speed chosen when recording; older takes: guessed from timing. */
  function takeSpeed(tk: PersonalProfile['takes'][number]): Speed {
    return tk.speed ?? speedOf(takeMs(tk));
  }

  function coverage(p: PersonalProfile | null, condition: TrainingNoise | null): Record<Speed, boolean> {
    const c = { slow: false, medium: false, fast: false };
    for (const tk of p?.takes ?? []) {
      if ((tk.noise ?? null) === condition) c[takeSpeed(tk)] = true;
    }
    return c;
  }

  function pick(s: Speed) {
    if (recording) return;
    speed = s;
    n = SPEEDS.find((x) => x.id === s)!.n;
  }

  async function loadProfile() {
    try {
      profile = await api.profile();
    } catch (e) {
      errorText = errText(e);
    }
  }

  async function start() {
    if (busy) return;
    busy = true;
    errorText = '';
    result = null;
    takeIndex = null;
    added = false;
    try {
      await api.startRecording();
      recordedSpeed = speed;
      recordedNoise = noisy ? noise : null;
      recording = true;
      secs = 0;
      tick = setInterval(async () => {
        try {
          const view = await api.liveView();
          secs = view.elapsed_secs;
          speaking = secs >= NOISE_SECONDS && view.speaking;
        } catch {
          speaking = false;
        }
      }, 200);
    } catch (e) {
      if (String(e).includes('already recording')) liveBusy = true;
      else errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  /** Stops the counting session that holds the microphone (it is saved as usual). */
  async function stopLive() {
    if (busy) return;
    busy = true;
    errorText = '';
    try {
      liveRunning.set(false);
      stopCounting();
      await api.stopLive();
      liveBusy = false;
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    if (busy || !recording) return;
    busy = true;
    if (tick) clearInterval(tick);
    tick = null;
    speaking = false;
    try {
      const meta = await api.stopRecording(n, true, recordedNoise);
      recording = false;
      takeIndex = meta.id;
      if (meta.duration_ms < 1500) {
        errorText = $t('voice.tooShort');
        takeIndex = null;
      } else {
        result = await api.validateTake(meta.id);
      }
    } catch (e) {
      errorText = String(e).includes('calibration-too-short') ? $t('voice.tooShort') : errText(e);
      recording = false;
    } finally {
      busy = false;
    }
  }

  async function add() {
    if (takeIndex === null || busy) return;
    busy = true;
    try {
      profile = await api.addTakeToProfile(takeIndex, recordedSpeed);
      added = true;
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  function discard() {
    result = null;
    takeIndex = null;
    added = false;
  }

  async function removeTake(id: number) {
    busy = true;
    try {
      profile = await api.deleteTake(id);
      confirmDelete = null;
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  async function resetVoice() {
    try {
      await api.clearProfile();
      confirmReset = false;
      await loadProfile();
    } catch (e) {
      errorText = errText(e);
    }
  }

  // --- Guided recording ----------------------------------------------------
  // One recording, cut into takes afterwards: the page says what to chant
  // and lights up the syllables at the user's pace, so nobody has to count.
  // Everything runs on the clock of the audio (seconds heard by the engine).
  type GuideStage = { speed: Speed; n: number; noise: TrainingNoise | null };
  /** A short practice: the bell opens and closes it, the beads in between. */
  const GUIDE: GuideStage[] = [
    { speed: 'slow', n: 3, noise: 'bell' },
    { speed: 'fast', n: 10, noise: 'juzu' },
    { speed: 'slow', n: 3, noise: 'bell' },
    { speed: 'slow', n: 3, noise: null },
    { speed: 'medium', n: 6, noise: null },
  ];
  /** Milliseconds per Daimoku when the profile does not say. */
  const DEFAULT_MS: Record<Speed, number> = { slow: 5600, medium: 1700, fast: 1050 };
  const PACE_LIMITS: Record<Speed, [number, number]> = { slow: [2600, 12000], medium: [1150, 2600], fast: [500, 1150] };
  /** Seconds to read what comes next before each stage. */
  const READY_SECS = 4;
  /** Share of a Daimoku spent on the syllables; the rest is breath. */
  const VOICED: Record<Speed, number> = { slow: 0.8, medium: 0.92, fast: 0.94 };
  const DAIMOKU = ['Nam', 'myo', 'ho', 'ren', 'ge', 'kyo'].flatMap((text, i) =>
    i === 0 ? [{ text, beat: true }] : [{ text: '-', beat: false }, { text, beat: true }]
  );
  const HINT: Record<'bell' | 'juzu' | 'both' | 'clean', Key> = {
    bell: 'guide.hintBell',
    juzu: 'guide.hintJuzu',
    both: 'voice.both',
    clean: 'guide.hintClean',
  };

  /** A stage on the clock: read from `ready`, chant from `from` to `to`, recorded until `end`. */
  type Slot = { ready: number; from: number; to: number; end: number; ms: number };
  type GuideRow = { id: number; expected: number; detected: number | null; keep: boolean };

  let guide: null | 'intro' | 'run' | 'busy' | 'review' | 'added' = null;
  let pace: Record<Speed, number> = { ...DEFAULT_MS };
  let slots: Slot[] = [];
  let gNow = 0;
  let gSync = { audio: 0, wall: 0 };
  let gPoll: ReturnType<typeof setInterval> | null = null;
  let gFrame = 0;
  let gRows: GuideRow[] = [];
  let gBell = 0;

  /** The user's own pace at each speed: the median of their clean takes. */
  function ownPace(p: PersonalProfile | null): Record<Speed, number> {
    const out = { ...DEFAULT_MS };
    for (const sp of ['slow', 'medium', 'fast'] as Speed[]) {
      const ms = (p?.takes ?? [])
        .filter((tk) => !tk.noise && takeSpeed(tk) === sp)
        .map(takeMs)
        .sort((a, b) => a - b);
      if (ms.length) out[sp] = Math.min(PACE_LIMITS[sp][1], Math.max(PACE_LIMITS[sp][0], ms[Math.floor(ms.length / 2)]));
    }
    return out;
  }

  function changePace(sp: Speed, factor: number) {
    const [lo, hi] = PACE_LIMITS[sp];
    pace = { ...pace, [sp]: Math.min(hi, Math.max(lo, Math.round(pace[sp] * factor))) };
  }

  function openGuide() {
    errorText = '';
    pace = ownPace(profile);
    guide = 'intro';
  }

  function timetable(): Slot[] {
    let at = NOISE_SECONDS;
    return GUIDE.map((g) => {
      const ms = pace[g.speed];
      const from = at + READY_SECS;
      const to = from + (g.n * ms) / 1000;
      // a little after the last syllable: whoever chants runs behind the lights
      const end = to + Math.min(2, Math.max(0.8, (0.35 * ms) / 1000));
      const slot = { ready: at, from, to, end, ms };
      at = end;
      return slot;
    });
  }

  function stopGuideClock() {
    if (gPoll) clearInterval(gPoll);
    gPoll = null;
    cancelAnimationFrame(gFrame);
    keepScreen('voice', false);
  }

  async function startGuide() {
    if (busy) return;
    busy = true;
    errorText = '';
    try {
      await api.startRecording();
      slots = timetable();
      gNow = 0;
      gSync = { audio: 0, wall: performance.now() };
      guide = 'run';
      keepScreen('voice', true);
      gPoll = setInterval(async () => {
        try {
          const view = await api.liveView();
          gSync = { audio: view.elapsed_secs, wall: performance.now() };
        } catch {
          /* the next answer sets the clock again */
        }
      }, 250);
      const step = () => {
        if (guide !== 'run') return;
        // never backwards: the engine's clock arrives in small jumps
        gNow = Math.max(gNow, gSync.audio + (performance.now() - gSync.wall) / 1000);
        if (gNow >= slots[slots.length - 1].end) finishGuide();
        else gFrame = requestAnimationFrame(step);
      };
      gFrame = requestAnimationFrame(step);
    } catch (e) {
      if (String(e).includes('already recording')) liveBusy = true;
      else errorText = errText(e);
      guide = null;
    } finally {
      busy = false;
    }
  }

  async function finishGuide() {
    if (guide !== 'run') return;
    guide = 'busy';
    stopGuideClock();
    try {
      const stages = slots.map((sl, i) => ({
        // a moment before the lights start: some begin early
        start_ms: Math.round((sl.from - 0.3) * 1000),
        end_ms: Math.round(sl.end * 1000),
        n: GUIDE[i].n,
        speed: GUIDE[i].speed,
        noise: GUIDE[i].noise,
      }));
      const cut = await api.stopGuided(stages);
      gBell = cut.bell_partials;
      gRows = cut.takes.map((m) => ({ id: m.id, expected: m.expected_daimoku_count, detected: null, keep: true }));
      for (let i = 0; i < gRows.length; i++) {
        try {
          gRows[i].detected = (await api.validateTake(gRows[i].id)).detected;
        } catch {
          gRows[i].detected = null;
        }
        gRows = gRows;
      }
      guide = 'review';
    } catch (e) {
      errorText = String(e).includes('calibration-too-short') ? $t('voice.tooShort') : errText(e);
      guide = null;
    }
  }

  async function addGuide() {
    if (busy) return;
    busy = true;
    try {
      profile = await api.addGuidedTakes(gRows.filter((r) => r.keep).map((r) => r.id));
      await api.clearTrainingTakes().catch(() => {});
      guide = 'added';
    } catch (e) {
      errorText = errText(e);
    } finally {
      busy = false;
    }
  }

  async function closeGuide() {
    const running = guide === 'run';
    guide = null;
    stopGuideClock();
    if (running) await api.stopRecording(1).catch(() => {});
    await api.clearTrainingTakes().catch(() => {});
  }

  $: gIndex = guide === 'run' ? Math.max(0, slots.findIndex((sl) => gNow < sl.end)) : 0;
  $: gSlot = guide === 'run' ? slots[gIndex] : null;
  $: gStage = GUIDE[gIndex];
  $: gPhase = !gSlot ? 'idle' : gNow < NOISE_SECONDS ? 'silence' : gNow < gSlot.from ? 'ready' : 'chant';
  /** Daimoku in progress (1..n) and how far its syllables have got (0..6). */
  $: gTurn = gSlot && gPhase === 'chant' ? Math.min(gStage.n, Math.floor(((gNow - gSlot.from) * 1000) / gSlot.ms) + 1) : 0;
  $: gBeat =
    gSlot && gPhase === 'chant'
      ? gNow >= gSlot.to
        ? 6
        : Math.min(6, ((((gNow - gSlot.from) * 1000) % gSlot.ms) / gSlot.ms / VOICED[gStage.speed]) * 6)
      : 0;

  onMount(async () => {
    await loadProfile();
    // a session may still run in the engine although this page does not
    // know it (the page was reloaded)
    try {
      const v = await api.liveView();
      if (!recording && v.elapsed_secs > 0 && !v.finished) liveBusy = true;
    } catch {
      /* the Record button reports it */
    }
  });
  onDestroy(() => {
    if (tick) clearInterval(tick);
    if (recording) api.stopRecording(n).catch(() => {});
    const guiding = guide === 'run';
    stopGuideClock();
    if (guiding) api.stopRecording(1).catch(() => {});
  });
</script>

<div class="page">
  <h1>{$t('voice.title')}</h1>
  <p class="lead glass">{$t('voice.lead')}</p>

  {#if guide === null}
    <button class="btn primary guide-open" on:click={openGuide} disabled={busy || recording || result !== null || micTaken}>{$t('guide.title')}</button>
  {:else}
    <section class="card glass guide">
      {#if guide === 'intro'}
        <h2>{$t('guide.title')}</h2>
        <p class="g-lead">{$t('guide.lead')}</p>
        <ol class="g-plan">
          {#each GUIDE as g}
            <li>
              <span class="tk-speed {g.speed}">{$t(('voice.' + g.speed) as Key)}</span>
              <span>{$t('voice.takeLine', { n: g.n })}</span>
              <span class="muted">{$t(g.noise ? (('voice.' + g.noise) as Key) : 'voice.clean')}</span>
            </li>
          {/each}
        </ol>
        <p class="g-label">{$t('guide.tempo')}</p>
        {#each SPEEDS as s}
          <div class="howmany">
            <span>{$t(s.label)}</span>
            <div class="stepper">
              <button on:click={() => changePace(s.id, 1 / 1.06)} aria-label={$t('common.less')}>−</button>
              <span>{(pace[s.id] / 1000).toFixed(1)}</span>
              <button on:click={() => changePace(s.id, 1.06)} aria-label={$t('common.more')}>+</button>
            </div>
          </div>
        {/each}
        <p class="noise-note">{$t('voice.noiseInstruction')}</p>
        <div class="actions">
          <button class="btn primary" on:click={startGuide} disabled={busy}>{$t('guide.start')}</button>
          <button class="btn ghost" on:click={closeGuide} disabled={busy}>{$t('common.cancel')}</button>
        </div>
      {:else if guide === 'run' && gSlot}
        <div class="g-top">
          <span class="dot on"></span>
          <span>{$t('guide.stageOf', { i: gIndex + 1, n: GUIDE.length })}</span>
          <span class="clock">{fmtClock(gNow)}</span>
        </div>
        <div class="g-stage" role="status" aria-live="polite">
          {#if gPhase === 'silence'}
            <p class="g-big">{$t('voice.noiseWait', { n: Math.ceil(NOISE_SECONDS - gNow) })}</p>
          {:else}
            <p class="g-what">
              <span class="tk-speed {gStage.speed}">{$t(('voice.' + gStage.speed) as Key)}</span>
              <span>{$t('voice.takeLine', { n: gStage.n })}</span>
            </p>
            <p class="g-hint">{$t(HINT[gStage.noise ?? 'clean'])}</p>
            {#if gPhase === 'ready'}
              <p class="g-label">{$t('guide.next')}</p>
              <p class="g-count">{Math.ceil(gSlot.from - gNow)}</p>
            {:else}
              <p class="g-count">{gTurn} <span>{$t('voice.of', { n: gStage.n })}</span></p>
            {/if}
          {/if}
        </div>
        <div class="g-line" class:waiting={gPhase !== 'chant'}>
          <KaraokeLine pieces={DAIMOKU} beat={gBeat} />
        </div>
        <div class="g-dots" aria-hidden="true">
          {#each Array(gPhase === 'silence' ? 0 : gStage.n) as _, i}
            <span class:on={gPhase === 'chant' && i < gTurn}></span>
          {/each}
        </div>
        <button class="btn ghost" on:click={closeGuide}>{$t('common.cancel')}</button>
      {:else if guide === 'busy'}
        <p class="g-big" role="status">{$t('guide.busy')}</p>
      {:else if guide === 'review'}
        <h2>{$t('guide.title')}</h2>
        <p class="g-lead">{$t('guide.review')}</p>
        <ul class="g-rows">
          {#each gRows as row, i}
            <li>
              <label>
                <input type="checkbox" bind:checked={row.keep} />
                <span class="tk-speed {GUIDE[i].speed}">{$t(('voice.' + GUIDE[i].speed) as Key)}</span>
                <span class="muted">{$t(GUIDE[i].noise ? (('voice.' + GUIDE[i].noise) as Key) : 'voice.clean')}</span>
              </label>
              <span class="g-heard" class:good={row.detected === row.expected}>
                {row.detected ?? '…'} <span>{$t('voice.of', { n: row.expected })}</span>
              </span>
            </li>
          {/each}
        </ul>
        <p class="note" class:good={gBell > 0}>{$t(gBell > 0 ? 'guide.bellOk' : 'guide.bellNo')}</p>
        <div class="actions">
          <button class="btn primary" on:click={addGuide} disabled={busy || (gBell === 0 && !gRows.some((r) => r.keep))}>{busy ? $t('voice.learning') : $t('guide.add')}</button>
          <button class="btn ghost" on:click={closeGuide} disabled={busy}>{$t('voice.discard')}</button>
        </div>
      {:else if guide === 'added'}
        <p class="g-big">{$t('voice.added')}</p>
        <div class="actions">
          <button class="btn primary" on:click={closeGuide}>{$t('common.ok')}</button>
        </div>
      {/if}
    </section>
  {/if}

  {#if guide === null}
  <div class="seg training-mode two-options">
    <button class:on={!noisy} aria-pressed={!noisy} disabled={busy || recording || result !== null} on:click={() => (noisy = false)}>{$t('voice.clean')}</button>
    <button class:on={noisy} aria-pressed={noisy} disabled={busy || recording || result !== null} on:click={() => (noisy = true)}>{$t('voice.noisy')}</button>
  </div>
  {#if noisy}
    <div class="seg training-mode">
      {#each NOISES as condition}
        <button class:on={noise === condition.id} aria-pressed={noise === condition.id} disabled={busy || recording || result !== null} on:click={() => (noise = condition.id)}>{$t(condition.label)}</button>
      {/each}
    </div>
    <p class="noise-note">{$t('voice.noisyInstruction')}</p>
  {/if}

  <div class="checks">
    {#each SPEEDS as s}
      <div class="check glass" class:ok={done[s.id]}>
        <span class="mark">{done[s.id] ? '✓' : ''}</span>{$t(s.label)}
      </div>
    {/each}
  </div>

  <section class="card glass rec">
    {#if !recording && !result}
      <div class="seg">
        {#each SPEEDS as s}
          <button class:on={speed === s.id} on:click={() => pick(s.id)}>{$t(s.label)}</button>
        {/each}
      </div>
      <div class="howmany">
        <span>{$t('voice.howMany')}</span>
        <div class="stepper">
          <button on:click={() => (n = Math.max(1, n - 1))} aria-label={$t('common.less')}>−</button>
          <span>{n}</span>
          <button on:click={() => (n = Math.min(30, n + 1))} aria-label={$t('common.more')}>+</button>
        </div>
      </div>
      <p class="instr">{$t('voice.instr', { n, how: $t(sp.how) })}</p>
      {#if !noisy}<p class="noise-note">{$t('voice.noiseInstruction')}</p>{/if}
    {:else if recording}
      <div class="rec-live">
        <span class="dot" class:on={speaking}></span>
        <span role="status" aria-live="polite">{secs < NOISE_SECONDS
          ? $t('voice.noiseWait', { n: Math.ceil(NOISE_SECONDS - secs) })
          : $t('voice.recording', { n })}</span>
        <span class="clock">{fmtClock(Math.max(0, secs - NOISE_SECONDS))}</span>
      </div>
      {#if secs >= NOISE_SECONDS}<p class="noise-note" role="status">{$t('voice.noiseReady')}</p>{/if}
    {/if}

    {#if micTaken && !result}
      <p class="busy-note">{$t('voice.liveRunning')}</p>
      <button class="btn primary stop-live" on:click={stopLive} disabled={busy}>{$t('voice.stopLive')}</button>
    {/if}

    {#if !result}
      <button class="go" class:stop={recording} on:click={recording ? stop : start} disabled={busy || micTaken || (recording && secs < NOISE_SECONDS)} aria-label={recording ? $t('voice.finish') : $t('voice.record')}>
        {#if recording}
          <svg viewBox="0 0 24 24"><rect x="6.5" y="6.5" width="11" height="11" rx="2.5" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="6.5" /></svg>
        {/if}
      </button>
      <div class="go-label">{recording ? $t('voice.finish') : $t('voice.record')}</div>
    {:else}
      <div class="result" class:good={result.ok}>
        <div class="r-big">{result.detected} <span>{$t('voice.of', { n: result.expected })}</span></div>
        <div class="r-sub">
          {#if added}
            {$t('voice.added')}
          {:else if result.ok}
            {$t('voice.okAll')}
          {:else}
            {$t('voice.notAll')}
          {/if}
        </div>
      </div>
      <div class="actions">
        {#if added}
          <button class="btn primary" on:click={discard}>{$t('voice.another')}</button>
        {:else}
          <button class="btn ghost" on:click={discard} disabled={busy}>{$t('voice.discard')}</button>
          <button class="btn primary" on:click={add} disabled={busy}>{busy ? $t('voice.learning') : $t('common.add')}</button>
        {/if}
      </div>
      <p class="note">{$t('voice.note', { n: result.expected })}</p>
    {/if}
  </section>

  <section class="card glass">
    <div class="row">
      <div class="c-title">{$t('voice.learned')}</div>
      <div class="c-count">{profile?.takes.length ?? 0}</div>
    </div>
    {#if profile && profile.takes.length > 0}
      <ul>
        {#each profile.takes as tk (tk.id)}
          <li>
            <span class="tk-speed" data-speed={takeSpeed(tk)}>{$t(SPEEDS.find((s) => s.id === takeSpeed(tk))!.label)}</span>
            <span class="tk-main">
              {$t('voice.takeLine', { n: tk.n_daimoku })}
              <span class="condition">{$t(tk.noise ? NOISES.find((c) => c.id === tk.noise)!.label : 'voice.clean')}</span>
              <span class="muted">· {$t('voice.each', { s: (takeMs(tk) / 1000).toFixed(1) })}</span>
            </span>
            {#if confirmDelete === tk.id}
              <span class="tk-confirm">
                <button class="mini ghost" on:click={() => (confirmDelete = null)}>{$t('common.no')}</button>
                <button class="mini danger" on:click={() => removeTake(tk.id)} disabled={busy}>{$t('common.delete')}</button>
              </span>
            {:else}
              <button class="icon-btn share" on:click={() => shareTake(tk.id)} aria-label={$t('share.button')}>
                <svg class="share-ico" viewBox="0 0 24 24" aria-hidden="true"><circle cx="18" cy="5" r="2.6" /><circle cx="6" cy="12" r="2.6" /><circle cx="18" cy="19" r="2.6" /><path d="M8.3 10.8l7.4-4.3M8.3 13.2l7.4 4.3" /></svg>
              </button>
              <button class="icon-btn" on:click={() => (confirmDelete = tk.id)} aria-label={$t('voice.deleteConfirm')}>
                <svg viewBox="0 0 24 24"><path d="M5 7h14M10 7V5h4v2M7 7l1 12h8l1-12" /></svg>
              </button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if confirmReset}
        <div class="confirm">
          <span>{$t('voice.resetConfirm')}</span>
          <button class="btn ghost small" on:click={() => (confirmReset = false)}>{$t('common.no')}</button>
          <button class="btn danger small" on:click={resetVoice}>{$t('voice.resetYes')}</button>
        </div>
      {:else}
        <button class="reset" on:click={() => (confirmReset = true)}>{$t('voice.reset')}</button>
      {/if}
    {:else}
      <p class="muted small-text">{$t('voice.none')}</p>
    {/if}
    {#if pendingVoice}
      <div class="confirm">
        <span>{$t('voice.importConfirm', { n: pendingVoice.n })}</span>
        <button class="btn ghost small" on:click={() => (pendingVoice = null)}>{$t('common.no')}</button>
        <button class="btn primary small" on:click={confirmImport} disabled={busy}>{$t('common.yes')}</button>
      </div>
    {:else}
      <div class="voice-io">
        {#if profile && profile.takes.length > 0}
          <button class="btn ghost small" on:click={shareVoice} disabled={busy}>{$t('voice.share')}</button>
        {/if}
        <button class="btn ghost small" on:click={() => voiceFile.click()} disabled={busy}>{$t('voice.import')}</button>
      </div>
    {/if}
    {#if voiceMsg}<p class="muted small-text">{voiceMsg}</p>{/if}
    <input class="hidden-file" type="file" accept=".json,application/json" bind:this={voiceFile} on:change={pickVoice} />
  </section>

  {/if}

  {#if errorText}<p class="error">{errorText}</p>{/if}
</div>

<style>
  .guide-open {
    width: 100%;
    margin-bottom: 14px;
  }
  .guide {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 14px;
  }
  .guide h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .g-lead {
    margin: 0;
    line-height: 1.5;
    color: var(--muted);
    font-size: 0.92rem;
  }
  .g-plan,
  .g-rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .g-plan li,
  .g-rows li,
  .g-rows label {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .g-rows li {
    justify-content: space-between;
  }
  .g-rows input {
    width: 20px;
    height: 20px;
    accent-color: var(--accent);
  }
  .g-heard {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .g-heard span {
    font-weight: 400;
    color: var(--muted);
  }
  .g-heard.good {
    color: var(--accent);
  }
  .g-label {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .g-top {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .g-top .clock {
    margin-left: auto;
  }
  .g-stage {
    min-height: 168px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    text-align: center;
  }
  .g-stage p {
    margin: 0;
  }
  .g-what {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 1.15rem;
    font-weight: 600;
  }
  .g-hint {
    color: var(--accent);
    font-weight: 600;
  }
  .g-big {
    margin: 0;
    text-align: center;
    font-size: 1.2rem;
    font-weight: 600;
    line-height: 1.4;
  }
  .g-count {
    font-size: 3rem;
    font-weight: 700;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }
  .g-count span {
    font-size: 1.1rem;
    font-weight: 400;
    color: var(--muted);
  }
  .g-line {
    text-align: center;
    font-size: 1.7rem;
    font-weight: 600;
    transition: opacity 0.2s;
  }
  .g-line.waiting {
    opacity: 0.35;
  }
  .g-dots {
    min-height: 12px;
    display: flex;
    justify-content: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .g-dots span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--surface-2);
    border: 1px solid var(--line);
  }
  .g-dots span.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .note.good {
    color: var(--accent);
  }

  .training-mode { margin-bottom: 12px; }
  .seg.two-options { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .training-mode button { white-space: normal; min-width: 0; overflow-wrap: anywhere; }
  .condition { display: block; font-size: 0.8em; opacity: 0.7; }
  .noise-note { text-align: center; color: var(--accent); font-weight: 600; line-height: 1.5; margin: 12px 0; }
  .page {
    width: 100%;
    max-width: 520px;
    margin: 0 auto;
    padding: 18px 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h1 {
    font-size: 1.5rem;
    margin: 4px 2px 0;
    text-shadow: var(--title-shadow);
  }
  .lead {
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.45;
    margin: 0;
    padding: 12px 14px;
  }
  .checks {
    display: flex;
    gap: 8px;
  }
  .check {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 0;
    border-radius: 12px;
    color: var(--faint);
    font-size: 0.85rem;
  }
  .check .mark {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1.5px solid var(--faint);
    display: grid;
    place-items: center;
    font-size: 0.7rem;
  }
  .check.ok {
    color: var(--text);
  }
  .check.ok .mark {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-ink);
  }

  .card {
    padding: 16px;
  }
  .rec {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .seg {
    width: 100%;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    background: var(--surface-2);
    border-radius: 12px;
    padding: 3px;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 9px 0;
    border-radius: 9px;
    font-weight: 500;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .howmany {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .stepper {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 3px;
  }
  .stepper button {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: none;
    background: transparent;
    font-size: 1.2rem;
    cursor: pointer;
  }
  .stepper span {
    min-width: 3ch;
    text-align: center;
    font-weight: 700;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .busy-note {
    margin: 0 0 10px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--accent-soft);
    line-height: 1.45;
    font-size: 0.92rem;
  }
  .stop-live {
    display: block;
    margin: 0 auto 12px;
  }
  .instr {
    margin: 0;
    text-align: center;
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.4;
  }
  .rec-live {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.9rem;
    padding: 12px 0 4px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    animation: blink 1.2s infinite;
  }
  .dot.on {
    background: var(--green);
    box-shadow: 0 0 10px var(--green);
    animation: none;
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
  .clock {
    font-variant-numeric: tabular-nums;
    color: var(--faint);
  }
  button.go {
    width: 76px;
    height: 76px;
    border-radius: 50%;
    border: 3px solid var(--line);
    background: var(--surface-2);
    color: var(--red);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  button.go.stop {
    background: var(--red);
    border-color: var(--red);
    color: #fff;
  }
  button.go svg {
    width: 32px;
    height: 32px;
    fill: currentColor;
  }
  .go-label {
    margin-top: -8px;
    font-size: 0.8rem;
    color: var(--faint);
  }
  .result {
    text-align: center;
    padding: 6px 0 0;
  }
  .r-big {
    font-size: 2.6rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--accent);
  }
  .result.good .r-big {
    color: var(--green);
  }
  .r-big span {
    font-size: 1.1rem;
    color: var(--muted);
    font-weight: 500;
  }
  .r-sub {
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.4;
    max-width: 30ch;
    margin: 4px auto 0;
  }
  .actions {
    display: flex;
    gap: 10px;
    width: 100%;
  }
  .btn {
    flex: 1;
    border: none;
    border-radius: 12px;
    padding: 12px 10px;
    font-weight: 600;
    cursor: pointer;
  }
  .btn.small {
    flex: 0;
    padding: 8px 12px;
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
  }
  .note {
    margin: 0;
    font-size: 0.75rem;
    color: var(--faint);
    text-align: center;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .c-title {
    font-weight: 600;
  }
  .c-count {
    color: var(--accent);
    font-weight: 700;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--line);
    font-size: 0.9rem;
    min-height: 48px;
  }
  .tk-speed {
    font-size: 0.72rem;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    white-space: nowrap;
  }
  .tk-main {
    flex: 1;
    min-width: 0;
  }
  .muted {
    color: var(--muted);
  }
  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--faint);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .icon-btn svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .icon-btn:active {
    color: var(--red);
  }
  :global(.share-ico) {
    width: 1.15em;
    height: 1.15em;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    flex: none;
  }
  .icon-btn.share:active {
    color: var(--accent);
  }
  .tk-confirm {
    display: flex;
    gap: 6px;
  }
  .mini {
    border: none;
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .mini.ghost {
    background: var(--surface-2);
  }
  .mini.danger {
    background: var(--red);
    color: #fff;
  }
  .small-text {
    font-size: 0.85rem;
    margin: 8px 0 0;
  }
  .voice-io {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .hidden-file {
    display: none;
  }
  .reset {
    margin-top: 10px;
    background: none;
    border: none;
    color: var(--red);
    font-size: 0.85rem;
    padding: 4px 0;
    cursor: pointer;
  }
  .confirm {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 0.85rem;
    flex-wrap: wrap;
  }
  .confirm span {
    flex: 1;
  }
  .error {
    color: var(--red);
    font-size: 0.85rem;
  }
</style>
