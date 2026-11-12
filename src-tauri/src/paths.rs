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

#[cfg(test)]
pub fn data_dir() -> Option<PathBuf> {
    // Never use installed application data in tests, on any OS.
    thread_local! {
        static TEST_DIR: PathBuf = {
            let root = std::env::temp_dir().join(format!("daimoku-tests-{}", std::process::id()));
            root.join(format!("{:?}", std::thread::current().id()).replace(':', "_"))
        };
    }
    Some(TEST_DIR.with(Clone::clone))
}

#[cfg(not(test))]
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = DATA_DIR.get() {
        return Some(d.clone());
    }
    Some(dirs::data_dir()?.join("daimuku-counter"))
}

/// Folder for files the user saves on purpose (WAV of a session).
/// Desktop: the Desktop (or home). Mobile: `<app data>/exports`.
#[cfg(test)]
pub fn export_dir() -> Option<PathBuf> {
    let dir = data_dir()?.join("exports");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

#[cfg(not(test))]
pub fn export_dir() -> Option<PathBuf> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let d = data_dir()?.join("exports");
        let _ = std::fs::create_dir_all(&d);
        prune_old(&d);
        Some(d)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        dirs::desktop_dir().or_else(dirs::home_dir)
    }
}

/// On mobile the exports folder is only a hand-over point to the system
/// "save"/"share" screens: files older than a day are removed.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn prune_old(dir: &std::path::Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let limit = std::time::Duration::from_secs(24 * 3600);
    for e in rd.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > limit);
        if old {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn application_data_is_isolated_even_on_windows() {
        let actual = super::data_dir().unwrap();
        assert!(actual.starts_with(std::env::temp_dir()));
        assert_ne!(Some(actual.clone()), dirs::data_dir().map(|p| p.join("daimuku-counter")));
        assert_eq!(actual, super::data_dir().unwrap());
        let other = std::thread::spawn(|| super::data_dir().unwrap()).join().unwrap();
        assert_ne!(actual, other);
    }
}
