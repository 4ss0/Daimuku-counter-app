//! What the Android side reads from the engine by itself (JNI), without
//! going through the web page: with the screen locked the page's timers are
//! slowed down to one a minute, so the counting service asks for the count
//! directly (`Native.kt`, `CountingService.kt`).

use crate::audio::LiveCounter;
use std::sync::{Arc, OnceLock};

static LIVE: OnceLock<Arc<LiveCounter>> = OnceLock::new();

/// The live counter the app uses (set once, at start).
pub fn set_live(live: Arc<LiveCounter>) {
    let _ = LIVE.set(live);
}

/// The Daimoku counted so far by the live counter, or -1 before it exists.
pub fn live_count() -> i32 {
    LIVE.get()
        .map(|l| l.count().min(i32::MAX as usize) as i32)
        .unwrap_or(-1)
}

/// `com.daimuku.counter.Native.liveCount()`
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_daimuku_counter_Native_liveCount(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> i32 {
    // a panic must not cross into Java
    std::panic::catch_unwind(live_count).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_once_set() {
        let live = Arc::new(LiveCounter::new(48_000));
        set_live(Arc::clone(&live));
        assert_eq!(live_count(), 0);
    }
}
