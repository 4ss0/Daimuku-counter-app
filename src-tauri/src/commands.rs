use crate::audio::{self, AudioState, RecordedAudio};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct RecordingSummary {
    pub samples_count: usize,
    pub duration_ms: u64,
    pub sample_rate: u32,
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<audio::DeviceInfo>, String> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>) -> Result<(), String> {
    state.start()
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AudioState>) -> Result<RecordingSummary, String> {
    let rx = state.stop()?;
    let audio = rx
        .await
        .map_err(|e| format!("audio thread dropped reply: {e}"))?;
    Ok(summary_of(audio))
}

fn summary_of(audio: RecordedAudio) -> RecordingSummary {
    let samples_count = audio.samples.len();
    let duration_ms = if audio.sample_rate > 0 {
        (samples_count as u64 * 1000) / audio.sample_rate as u64
    } else {
        0
    };
    RecordingSummary {
        samples_count,
        duration_ms,
        sample_rate: audio.sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_computes_duration() {
        // 48000 samples at 48000 Hz = 1000 ms
        let audio = RecordedAudio {
            samples: vec![0.0; 48000],
            sample_rate: 48000,
        };
        let summary = summary_of(audio);
        assert_eq!(summary.samples_count, 48000);
        assert_eq!(summary.duration_ms, 1000);
        assert_eq!(summary.sample_rate, 48000);
    }

    #[test]
    fn summary_handles_zero_sample_rate() {
        let audio = RecordedAudio {
            samples: vec![],
            sample_rate: 0,
        };
        let summary = summary_of(audio);
        assert_eq!(summary.samples_count, 0);
        assert_eq!(summary.duration_ms, 0);
    }
}