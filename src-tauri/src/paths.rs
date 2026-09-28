//! Where the app keeps its files.
//!
//! On Windows/macOS/Linux this is the per-user data folder
//! (`%APPDATA%\daimuku-counter` on Windows), unchanged from earlier
//! versions so existing profiles are kept. On Android and iOS that folder
//! does not exist: `lib.rs` sets the app's private data directory, given
//! by Tauri, before anything is loaded.

use std::path::PathBuf;
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Overrides the data directory. Must be called before the profile or the
/// session history are loaded; later calls are ignored.
pub fn set_data_dir(dir: PathBuf) {
    let _ = DATA_DIR.set(dir);
}

pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = DATA_DIR.get() {
        return Some(d.clone());
    }
    Some(dirs::data_dir()?.join("daimuku-counter"))
}

/// Folder for files the user saves on purpose (WAV of a session).
/// Desktop: the Desktop (or home). Mobile: `<app data>/exports`.
pub fn export_dir() -> Option<PathBuf> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let d = data_dir()?.join("exports");
        let _ = std::fs::create_dir_all(&d);
        Some(d)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        dirs::desktop_dir().or_else(dirs::home_dir)
    }
}
