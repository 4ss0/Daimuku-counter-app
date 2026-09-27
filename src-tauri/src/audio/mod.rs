use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use serde::Serialize;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::oneshot;

use crate::dsp::{count_daimoku_with_profile, DaimokuCountResult, StreamingState};
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
// Live counter (incremental batch)
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
    samples: Vec<f32>,
    sample_rate: u32,
    profile: Option<PersonalProfile>,
    profile_fp: u64,
    cached: Option<DaimokuCountResult>,
    last_compute_len: usize,
    computed_once: bool,
}

const MIN_NEW_SECS_F32: f32 = 0.5;

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
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.inner.lock().map(|g| g.sample_rate).unwrap_or(48_000)
    }

    pub fn snapshot(&self) -> LiveSnapshot {
        let (to_compute, sr, cached, profile) = {
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
                )
            } else {
                (None, g.sample_rate, g.cached.clone(), g.profile.clone())
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
                }
            }
        }

        let count = result.as_ref().map(|r| r.count).unwrap_or(0);
        let period_ms = result.as_ref().map(|r| r.period_ms).filter(|p| *p > 0.0);
        let samples_len = self.inner.lock().map(|g| g.samples.len()).unwrap_or(0);
        let state = if samples_len < (sr as usize / 2) {
            StreamingState::Warming
        } else if count == 0 {
            StreamingState::Idle
        } else if count >= 2 {
            StreamingState::Locked
        } else {
            StreamingState::Warming
        };

        LiveSnapshot {
            count,
            state,
            period_ms,
            sample_rate: sr,
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
        let (samples, sr, profile) = {
            let g = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return None,
            };
            if g.samples.is_empty() {
                return None;
            }
            (g.samples.clone(), g.sample_rate, g.profile.clone())
        };
        count_daimoku_with_profile(&samples, sr, profile.as_ref())
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