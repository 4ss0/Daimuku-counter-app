use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use serde::Serialize;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::oneshot;

use crate::dsp::{DaimokuCountResult, StreamingState};
use crate::engine::{Engine, EngineState};
use crate::profile::{base_only_model, placeholder_model, PersonalProfile};

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
// Live counter
// -----------------------------------------------------------------------------
//
// Unlike the old template-matching counter, `Engine` is a true streaming
// decoder: `push()` is O(1) amortised per sample regardless of how long
// the recording has been running, and memory is bounded by a small ring
// buffer (~5s of lag), not by the whole session's audio. So there is no
// "recompute the whole buffer every half second" step to manage any more
// - `snapshot()` below is just reading state the engine already
// maintains incrementally.
//
// `finish()` finalises the engine's last few lag-buffered frames and is
// the only mutating, one-shot operation; it is wired to fire when the
// recording actually stops (`AudioCommand::Stop`, below), not on every
// poll, so it is safe for the frontend to call `finish()`/`live_finish`
// as often as it likes - before the real stop it returns the current
// best live estimate, and after it returns the same finalised result
// every time.

#[derive(Debug, Clone, Copy)]
pub struct LiveSnapshot {
    pub count: usize,
    pub state: StreamingState,
    pub period_ms: Option<f32>,
    pub sample_rate: u32,
}

/// Everything the live-counting screen shows. Cheap to build.
#[derive(Debug, Clone, Serialize)]
pub struct LiveView {
    pub count: usize,
    /// "idle" | "warming" | "locked"
    pub state: String,
    /// Someone is producing sound right now.
    pub speaking: bool,
    pub elapsed_secs: f32,
    /// Typical time per Daimoku (ms), once known.
    pub period_ms: Option<f32>,
    /// Second of the session at which each of the most recent Daimoku
    /// was counted (at most `LIVE_VIEW_EVENTS`, oldest first).
    pub recent_events_secs: Vec<f32>,
    pub finished: bool,
}

const LIVE_VIEW_EVENTS: usize = 50;

pub fn state_str(s: StreamingState) -> &'static str {
    match s {
        StreamingState::Warming => "warming",
        StreamingState::Locked => "locked",
        StreamingState::Idle => "idle",
    }
}

pub struct LiveCounter {
    inner: Mutex<LiveInner>,
}

struct LiveInner {
    engine: Option<Engine>,
    sample_rate: u32,
    /// Model to use for the *next* `reset()` (a running engine keeps the
    /// model it was built with - swapping models mid-recording would
    /// invalidate everything decoded so far).
    pending_model: crate::model::Model,
    /// Cached result of the one authoritative `finish()` call, if the
    /// recording has actually stopped.
    finished_result: Option<DaimokuCountResult>,
}

impl LiveCounter {
    pub fn new(initial_sample_rate: u32) -> Self {
        Self {
            inner: Mutex::new(LiveInner {
                engine: None,
                sample_rate: initial_sample_rate,
                // replaced by the real model as soon as it is loaded
                pending_model: placeholder_model(),
                finished_result: None,
            }),
        }
    }

    /// Starts a new recording: builds a fresh engine from the currently
    /// selected model, discarding any previous session's counts.
    pub fn reset(&self, sample_rate: u32) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.sample_rate = sample_rate;
        g.engine = Some(Engine::new(&g.pending_model, sample_rate));
        g.finished_result = None;
    }

    /// Selects the model to use for the *next* recording. Does not
    /// affect a recording already in progress.
    pub fn set_profile(&self, profile: Option<PersonalProfile>) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.pending_model = profile.map(|p| p.model).unwrap_or_else(base_only_model);
    }

    pub fn push(&self, samples: &[f32]) {
        if let Ok(mut g) = self.inner.lock() {
            if let Some(engine) = g.engine.as_mut() {
                engine.push(samples);
            }
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.inner.lock().map(|g| g.sample_rate).unwrap_or(48_000)
    }

    /// Cheap, non-destructive: safe to poll as often as the UI wants
    /// while recording is in progress.
    pub fn snapshot(&self) -> LiveSnapshot {
        let g = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return empty_snapshot(),
        };
        let sr = g.sample_rate;
        let Some(engine) = g.engine.as_ref() else {
            return LiveSnapshot {
                count: 0,
                state: EngineState::Idle,
                period_ms: None,
                sample_rate: sr,
            };
        };
        let s = engine.snapshot();
        LiveSnapshot {
            count: s.count,
            state: s.state,
            period_ms: s.period_ms,
            sample_rate: sr,
        }
    }

    /// Detailed view for the live-counting screen. Non-destructive.
    pub fn view(&self) -> LiveView {
        let empty = LiveView {
            count: 0,
            state: "idle".to_string(),
            speaking: false,
            elapsed_secs: 0.0,
            period_ms: None,
            recent_events_secs: Vec::new(),
            finished: false,
        };
        let g = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return empty,
        };
        let Some(engine) = g.engine.as_ref() else {
            return empty;
        };
        let snap = engine.snapshot();
        let ev = engine.events();
        let from = ev.len().saturating_sub(LIVE_VIEW_EVENTS);
        LiveView {
            count: snap.count,
            state: state_str(snap.state).to_string(),
            speaking: engine.is_speaking() && !engine.is_finished(),
            elapsed_secs: engine.elapsed_secs(),
            period_ms: snap.period_ms,
            recent_events_secs: ev[from..].iter().map(|e| e.frame as f32 * 0.01).collect(),
            finished: engine.is_finished(),
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

    /// Called once, when the recording has actually stopped (from the
    /// audio thread, right after the input stream is torn down).
    /// Finalises the engine's pending lag window and caches the result.
    /// Safe to call more than once (e.g. defensively) - only the first
    /// call does any work.
    fn mark_finished(&self) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if g.finished_result.is_some() {
            return;
        }
        let Some(engine) = g.engine.as_mut() else {
            return;
        };
        if !engine.is_finished() {
            engine.finish();
        }
        let sr = g.sample_rate;
        g.finished_result = Some(engine_to_result(g.engine.as_ref().unwrap(), sr));
    }

    /// Best current result: the authoritative one if the recording has
    /// stopped, otherwise a live estimate from what has been decoded so
    /// far. Never mutates engine state that would affect future counts.
    pub fn finish(&self) -> Option<DaimokuCountResult> {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = &g.finished_result {
            return Some(r.clone());
        }
        let engine = g.engine.as_ref()?;
        if engine.frames_seen() == 0 {
            return None;
        }
        let sr = g.sample_rate;
        Some(engine_to_result(engine, sr))
    }
}

fn engine_to_result(engine: &Engine, sample_rate: u32) -> DaimokuCountResult {
    let hop_secs = 0.010_f32;
    let duration_secs = engine.frames_seen() as f32 * hop_secs;
    let active_duration_secs = engine.active_frames() as f32 * hop_secs;
    let period_ms = engine.period_ms().unwrap_or(0.0);
    let confidence = if engine.count() > 0 {
        (engine.mean_llr() / engine.params().llr_hi.max(0.1)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let _ = sample_rate;
    DaimokuCountResult {
        count: engine.count(),
        confidence,
        phrase_count: 0,
        mean_period_ms: period_ms,
        method: "nam-myoho-renge-kyo".to_string(),
        period_ms,
        segment_count: 0,
        duration_secs,
        active_duration_secs,
    }
}

fn empty_snapshot() -> LiveSnapshot {
    LiveSnapshot {
        count: 0,
        state: EngineState::Idle,
        period_ms: None,
        sample_rate: 48_000,
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
    /// Start capturing. `keep_secs` caps how much audio is kept in memory
    /// (the live counter always sees everything): `None` = keep it all
    /// (training takes), `Some(n)` = keep only the first n seconds (live
    /// sessions, which can last an hour).
    Start {
        keep_secs: Option<u32>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop(oneshot::Sender<RecordedAudio>),
}

/// Audio kept from a live session, for "save WAV" (the first 10 minutes).
pub const LIVE_KEEP_SECS: u32 = 600;

pub struct AudioState {
    cmd_tx: Mutex<Sender<AudioCommand>>,
    live: Arc<LiveCounter>,
    /// Audio of the last finished live session (see `LIVE_KEEP_SECS`).
    last_live: Mutex<Option<RecordedAudio>>,
}

impl AudioState {
    pub fn spawn() -> Result<Self, String> {
        let live = Arc::new(LiveCounter::new(48_000));
        let cmd_tx = spawn_audio_thread(Arc::clone(&live))?;
        Ok(Self {
            cmd_tx: Mutex::new(cmd_tx),
            live,
            last_live: Mutex::new(None),
        })
    }

    /// Sends a command to the audio thread. If the thread is gone (it
    /// should never be: every call into the audio backend is guarded), a
    /// fresh one is started and the command is sent again.
    fn send(&self, cmd: AudioCommand) -> Result<(), String> {
        let mut tx = self.cmd_tx.lock().unwrap();
        match tx.send(cmd) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(cmd)) => {
                eprintln!("[audio] audio thread was gone, restarting it");
                *tx = spawn_audio_thread(Arc::clone(&self.live))?;
                tx.send(cmd)
                    .map_err(|_| "audio thread is not running".to_string())
            }
        }
    }

    /// Starts a recording that keeps all its audio (training takes).
    pub fn start(&self) -> Result<(), String> {
        self.send_start(None)
    }

    /// Starts a live counting session (audio kept only for the first
    /// `LIVE_KEEP_SECS`, so an hour of chanting cannot fill the memory).
    pub fn start_live(&self) -> Result<(), String> {
        self.send_start(Some(LIVE_KEEP_SECS))
    }

    /// Waits until the microphone is really open, so that a missing
    /// permission or device is reported to the UI instead of silently
    /// counting nothing.
    fn send_start(&self, keep_secs: Option<u32>) -> Result<(), String> {
        let (reply, rx) = mpsc::channel();
        self.send(AudioCommand::Start { keep_secs, reply })?;
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| "the microphone did not respond".to_string())?
    }

    pub fn set_last_live(&self, audio: RecordedAudio) {
        *self.last_live.lock().unwrap() = Some(audio);
    }

    pub fn last_live(&self) -> Option<RecordedAudio> {
        self.last_live.lock().unwrap().clone()
    }

    pub fn live_view(&self) -> LiveView {
        self.live.view()
    }

    pub fn stop(&self) -> Result<oneshot::Receiver<RecordedAudio>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.send(AudioCommand::Stop(reply_tx))?;
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

fn spawn_audio_thread(live: Arc<LiveCounter>) -> Result<Sender<AudioCommand>, String> {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    thread::Builder::new()
        .name("audio-engine".into())
        .spawn(move || audio_thread_main(cmd_rx, live))
        .map_err(|e| format!("failed to spawn audio thread: {e}"))?;
    Ok(cmd_tx)
}

/// Text of a caught panic, so it can be shown instead of lost.
fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

fn audio_thread_main(cmd_rx: mpsc::Receiver<AudioCommand>, live: Arc<LiveCounter>) {
    let mut current: Option<ActiveRecording> = None;

    for cmd in cmd_rx {
        match cmd {
            AudioCommand::Start { keep_secs, reply } => {
                if current.is_some() {
                    let _ = reply.send(Err("already recording".to_string()));
                    continue;
                }
                // The audio backend (AAudio via JNI on Android) may panic
                // on an unexpected device; report it instead of letting
                // this thread die.
                let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    ActiveRecording::start(Arc::clone(&live), keep_secs)
                }));
                match started {
                    Ok(Ok(rec)) => {
                        current = Some(rec);
                        let _ = reply.send(Ok(()));
                    }
                    Ok(Err(e)) => {
                        eprintln!("[audio] failed to start: {e}");
                        let _ = reply.send(Err(e));
                    }
                    Err(p) => {
                        let e = format!("audio backend error: {}", panic_text(p));
                        eprintln!("[audio] {e}");
                        let _ = reply.send(Err(e));
                    }
                }
            }
            AudioCommand::Stop(reply_tx) => {
                let rec = current.take();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    rec.map(ActiveRecording::stop)
                }))
                .ok()
                .flatten()
                .unwrap_or(RecordedAudio {
                    samples: Vec::new(),
                    sample_rate: 0,
                });
                // The stream is torn down now: no more audio will ever
                // arrive for this session, so this is the one correct
                // moment to finalise the engine's pending lag window.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| live.mark_finished()));
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

/// Appends to the kept recording, up to `max` samples in total.
fn keep(buf: &Mutex<Vec<f32>>, mono: &[f32], max: usize) {
    if let Ok(mut b) = buf.lock() {
        let room = max.saturating_sub(b.len());
        if room > 0 {
            let n = room.min(mono.len());
            b.extend_from_slice(&mono[..n]);
        }
    }
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
    fn start(live: Arc<LiveCounter>, keep_secs: Option<u32>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;

        // On Android, cpal's default_input_config() probes ~40 formats
        // through JNI; asking AAudio directly for 48 kHz mono is simpler
        // and more robust (AAudio converts from whatever the mic delivers).
        #[cfg(target_os = "android")]
        let (sample_rate, channels, formats, config) = (
            48_000u32,
            1u16,
            vec![SampleFormat::F32, SampleFormat::I16],
            StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48_000),
                buffer_size: cpal::BufferSize::Default,
            },
        );
        #[cfg(not(target_os = "android"))]
        let (sample_rate, channels, formats, config) = {
            let supported = device
                .default_input_config()
                .map_err(|e| format!("failed to get default input config: {e}"))?;
            let sr = supported.sample_rate().0;
            let ch = supported.channels();
            let fmt = supported.sample_format();
            let cfg: StreamConfig = supported.into();
            (sr, ch, vec![fmt], cfg)
        };

        live.reset(sample_rate);

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let keep_max = keep_secs.map_or(usize::MAX, |s| s as usize * sample_rate as usize);

        let build = |sample_format: SampleFormat| -> Result<cpal::Stream, String> {
            match sample_format {
                SampleFormat::F32 => {
                    let buf = Arc::clone(&buffer);
                    let live = Arc::clone(&live);
                    device.build_input_stream(
                        &config,
                        move |data: &[f32], _| {
                            let mono = downmix_to_mono(data, channels);
                            keep(&buf, &mono, keep_max);
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
                            keep(&buf, &mono, keep_max);
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
                            keep(&buf, &mono, keep_max);
                            live.push(&mono);
                        },
                        on_stream_error,
                        None,
                    )
                }
                other => return Err(format!("unsupported sample format: {other:?}")),
            }
            .map_err(|e| format!("failed to build input stream: {e}"))
        };

        // Try each candidate format in turn, keeping the first error.
        let mut stream = None;
        let mut first_err = None;
        for f in formats {
            match build(f) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => {
                    first_err.get_or_insert(e);
                }
            }
        }
        let stream = stream.ok_or_else(|| first_err.unwrap_or_else(|| "no input format".into()))?;

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
        let samples = std::mem::take(&mut *self.buffer.lock().unwrap_or_else(|e| e.into_inner()));
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
        assert!(snap.period_ms.is_none());
    }

    #[test]
    fn live_counter_accepts_silence_without_counting() {
        let lc = LiveCounter::new(48_000);
        lc.reset(48_000);
        let silence = vec![0.0f32; 48_000];
        lc.push(&silence);
        assert_eq!(lc.count(), 0);
    }

    #[test]
    fn live_counter_reset_clears_state() {
        let lc = LiveCounter::new(48_000);
        lc.reset(48_000);
        let tone = vec![0.5f32; 480];
        lc.push(&tone);
        lc.reset(44_100);
        assert_eq!(lc.count(), 0);
        assert_eq!(lc.sample_rate(), 44_100);
    }

    #[test]
    fn failed_start_keeps_the_audio_thread_alive() {
        // Without a usable microphone every start must fail with a real
        // reason, again and again - never with "thread is not running".
        let state = AudioState::spawn().expect("spawn");
        for _ in 0..3 {
            match state.start_live() {
                Ok(()) => {
                    // a real microphone exists on this machine: just stop
                    let _ = state.stop();
                    return;
                }
                Err(e) => assert!(!e.contains("not running"), "{e}"),
            }
        }
    }

    #[test]
    fn keep_caps_the_buffer() {
        let b = Mutex::new(Vec::new());
        keep(&b, &[1.0; 6], 10);
        keep(&b, &[2.0; 6], 10);
        keep(&b, &[3.0; 6], 10);
        let v = b.lock().unwrap();
        assert_eq!(v.len(), 10);
        assert_eq!(v[5], 1.0);
        assert_eq!(v[9], 2.0);
    }

    #[test]
    fn view_is_empty_before_reset() {
        let lc = LiveCounter::new(48_000);
        let v = lc.view();
        assert_eq!(v.count, 0);
        assert_eq!(v.state, "idle");
        assert!(!v.speaking);
    }

    #[test]
    fn finish_before_reset_returns_none() {
        let lc = LiveCounter::new(48_000);
        assert!(lc.finish().is_none());
    }

    #[test]
    fn finish_is_idempotent_after_mark_finished() {
        let lc = LiveCounter::new(48_000);
        lc.reset(48_000);
        lc.push(&vec![0.0f32; 48_000]);
        lc.mark_finished();
        let a = lc.finish();
        let b = lc.finish();
        assert_eq!(a.map(|r| r.count), b.map(|r| r.count));
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
