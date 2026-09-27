pub mod audio;
pub mod commands;
pub mod dsp;
pub mod session;
pub mod training;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_state = audio::AudioState::spawn().expect("failed to spawn audio engine");
    let training_store = training::TrainingStore::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(audio_state)
        .manage(training_store)
        .invoke_handler(tauri::generate_handler![
            // devices & recording
            commands::list_input_devices,
            commands::start_recording,
            commands::stop_recording,
            // live counter
            commands::live_status,
            commands::live_snapshot,
            commands::reset_live_counter,
            // training store
            commands::list_training_takes,
            commands::clear_training_takes,
            commands::export_training_wav,
            commands::validate_training_take,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}