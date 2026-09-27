use crate::audio::{self, AudioState, RecordedAudio};
use crate::dsp::{detect_onsets, OnsetConfig, OnsetInfo};
use crate::learning::{learn_template, DaimokuTemplate};
use crate::training::{TrainingStore, TrainingTakeMeta};
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<audio::DeviceInfo>, String> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn start_recording(state: State<'_, AudioState>) -> Result<(), String> {
    state.start()
}

#[tauri::command]
pub async fn stop_recording(
    audio_state: State<'_, AudioState>,
    training_store: State<'_, TrainingStore>,
    expected_daimoku_count: u32,
) -> Result<TrainingTakeMeta, String> {
    let rx = audio_state.stop()?;
    let audio = rx
        .await
        .map_err(|e| format!("audio thread dropped reply: {e}"))?;
    Ok(training_store.add(audio, expected_daimoku_count))
}

#[tauri::command]
pub fn list_training_takes(store: State<'_, TrainingStore>) -> Vec<TrainingTakeMeta> {
    store.list()
}

#[tauri::command]
pub fn clear_training_takes(store: State<'_, TrainingStore>) -> usize {
    store.clear()
}

#[tauri::command]
pub fn export_training_wav(
    store: State<'_, TrainingStore>,
    index: usize,
) -> Result<String, String> {
    let audio = store.get(index)?;
    let dir = dirs::desktop_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "cannot determine output directory".to_string())?;
    let path = dir.join(format!("daimuku-training-{index}.wav"));
    write_wav(&path, &audio)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn debug_detect_onsets(
    store: State<'_, TrainingStore>,
    index: usize,
) -> Result<OnsetInfo, String> {
    let audio = store.get(index)?;
    Ok(detect_onsets(&audio.samples, audio.sample_rate, OnsetConfig::default()))
}

#[derive(Debug, Serialize)]
pub struct LearnResult {
    pub onsets: OnsetInfo,
    pub template: DaimokuTemplate,
}

#[tauri::command]
pub fn learn_template_from_take(
    store: State<'_, TrainingStore>,
    index: usize,
) -> Result<LearnResult, String> {
    let (audio, expected) = store.get_with_expected(index)?;
    let onsets = detect_onsets(&audio.samples, audio.sample_rate, OnsetConfig::default());
    let template = learn_template(&onsets.ioi_ms, expected)?;
    Ok(LearnResult { onsets, template })
}

fn write_wav(path: &Path, audio: &RecordedAudio) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("failed to create wav file: {e}"))?;
    for &s in &audio.samples {
        writer
            .write_sample(s)
            .map_err(|e| format!("failed to write sample: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("failed to finalize wav file: {e}"))?;
    Ok(())
}