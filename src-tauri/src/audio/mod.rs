use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use serde::Serialize;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::oneshot;

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

/// Handle to the audio engine. Holds only `Send + Sync` types, so it can be
/// stored inside Tauri's shared state. The actual `cpal::Stream` lives on a
/// dedicated thread and is never touched from here.
pub struct AudioState {
    cmd_tx: Mutex<Sender<AudioCommand>>,
}

impl AudioState {
    pub fn spawn() -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || audio_thread_main(cmd_rx))
            .map_err(|e| format!("failed to spawn audio thread: {e}"))?;
        Ok(Self {
            cmd_tx: Mutex::new(cmd_tx),
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
}

fn audio_thread_main(cmd_rx: mpsc::Receiver<AudioCommand>) {
    let mut current: Option<ActiveRecording> = None;

    for cmd in cmd_rx {
        match cmd {
            AudioCommand::Start => {
                if current.is_some() {
                    eprintln!("[audio] start ignored: already recording");
                    continue;
                }
                match ActiveRecording::start() {
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

/// A running recording. Owns the stream; dropping it stops the capture.
struct ActiveRecording {
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    _stream: cpal::Stream,
}

fn on_stream_error(err: cpal::StreamError) {
    eprintln!("[audio] stream error: {err}");
}

/// Downmix an interleaved multi-channel frame buffer to mono by averaging
/// channels. Allocates a fresh Vec; the caller is responsible for ownership.
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
    fn start() -> Result<Self, String> {
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

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        let stream = match sample_format {
            SampleFormat::F32 => {
                let buf = Arc::clone(&buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let mono = downmix_to_mono(data, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
                    },
                    on_stream_error,
                    None,
                )
            }
            SampleFormat::I16 => {
                let buf = Arc::clone(&buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let as_f32: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        let mono = downmix_to_mono(&as_f32, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
                    },
                    on_stream_error,
                    None,
                )
            }
            SampleFormat::U16 => {
                let buf = Arc::clone(&buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let as_f32: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 - 32768.0) / 32768.0)
                            .collect();
                        let mono = downmix_to_mono(&as_f32, channels);
                        buf.lock().unwrap().extend_from_slice(&mono);
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
        // Dropping the stream stops the capture; the audio callback is
        // guaranteed not to run anymore after this point.
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
        // Two interleaved stereo frames: (L=1.0, R=3.0), (L=-1.0, R=1.0)
        let stereo = [1.0_f32, 3.0, -1.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono, vec![2.0, 0.0]);

        // Mono input must pass through unchanged.
        let mono_in = [0.5_f32, -0.25];
        assert_eq!(downmix_to_mono(&mono_in, 1), vec![0.5, -0.25]);
    }

    /// Ignored by default because it requires a working microphone.
    /// Run with: cargo test -- --ignored --nocapture
    #[test]
    #[ignore]
    fn records_real_audio() {
        let state = AudioState::spawn().expect("failed to spawn audio engine");
        state.start().expect("failed to start");

        let elapsed_secs = 2.0_f32;
        std::thread::sleep(Duration::from_millis((elapsed_secs * 1000.0) as u64));

        let rx = state.stop().expect("failed to stop");
        let audio = block_on(rx);

        assert!(audio.sample_rate > 0, "sample rate is zero");
        assert!(
            !audio.samples.is_empty(),
            "no samples captured from the microphone"
        );

        let audio_secs = audio.samples.len() as f32 / audio.sample_rate as f32;
        // WASAPI shared mode can spend 300+ ms warming up the stream. Assert
        // that we captured a plausible amount of audio, and reject any value
        // that would indicate a channel or sample-rate mismatch.
        assert!(
            (1.0..=elapsed_secs + 0.2).contains(&audio_secs),
            "captured {audio_secs:.2}s in {elapsed_secs:.2}s of wall time — \
             stream may have stalled or the config is wrong"
        );
        println!(
            "captured {} samples at {} Hz = {:.2}s (wall time {:.2}s)",
            audio.samples.len(),
            audio.sample_rate,
            audio_secs,
            elapsed_secs
        );
    }

    /// Minimal blocking wait for a oneshot receiver without pulling in
    /// a full tokio runtime at the call site.
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