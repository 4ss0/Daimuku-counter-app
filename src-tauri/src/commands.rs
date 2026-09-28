use crate::audio::{self, AudioState, LiveView, RecordedAudio};
use crate::dsp::{count_daimoku_with_profile, DaimokuCountResult, StreamingState};
use crate::history::{started_before_now, History, SessionRecord};
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

// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn live_status(
    state: State<'_, AudioState>,
    profile_state: State<'_, ProfileState>,
) -> Result<LiveStatus, String> {
    // Only arms the model for the *next* reset: never disturbs a session
    // that is already counting.
    state.live_set_profile(Some(profile_state.snapshot()));

    let s = state.live_state();
    Ok(LiveStatus {
        count: state.live_count(),
        state: streaming_state_str(s).to_string(),
        period_ms: state.live_period_secs().map(|p| p * 1000.0),
        sample_rate: state.live_sample_rate(),
        profile_used: true,
    })
}

// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn live_snapshot(
    state: State<'_, AudioState>,
    profile_state: State<'_, ProfileState>,
) -> Result<Option<DaimokuCountResult>, String> {
    state.live_set_profile(Some(profile_state.snapshot()));
    Ok(state.live_finish())
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
// Live counting session (what an end user does: press start, chant, stop)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LiveSessionSummary {
    pub count: usize,
    pub duration_secs: f32,
    pub period_ms: Option<f32>,
    /// Seconds of audio kept for "save WAV" (the start of the session).
    pub saved_audio_secs: f32,
    /// The saved session (for statistics), if at least one Daimoku was
    /// counted. Its `count` can be corrected with `update_session_count`.
    pub session: Option<SessionRecord>,
}

/// Starts counting with the current personal profile. Unlike
/// `start_recording`, the audio does not end up in the training list.
// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn start_live_session(
    audio_state: State<'_, AudioState>,
    profile_state: State<'_, ProfileState>,
) -> Result<(), String> {
    audio_state.live_set_profile(Some(profile_state.snapshot()));
    audio_state.start_live()
}

#[tauri::command]
pub async fn stop_live_session(
    audio_state: State<'_, AudioState>,
    history: State<'_, History>,
) -> Result<LiveSessionSummary, String> {
    let rx = audio_state.stop()?;
    let audio = rx
        .await
        .map_err(|e| format!("audio thread dropped reply: {e}"))?;
    let view = audio_state.live_view();
    let saved_audio_secs = if audio.sample_rate > 0 {
        audio.samples.len() as f32 / audio.sample_rate as f32
    } else {
        0.0
    };
    if !audio.samples.is_empty() {
        audio_state.set_last_live(audio);
    }
    let session = if view.count > 0 {
        let n = view.count as u32;
        Some(history.add(started_before_now(view.elapsed_secs), view.elapsed_secs, n, n, false)?)
    } else {
        None
    };
    Ok(LiveSessionSummary {
        count: view.count,
        duration_secs: view.elapsed_secs,
        period_ms: view.period_ms,
        saved_audio_secs,
        session,
    })
}

/// Everything the live screen needs; cheap, poll it every ~150 ms.
#[tauri::command]
pub fn live_view(state: State<'_, AudioState>) -> LiveView {
    state.live_view()
}

/// Saves the last live session's audio to the Desktop (or home folder),
/// to send it for analysis when the count was wrong.
#[tauri::command]
pub fn export_live_session_wav(state: State<'_, AudioState>) -> Result<String, String> {
    let audio = state
        .last_live()
        .ok_or_else(|| "nessuna sessione live registrata".to_string())?;
    let dir = crate::paths::export_dir()
        .ok_or_else(|| "cannot determine output directory".to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = dir.join(format!("daimuku-live-{stamp}.wav"));
    write_wav(&path, &audio)?;
    Ok(path.to_string_lossy().into_owned())
}

// -----------------------------------------------------------------------------
// Session history (statistics) and daily goal
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn list_sessions(history: State<'_, History>) -> Vec<SessionRecord> {
    history.list()
}

/// Corrects the credited count of a saved session.
#[tauri::command]
pub fn update_session_count(
    history: State<'_, History>,
    id: u64,
    count: u32,
) -> Result<SessionRecord, String> {
    history.set_count(id, count)
}

#[tauri::command]
pub fn delete_session(history: State<'_, History>, id: u64) -> Result<(), String> {
    history.delete(id)
}

/// Daimoku chanted without the counter (e.g. at a meeting).
#[tauri::command]
pub fn add_manual_session(history: State<'_, History>, count: u32) -> Result<SessionRecord, String> {
    history.add_manual(count)
}

#[tauri::command]
pub fn get_daily_goal(history: State<'_, History>) -> u32 {
    history.daily_goal()
}

#[tauri::command]
pub fn set_daily_goal(history: State<'_, History>, goal: u32) -> Result<u32, String> {
    history.set_daily_goal(goal)
}

// -----------------------------------------------------------------------------
// Personal profile
// -----------------------------------------------------------------------------

// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn get_personal_profile(state: State<'_, ProfileState>) -> Result<PersonalProfile, String> {
    Ok(state.snapshot())
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub expected: u32,
    pub detected: usize,
    pub ok: bool,
    pub profile_used: bool,
    pub analysis: DaimokuCountResult,
}

// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn validate_training_take(
    store: State<'_, TrainingStore>,
    profile_state: State<'_, ProfileState>,
    index: usize,
) -> Result<ValidationResult, String> {
    let (audio, expected) = store.get_with_expected(index)?;

    let snap = profile_state.snapshot();
    let analysis = count_daimoku_with_profile(&audio.samples, audio.sample_rate, Some(&snap))
        .ok_or_else(|| {
            "Nessun audio utilizzabile nella registrazione (troppo breve o silenziosa).".to_string()
        })?;

    let ok = analysis.count == expected as usize;
    Ok(ValidationResult {
        expected,
        detected: analysis.count,
        ok,
        profile_used: true,
        analysis,
    })
}

/// Adds a recorded take to the personal profile. `ProfileState::add_take`
/// persists the audio + index to disk and retrains the model in one step,
/// so there is no separate save call.
// async: may wait for the voice model to finish loading
#[tauri::command(async)]
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

    profile_state.add_take(&audio.samples, audio.sample_rate, expected as u32)?;
    let new_profile = profile_state.snapshot();
    audio_state.live_set_profile(Some(new_profile.clone()));

    Ok(new_profile)
}

/// Deletes one of the user's takes (by its `id`) and retrains.
// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn delete_profile_take(
    profile_state: State<'_, ProfileState>,
    audio_state: State<'_, AudioState>,
    id: usize,
) -> Result<PersonalProfile, String> {
    profile_state.delete_take(id)?;
    let p = profile_state.snapshot();
    audio_state.live_set_profile(Some(p.clone()));
    Ok(p)
}

// -----------------------------------------------------------------------------
// App preferences (language, theme, colour): an opaque JSON object owned by
// the frontend, stored next to the history.
// -----------------------------------------------------------------------------

#[tauri::command]
pub fn get_prefs() -> serde_json::Value {
    crate::paths::data_dir()
        .and_then(|d| std::fs::read_to_string(d.join("prefs.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

#[tauri::command]
pub fn set_prefs(prefs: serde_json::Value) -> Result<(), String> {
    if !prefs.is_object() {
        return Err("prefs must be an object".to_string());
    }
    let dir = crate::paths::data_dir().ok_or_else(|| "no data directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("prefs.json.tmp");
    std::fs::write(&tmp, prefs.to_string()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join("prefs.json")).map_err(|e| e.to_string())
}

/// Deletes the user's takes and falls back to the model trained on the
/// four built-in reference recordings only.
// async: may wait for the voice model to finish loading
#[tauri::command(async)]
pub fn clear_personal_profile(
    profile_state: State<'_, ProfileState>,
    audio_state: State<'_, AudioState>,
) -> Result<(), String> {
    profile_state.clear()?;
    audio_state.live_set_profile(Some(profile_state.snapshot()));
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
    let dir = crate::paths::export_dir()
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
