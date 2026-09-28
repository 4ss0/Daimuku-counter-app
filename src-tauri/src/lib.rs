pub mod audio;
pub mod backup;
pub mod base;
pub mod commands;
pub mod dsp;
pub mod engine;
pub mod features;
pub mod history;
pub mod hmm;
pub mod model;
pub mod paths;
pub mod profile;
pub mod session;
pub mod train;
pub mod training;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // On Android/iOS the usual per-user data folder does not
            // exist: use the app's private folder. Must happen before the
            // profile and the history are loaded.
            #[cfg(mobile)]
            {
                if let Ok(dir) = app.path().app_data_dir() {
                    paths::set_data_dir(dir);
                }
            }

            // Everything the UI may call is registered right away; the
            // voice model (slow to build) is loaded in the background and
            // commands that need it simply wait for it.
            app.manage(history::History::load());
            app.manage(training::TrainingStore::new());
            app.manage(profile::ProfileState::empty());
            app.manage(audio::AudioState::spawn()?);

            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("profile-loader".into())
                .spawn(move || {
                    let profile = handle.state::<profile::ProfileState>();
                    profile.load();
                    handle
                        .state::<audio::AudioState>()
                        .live_set_profile(Some(profile.snapshot()));
                })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // devices & recording (voice training)
            commands::list_input_devices,
            commands::start_recording,
            commands::stop_recording,
            // live counter (legacy polling API)
            commands::live_status,
            commands::live_snapshot,
            commands::reset_live_counter,
            // live counting session
            commands::start_live_session,
            commands::stop_live_session,
            commands::live_view,
            commands::export_live_session_wav,
            // history & statistics
            commands::list_sessions,
            commands::update_session_count,
            commands::delete_session,
            commands::add_manual_session,
            commands::get_daily_goal,
            commands::set_daily_goal,
            // personal profile
            commands::get_personal_profile,
            commands::add_take_to_profile,
            commands::clear_personal_profile,
            commands::delete_profile_take,
            // preferences
            commands::get_prefs,
            commands::set_prefs,
            // backup & exports
            commands::create_backup,
            commands::restore_backup,
            commands::write_text_export,
            commands::validate_training_take,
            // training store
            commands::list_training_takes,
            commands::clear_training_takes,
            commands::export_training_wav,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
