//! The app's recognition modules, compiled from src-tauri/src.
#[path = "../../../src-tauri/src/base.rs"]
pub mod base;
#[path = "../../../src-tauri/src/engine.rs"]
pub mod engine;
#[path = "../../../src-tauri/src/features.rs"]
pub mod features;
#[path = "../../../src-tauri/src/hmm.rs"]
pub mod hmm;
#[path = "../../../src-tauri/src/model.rs"]
pub mod model;
#[path = "../../../src-tauri/src/train.rs"]
pub mod train;
/// The previous engine (v0.4), kept only for comparison.
pub mod engine_old;
