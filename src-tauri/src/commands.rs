use crate::audio::{self, AudioState, RecordedAudio};
use crate::dsp::{count_daimoku_with_profile, DaimokuCountResult, StreamingState};
use crate::profile::{PersonalProfile, ProfileState};
use crate::training::{TrainingStore, TrainingTakeMeta};
use serde::Serialize;
use std::path::Path;
use tauri::State;

// -----------------------------------------------------------------------------
// Devices & recording
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// Live counter
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LiveStatus {
    pub count: usize,
    pub state: String,
    pub period_ms: Option<f32>,
    pub sample_rate: u32,
    pub profile_used: bool,
}

#[tauri::command]
pub fn live_status(
    state: State<'_, AudioState>,
    profile_state: State<'_, ProfileState>,
) -> LiveStatus {
    let snap = profile_state.snapshot();
    let usable = snap.is_usable();
    state.live_set_profile(if usable { Some(snap) } else { None });

    let s = state.live_state();
    LiveStatus {
        count: state.live_count(),
        state: streaming_state_str(s).to_string(),
        period_ms: state.live_period_secs().map(|p| p * 1000.0),
        sample_rate: state.live_sample_rate(),
        profile_used: usable,
    }
}

#[tauri::command]
pub fn live_snapshot(
    state: State<'_, AudioState>,
    profile_state: State<'_, ProfileState>,
) -> Option<DaimokuCountResult> {
    let snap = profile_state.snapshot();
    state.live_set_profile(if snap.is_usable() { Some(snap) } else { None });
    state.live_finish()
}

#[tauri::command]
pub fn reset_live_counter(
    state: State<'_, AudioState>,
    sample_rate: u32,
) -> Result<(), String> {
    if sample_rate == 0 {
        return Err("sample_rate must be > 0".to_string());
    }
    state.live_reset(sample_rate);
    Ok(())
}

fn streaming_state_str(s: StreamingState) -> &'static str {
    match s {
        StreamingState::Warming => "warming",
        StreamingState::Locked => "locked",
        StreamingState::Idle => "idle",
    }
}

// -----------------------------------------------------------------------------
// Personal profile
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn get_personal_profile(state: State<'_, ProfileState>) -> PersonalProfile {
    state.snapshot()
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub expected: u32,
    pub detected: usize,
    pub ok: bool,
    pub profile_used: bool,
    pub analysis: DaimokuCountResult,
}

#[tauri::command]
pub fn validate_training_take(
    store: State<'_, TrainingStore>,
    profile_state: State<'_, ProfileState>,
    index: usize,
) -> Result<ValidationResult, String> {
    let (audio, expected) = store.get_with_expected(index)?;

    let snap = profile_state.snapshot();
    let profile_used = snap.is_usable();
    let profile_ref = if profile_used { Some(&snap) } else { None };

    let analysis = count_daimoku_with_profile(&audio.samples, audio.sample_rate, profile_ref)
        .ok_or_else(|| {
            "No periodicity detected in the recording. Recite with clear syllables and steady rhythm."
                .to_string()
        })?;

    let ok = analysis.count == expected as usize;
    Ok(ValidationResult {
        expected,
        detected: analysis.count,
        ok,
        profile_used,
        analysis,
    })
}

#[tauri::command]
pub fn add_take_to_profile(
    store: State<'_, TrainingStore>,
    profile_state: State<'_, ProfileState>,
    audio_state: State<'_, AudioState>,
    index: usize,
) -> Result<PersonalProfile, String> {
    let (audio, expected) = store.get_with_expected(index)?;
    if expected == 0 {
        return Err("take has no declared Daimoku count".to_string());
    }

    let new_profile = profile_state.with_mut(|p| {
        p.add_take(&audio.samples, audio.sample_rate, expected)
            .map(|_| ())
            .map_err(|e| e.to_string())
            .and_then(|_| p.save())
            .map(|_| p.clone())
    })?;

    let to_inject = if new_profile.is_usable() {
        Some(new_profile.clone())
    } else {
        None
    };
    audio_state.live_set_profile(to_inject);

    Ok(new_profile)
}

#[tauri::command]
pub fn clear_personal_profile(
    profile_state: State<'_, ProfileState>,
    audio_state: State<'_, AudioState>,
) -> Result<(), String> {
    profile_state.with_mut(|p| {
        p.clear();
        let _ = p.save();
    });
    audio_state.live_set_profile(None);
    Ok(())
}

// -----------------------------------------------------------------------------
// Training store
// -----------------------------------------------------------------------------

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

// -----------------------------------------------------------------------------
// WAV export helper
// -----------------------------------------------------------------------------

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