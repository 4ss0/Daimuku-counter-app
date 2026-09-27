use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use serde::Serialize;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::oneshot;

use crate::dsp::{count_daimoku_with_profile, DaimokuCountResult, StreamingState};
use crate::dsp_common::{find_active_segments, moving_average, rms_envelope};
use crate::profile::PersonalProfile;

// -----------------------------------------------------------------------------
// Device enumeration
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<DeviceInfo>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    let devices = host
        .input_devices()
        .map_err(|e| format!("failed to enumerate input devices: {e}"))?
        .filter_map(|d| d.name().ok())
        .map(|name| DeviceInfo {
            is_default: Some(&name) == default_name.as_ref(),
            name,
        })
        .collect();

    Ok(devices)
}

// -----------------------------------------------------------------------------
// Live counter (incremental batch, with a bounded sliding window)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct LiveSnapshot {
    pub count: usize,
    pub state: StreamingState,
    pub period_ms: Option<f32>,
    pub sample_rate: u32,
}

pub struct LiveCounter {
    inner: Mutex<LiveInner>,
}

struct LiveInner {
    /// Only the recent "tail" of the recording — bounded by
    /// `MAX_WINDOW_SECS` — is kept here for recompute. Older audio is
    /// trimmed away once its count has been folded into
    /// `committed_count` (see `maybe_trim`).
    samples: Vec<f32>,
    sample_rate: u32,
    profile: Option<PersonalProfile>,
    profile_fp: u64,
    cached: Option<DaimokuCountResult>,
    last_compute_len: usize,
    computed_once: bool,

    /// Daimoku already counted in audio that has since been trimmed
    /// out of `samples`. Added to the freshly-computed tail count to
    /// get the total shown to the user.
    committed_count: usize,
    /// Total samples ever pushed since the last reset, independent of
    /// trimming — used only for the Warming/Locked/Idle state and
    /// elapsed-time logic, never for counting.
    total_samples_seen: u64,
    /// Most recent valid period estimate. Kept around so the UI
    /// doesn't flash back to "—" right after a trim, before the
    /// shrunk tail has re-accumulated enough audio to estimate it
    /// again.
    last_known_period_ms: Option<f32>,
    /// Bumped on every reset; lets `maybe_trim` detect and safely
    /// abandon a trim if a reset happened while it was computing.
    generation: u64,
}

const MIN_NEW_SECS_F32: f32 = 0.5;

/// Once the retained audio buffer grows past this, we look for a safe
/// place to "freeze" the older part of it into `committed_count` and
/// drop the raw samples — this is what keeps the matched-filter
/// recompute bounded instead of growing (and getting quadratically
/// expensive) with the whole session's length.
const MAX_WINDOW_SECS: f32 = 35.0;

/// Always keep at least this much of the most recent audio in the
/// live buffer, even right after a trim, so there's enough context
/// left for period/phase estimation.
const MIN_KEEP_SECS: f32 = 8.0;

impl LiveCounter {
    pub fn new(initial_sample_rate: u32) -> Self {
        Self {
            inner: Mutex::new(LiveInner {
                samples: Vec::new(),
                sample_rate: initial_sample_rate,
                profile: None,
                profile_fp: 0,
                cached: None,
                last_compute_len: 0,
                computed_once: false,
                committed_count: 0,
                total_samples_seen: 0,
                last_known_period_ms: None,
                generation: 0,
            }),
        }
    }

    pub fn reset(&self, sample_rate: u32) {
        let mut g = self.inner.lock().unwrap();
        g.samples.clear();
        g.sample_rate = sample_rate;
        g.cached = None;
        g.last_compute_len = 0;
        g.computed_once = false;
        g.committed_count = 0;
        g.total_samples_seen = 0;
        g.last_known_period_ms = None;
        g.generation = g.generation.wrapping_add(1);
    }

    pub fn set_profile(&self, profile: Option<PersonalProfile>) {
        if let Ok(mut g) = self.inner.lock() {
            let fp = profile_fingerprint(profile.as_ref());
            if fp != g.profile_fp {
                g.profile = profile;
                g.profile_fp = fp;
                g.computed_once = false;
            }
        }
    }

    pub fn push(&self, samples: &[f32]) {
        if let Ok(mut g) = self.inner.lock() {
            g.samples.extend_from_slice(samples);
            g.total_samples_seen = g.total_samples_seen.saturating_add(samples.len() as u64);
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.inner.lock().map(|g| g.sample_rate).unwrap_or(48_000)
    }

    pub fn snapshot(&self) -> LiveSnapshot {
        self.maybe_trim();

        let (to_compute, sr, cached, profile, committed, last_period) = {
            let g = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return empty_snapshot(),
            };
            let min_new = (g.sample_rate as f32 * MIN_NEW_SECS_F32) as usize;
            let enough = g.samples.len() >= g.sample_rate as usize / 4;
            let delta = g.samples.len().saturating_sub(g.last_compute_len);
            let should_compute = enough && (!g.computed_once || delta >= min_new);
            if should_compute {
                (
                    Some(g.samples.clone()),
                    g.sample_rate,
                    None,
                    g.profile.clone(),
                    g.committed_count,
                    g.last_known_period_ms,
                )
            } else {
                (
                    None,
                    g.sample_rate,
                    g.cached.clone(),
                    g.profile.clone(),
                    g.committed_count,
                    g.last_known_period_ms,
                )
            }
        };

        let (result, computed_len) = match to_compute {
            Some(s) => {
                let len = s.len();
                let r = count_daimoku_with_profile(&s, sr, profile.as_ref());
                (r, Some(len))
            }
            None => (cached, None),
        };

        if let Some(len) = computed_len {
            if let Ok(mut g) = self.inner.lock() {
                if g.samples.len() >= len {
                    g.cached = result.clone();
                    g.last_compute_len = len;
                    g.computed_once = true;
                    if let Some(r) = &result {
                        if r.period_ms > 0.0 {
                            g.last_known_period_ms = Some(r.period_ms);
                        }
                    }
                }
            }
        }

        let tail_count = result.as_ref().map(|r| r.count).unwrap_or(0);
        let total_count = committed + tail_count;
        let period_ms = result
            .as_ref()
            .map(|r| r.period_ms)
            .filter(|p| *p > 0.0)
            .or(last_period);

        let total_seen = self
            .inner
            .lock()
            .map(|g| g.total_samples_seen)
            .unwrap_or(0);

        let state = if total_seen < (sr as u64 / 2) {
            StreamingState::Warming
        } else if total_count == 0 {
            StreamingState::Idle
        } else if total_count >= 2 {
            StreamingState::Locked
        } else {
            StreamingState::Warming
        };

        LiveSnapshot {
            count: total_count,
            state,
            period_ms,
            sample_rate: sr,
        }
    }

    /// Bounds the recompute cost of `snapshot()`: if the retained
    /// buffer has grown past `MAX_WINDOW_SECS`, look for a silence gap
    /// safely inside it, run one batch count on everything before that
    /// gap, fold it into `committed_count`, and drop those samples.
    /// If no safe gap exists yet (e.g. one long unbroken recitation),
    /// this is a no-op and the buffer is simply allowed to grow a bit
    /// past the target until a pause happens — favoring correctness
    /// (never risk splitting a Daimoku) over a hard memory bound.
    fn maybe_trim(&self) {
        let (sr, generation) = match self.inner.lock() {
            Ok(g) => (g.sample_rate, g.generation),
            Err(_) => return,
        };
        if sr == 0 {
            return;
        }
        let max_window_samples = (MAX_WINDOW_SECS * sr as f32) as usize;

        let (samples_snapshot, profile_snapshot) = {
            let g = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            if g.generation != generation || g.samples.len() <= max_window_samples {
                return;
            }
            (g.samples.clone(), g.profile.clone())
        };

        let Some(cut_sample) = find_safe_cut_point(&samples_snapshot, sr, MIN_KEEP_SECS) else {
            return;
        };
        if cut_sample == 0 {
            return;
        }

        let dropped = &samples_snapshot[..cut_sample];
        let dropped_result = count_daimoku_with_profile(dropped, sr, profile_snapshot.as_ref());
        let dropped_count = dropped_result.as_ref().map(|r| r.count).unwrap_or(0);

        if let Ok(mut g) = self.inner.lock() {
            if g.generation == generation && g.samples.len() >= samples_snapshot.len() {
                g.samples.drain(0..cut_sample);
                g.committed_count += dropped_count;
                g.last_compute_len = g.last_compute_len.saturating_sub(cut_sample);
                g.cached = None;
                g.computed_once = false;
            }
        }
    }

    pub fn count(&self) -> usize {
        self.snapshot().count
    }
    pub fn state(&self) -> StreamingState {
        self.snapshot().state
    }
    pub fn period_secs(&self) -> Option<f32> {
        self.snapshot().period_ms.map(|ms| ms / 1000.0)
    }

    pub fn finish(&self) -> Option<DaimokuCountResult> {
        self.maybe_trim();
        let (samples, sr, profile, committed) = {
            let g = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return None,
            };
            if g.samples.is_empty() && g.committed_count == 0 {
                return None;
            }
            (
                g.samples.clone(),
                g.sample_rate,
                g.profile.clone(),
                g.committed_count,
            )
        };
        let tail = count_daimoku_with_profile(&samples, sr, profile.as_ref());
        merge_committed(committed, tail)
    }
}

#[cfg(test)]
impl LiveCounter {
    fn debug_buffered_secs(&self) -> f32 {
        let g = self.inner.lock().unwrap();
        if g.sample_rate == 0 {
            0.0
        } else {
            g.samples.len() as f32 / g.sample_rate as f32
        }
    }
}

fn empty_snapshot() -> LiveSnapshot {
    LiveSnapshot {
        count: 0,
        state: StreamingState::Idle,
        period_ms: None,
        sample_rate: 48_000,
    }
}

fn profile_fingerprint(p: Option<&PersonalProfile>) -> u64 {
    match p {
        None => 0,
        Some(p) => {
            let n = p.takes.len() as u64;
            let per = (p.natural_period_ms.max(0.0) * 10.0) as u64;
            n.wrapping_mul(1_000_003).wrapping_add(per)
        }
    }
}

/// Looks for a point inside a sustained silence that still leaves at
/// least `min_keep_secs` of audio after it, so trimming everything
/// before that point can never cut a Daimoku in half. Returns `None`
/// if no such pause exists yet.
fn find_safe_cut_point(samples: &[f32], sr: u32, min_keep_secs: f32) -> Option<usize> {
    if sr == 0 || samples.is_empty() {
        return None;
    }
    let hop_ms = 10u32;
    let env = rms_envelope(samples, sr, 30, hop_ms);
    if env.is_empty() {
        return None;
    }
    let smoothed = moving_average(&env, 20);
    let peak = smoothed.iter().cloned().fold(f32::MIN, f32::max);
    if peak <= 1e-6 {
        return None;
    }
    let floor = 0.12 * peak;
    // ~200ms of continuous silence counts as a real pause worth
    // cutting on.
    let min_gap_frames = ((200.0 / hop_ms as f32).ceil() as usize).max(2);
    let segments = find_active_segments(&smoothed, floor, min_gap_frames);
    if segments.len() < 2 {
        return None;
    }

    let hop_samples = (hop_ms as usize * sr as usize) / 1000;
    let keep_from_sample = samples
        .len()
        .saturating_sub((min_keep_secs * sr as f32) as usize);

    for w in segments.windows(2).rev() {
        let gap_start = w[0].1;
        let gap_end = w[1].0;
        if gap_end <= gap_start {
            continue;
        }
        let mid_frame = gap_start + (gap_end - gap_start) / 2;
        let mid_sample = mid_frame * hop_samples;
        if mid_sample > 0 && mid_sample <= keep_from_sample {
            return Some(mid_sample);
        }
    }
    None
}

fn merge_committed(
    committed: usize,
    tail: Option<DaimokuCountResult>,
) -> Option<DaimokuCountResult> {
    match tail {
        Some(mut r) => {
            r.count += committed;
            Some(r)
        }
        None if committed > 0 => Some(DaimokuCountResult {
            count: committed,
            confidence: 0.5,
            phrase_count: 0,
            mean_period_ms: 0.0,
            method: "committed-only".to_string(),
            period_ms: 0.0,
            segment_count: 0,
            duration_secs: 0.0,
            active_duration_secs: 0.0,
        }),
        None => None,
    }
}

// -----------------------------------------------------------------------------
// Recording engine
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct RecordedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

enum AudioCommand {
    Start,
    Stop(oneshot::Sender<RecordedAudio>),
}

pub struct AudioState {
    cmd_tx: Mutex<Sender<AudioCommand>>,
    live: Arc<LiveCounter>,
}

impl AudioState {
    pub fn spawn() -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let live = Arc::new(LiveCounter::new(48_000));
        let live_for_thread = Arc::clone(&live);

        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || audio_thread_main(cmd_rx, live_for_thread))
            .map_err(|e| format!("failed to spawn audio thread: {e}"))?;

        Ok(Self {
            cmd_tx: Mutex::new(cmd_tx),
            live,
        })
    }

    pub fn start(&self) -> Result<(), String> {
        self.cmd_tx
            .lock()
            .unwrap()
            .send(AudioCommand::Start)
            .map_err(|_| "audio thread is not running".to_string())
    }

    pub fn stop(&self) -> Result<oneshot::Receiver<RecordedAudio>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .lock()
            .unwrap()
            .send(AudioCommand::Stop(reply_tx))
            .map_err(|_| "audio thread is not running".to_string())?;
        Ok(reply_rx)
    }

    pub fn live(&self) -> Arc<LiveCounter> {
        Arc::clone(&self.live)
    }
    pub fn live_reset(&self, sample_rate: u32) {
        self.live.reset(sample_rate);
    }
    pub fn live_set_profile(&self, p: Option<PersonalProfile>) {
        self.live.set_profile(p);
    }
    pub fn live_count(&self) -> usize {
        self.live.count()
    }
    pub fn live_state(&self) -> StreamingState {
        self.live.state()
    }
    pub fn live_period_secs(&self) -> Option<f32> {
        self.live.period_secs()
    }
    pub fn live_sample_rate(&self) -> u32 {
        self.live.sample_rate()
    }
    pub fn live_finish(&self) -> Option<DaimokuCountResult> {
        self.live.finish()
    }
}

fn audio_thread_main(cmd_rx: mpsc::Receiver<AudioCommand>, live: Arc<LiveCounter>) {
    let mut current: Option<ActiveRecording> = None;

    for cmd in cmd_rx {
        match cmd {
            AudioCommand::Start => {
                if current.is_some() {
                    eprintln!("[audio] start ignored: already recording");
                    continue;
                }
                match ActiveRecording::start(Arc::clone(&live)) {
                    Ok(rec) => current = Some(rec),
                    Err(e) => eprintln!("[audio] failed to start: {e}"),
                }
            }
            AudioCommand::Stop(reply_tx) => {
                let result = current
                    .take()
                    .map(ActiveRecording::stop)
                    .unwrap_or(RecordedAudio {
                        samples: Vec::new(),
                        sample_rate: 0,
                    });
                let _ = reply_tx.send(result);
            }
        }
    }
}

struct ActiveRecording {
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    _stream: cpal::Stream,
}

fn on_stream_error(err: cpal::StreamError) {
    eprintln!("[audio] stream error: {err}");
}

fn downmix_to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels as usize;
    if ch <= 1 {
        return data.to_vec();
    }
    data.chunks_exact(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}

impl ActiveRecording {
    fn start(live: Arc<LiveCounter>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;

        let supported = device
            .default_input_config()
            .map_err(|e| format!("failed to get default input config: {e}"))?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        live.reset(sample_rate);

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        let stream = match sample_format {
            SampleFormat::F32 => {
                let buf = Arc::clone(&buffer);
                let live = Arc::clone(&live);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let mono = downmix_to_mono(data, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
                        live.push(&mono);
                    },
                    on_stream_error,
                    None,
                )
            }
            SampleFormat::I16 => {
                let buf = Arc::clone(&buffer);
                let live = Arc::clone(&live);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let as_f32: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        let mono = downmix_to_mono(&as_f32, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
                        live.push(&mono);
                    },
                    on_stream_error,
                    None,
                )
            }
            SampleFormat::U16 => {
                let buf = Arc::clone(&buffer);
                let live = Arc::clone(&live);
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let as_f32: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 - 32768.0) / 32768.0)
                            .collect();
                        let mono = downmix_to_mono(&as_f32, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
                        live.push(&mono);
                    },
                    on_stream_error,
                    None,
                )
            }
            other => return Err(format!("unsupported sample format: {other:?}")),
        }
        .map_err(|e| format!("failed to build input stream: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("failed to start stream: {e}"))?;

        Ok(Self {
            buffer,
            sample_rate,
            _stream: stream,
        })
    }

    fn stop(self) -> RecordedAudio {
        drop(self._stream);
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
        RecordedAudio {
            samples,
            sample_rate: self.sample_rate,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn can_enumerate_input_devices() {
        let devices = list_input_devices().expect("listing failed");
        for d in &devices {
            let marker = if d.is_default { " (default)" } else { "" };
            println!("  - {}{}", d.name, marker);
        }
        assert!(!devices.is_empty(), "no input devices found");
    }

    #[test]
    fn downmix_averages_channels() {
        let stereo = [1.0_f32, 3.0, -1.0, 1.0];
        assert_eq!(downmix_to_mono(&stereo, 2), vec![2.0, 0.0]);
        let mono_in = [0.5_f32, -0.25];
        assert_eq!(downmix_to_mono(&mono_in, 1), vec![0.5, -0.25]);
    }

    #[test]
    fn live_counter_starts_empty() {
        let lc = LiveCounter::new(48_000);
        let snap = lc.snapshot();
        assert_eq!(snap.count, 0);
        assert_eq!(snap.state, StreamingState::Warming);
        assert!(snap.period_ms.is_none());
    }

    #[test]
    fn live_counter_accepts_silence_without_counting() {
        let lc = LiveCounter::new(48_000);
        let silence = vec![0.0f32; 48_000];
        lc.push(&silence);
        assert_eq!(lc.count(), 0);
    }

    #[test]
    fn live_counter_reset_clears_state() {
        let lc = LiveCounter::new(48_000);
        let tone = vec![0.5f32; 480];
        lc.push(&tone);
        lc.reset(44_100);
        assert_eq!(lc.count(), 0);
        assert_eq!(lc.sample_rate(), 44_100);
    }

    fn synth_bumps(bumps: usize, period_s: f32, sr: u32) -> Vec<f32> {
        use std::f32::consts::PI;
        let n = (bumps as f32 * period_s * sr as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / sr as f32;
            let car = (2.0 * PI * 220.0 * t).sin() + 0.4 * (2.0 * PI * 660.0 * t).sin();
            let phase = (t / period_s).fract();
            let bump = 0.55 + 0.45 * (PI * phase).sin().powf(0.6);
            v.push(car * bump * 0.4);
        }
        v
    }

    fn synth_silence(secs: f32, sr: u32) -> Vec<f32> {
        vec![0.0; (secs * sr as f32) as usize]
    }

    #[test]
    fn live_counter_bounds_buffer_growth_across_pauses() {
        let sr = 48_000u32;
        let lc = LiveCounter::new(sr);
        lc.reset(sr);

        // Several minutes' worth of pushes, but with regular pauses so
        // the window has plenty of safe cut points.
        for _ in 0..30 {
            let chunk = synth_silence(2.0, sr);
            lc.push(&chunk);
            let _ = lc.snapshot();
        }

        assert!(
            lc.debug_buffered_secs() < (MAX_WINDOW_SECS * 2.0),
            "buffer grew to {:.1}s — trimming does not seem to be bounding it",
            lc.debug_buffered_secs()
        );
    }

    #[test]
    fn live_counter_preserves_count_across_a_trim() {
        let sr = 48_000u32;
        let lc = LiveCounter::new(sr);
        lc.reset(sr);

        let rounds = 5usize;
        let per_round = 10usize;
        for _ in 0..rounds {
            // Push in chunks and poll snapshot() in between, like the
            // real audio callback + UI poll do, so trimming has a
            // chance to kick in mid-session exactly like in
            // production.
            let active = synth_bumps(per_round, 1.0, sr);
            for chunk in active.chunks(96_000) {
                lc.push(chunk);
                let _ = lc.snapshot();
            }
            let pause = synth_silence(3.0, sr);
            for chunk in pause.chunks(96_000) {
                lc.push(chunk);
                let _ = lc.snapshot();
            }
        }

        let result = lc.finish().expect("expected a count");
        let expected = (rounds * per_round) as i64;
        let got = result.count as i64;
        assert!(
            (got - expected).abs() <= 10,
            "expected roughly {expected} Daimoku across {rounds} rounds with \
             pauses, got {got}"
        );
    }

    #[test]
    #[ignore]
    fn records_real_audio() {
        let state = AudioState::spawn().expect("failed to spawn audio engine");
        state.start().expect("failed to start");

        let elapsed_secs = 2.0_f32;
        std::thread::sleep(Duration::from_millis((elapsed_secs * 1000.0) as u64));

        let live_count = state.live_count();
        let live_state = state.live_state();
        println!("live: count={live_count} state={live_state:?}");

        let rx = state.stop().expect("failed to stop");
        let audio = block_on(rx);

        assert!(audio.sample_rate > 0, "sample rate is zero");
        assert!(!audio.samples.is_empty(), "no samples");

        let audio_secs = audio.samples.len() as f32 / audio.sample_rate as f32;
        assert!((1.0..=elapsed_secs + 0.2).contains(&audio_secs));
        println!(
            "captured {} samples at {} Hz = {:.2}s",
            audio.samples.len(),
            audio.sample_rate,
            audio_secs
        );
    }

    fn block_on(rx: oneshot::Receiver<RecordedAudio>) -> RecordedAudio {
        use std::sync::mpsc;
        let (tx, done_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build test runtime");
            let result = rt.block_on(rx).expect("audio thread dropped the reply");
            let _ = tx.send(result);
        });
        done_rx.recv().expect("test thread panicked")
    }
}