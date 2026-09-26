use crate::audio::{self, AudioState};
use crate::training::{TrainingStore, TrainingTakeMeta};
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