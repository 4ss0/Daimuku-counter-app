package com.daimuku.counter

/**
 * What the app reads from the Rust engine directly (src-tauri/src/native.rs),
 * without going through the web page.
 */
object Native {
  // already loaded by the activity: loading it again does nothing
  private val loaded = try {
    System.loadLibrary("daimuku_counter_lib")
    true
  } catch (e: Throwable) {
    false
  }

  @JvmStatic private external fun liveCount(): Int

  /** The Daimoku counted so far in the running session, or -1 if unknown. */
  fun count(): Int =
    if (!loaded) -1
    else try {
      liveCount()
    } catch (e: Throwable) {
      -1
    }
}
