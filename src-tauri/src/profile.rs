//! Personal profile: the trained Nam-myoho-renge-kyo model, bootstrapped
//! from the 4 shipped base recordings and refined by the user's own
//! takes (any number, tagged only with how many Daimoku each contains -
//! the app's UI is what asks the user for a slow/medium/fast one).
//!
//! Persistence: each promoted take's raw audio is kept as a small WAV
//! file under the app data dir, plus a lightweight JSON index. The model
//! itself is never persisted directly - it is always *retrained from the
//! source audio* on load (base clips + every stored take), which is fast
//! (tens of milliseconds even with a dozen takes) and sidesteps any risk
//! of a stale/incompatible serialized model surviving an app update.

use crate::base::{decode_wav, BASE_CLIPS};
use crate::model::Model;
use crate::train::{align_item, build_item, train_model, TrainItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard};

/// Base clips count for roughly as much as one ordinary take; the user's
/// own recordings count for more, so the model leans towards their voice
/// once they have provided some.
const BASE_WEIGHT: f32 = 1.0;
const USER_WEIGHT: f32 = 2.5;
const MIN_TAKE_SECS: f32 = 0.5;

// ===========================================================================
// Data model
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeRecord {
    pub id: usize,
    pub n_daimoku: u32,
    pub duration_secs: f32,
    /// Median time from one Daimoku to the next in this take, once
    /// aligned against the current model. `None` until the first
    /// successful retrain.
    pub period_ms: Option<f32>,
    pub created_at: DateTime<Utc>,
}

/// What actually gets written to disk (audio lives in sibling .wav files).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileIndex {
    version: u32,
    takes: Vec<TakeRecord>,
}

/// Snapshot handed to the rest of the app: the trained model plus
/// display-friendly metadata. Cheap to clone (a few hundred floats).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalProfile {
    pub version: u32,
    pub takes: Vec<TakeRecord>,
    pub base_clip_count: usize,
    pub ref_llr: f32,
    pub min_cycle_ms: f32,
    pub max_cycle_ms: f32,
    pub model: Model,
}

impl PersonalProfile {
    /// Always true: the 4 shipped base recordings alone are enough to
    /// build a working model, before the user has recorded anything.
    pub fn is_usable(&self) -> bool {
        true
    }

    pub fn n_takes(&self) -> usize {
        self.takes.len()
    }
}

// ===========================================================================
// Persistence paths
// ===========================================================================

fn profile_dir() -> Option<PathBuf> {
    crate::paths::data_dir()
}
fn index_path() -> Option<PathBuf> {
    Some(profile_dir()?.join("profile_index.json"))
}
fn takes_dir() -> Option<PathBuf> {
    Some(profile_dir()?.join("takes"))
}
fn take_wav_path(id: usize) -> Option<PathBuf> {
    Some(takes_dir()?.join(format!("take-{id}.wav")))
}

fn write_wav_f32(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("wav create: {e}"))?;
    for &s in samples {
        w.write_sample(s).map_err(|e| format!("wav write: {e}"))?;
    }
    w.finalize().map_err(|e| format!("wav finalize: {e}"))
}

fn read_index() -> ProfileIndex {
    let Some(path) = index_path() else {
        return ProfileIndex::default();
    };
    match fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(idx) => idx,
            Err(e) => {
                eprintln!(
                    "[profile] could not parse {}: {e} - starting from an empty profile \
                     (the file was kept as .bak).",
                    path.display()
                );
                let _ = fs::rename(&path, path.with_extension("json.bak"));
                ProfileIndex::default()
            }
        },
        Err(_) => ProfileIndex::default(),
    }
}

fn write_index(idx: &ProfileIndex) -> Result<(), String> {
    let path = index_path().ok_or_else(|| "cannot determine data directory".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(idx).map_err(|e| format!("serialize: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &json).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename into {}: {e}", path.display()))
}

// ===========================================================================
// State holder
// ===========================================================================

struct ProfileInner {
    index: ProfileIndex,
    base_items: Vec<TrainItem>,
    /// Same order as `index.takes`.
    user_items: Vec<TrainItem>,
    model: Model,
}

/// Holds the trained model. Loading it means re-analysing the base clips
/// and every stored take, which takes a moment (seconds on a phone in a
/// debug build), so the app registers an *empty* state immediately and
/// fills it from a background thread (`load`). Every accessor waits until
/// the model is ready instead of failing.
pub struct ProfileState {
    inner: Mutex<Option<ProfileInner>>,
    ready: Condvar,
}

fn load_inner() -> ProfileInner {
    let base_items = load_base_items();
    let index = read_index();
    let mut user_items = Vec::with_capacity(index.takes.len());
    let mut kept_takes = Vec::with_capacity(index.takes.len());
    for t in &index.takes {
        let Some(path) = take_wav_path(t.id) else { continue };
        match fs::read(&path).ok().and_then(|b| decode_wav(&b).ok()) {
            Some((samples, sr)) => match build_item(&samples, sr, t.n_daimoku as usize, USER_WEIGHT) {
                Some(item) => {
                    user_items.push(item);
                    kept_takes.push(t.clone());
                }
                None => eprintln!("[profile] take {} could not be re-analysed, skipping", t.id),
            },
            None => eprintln!("[profile] missing/corrupt audio for take {}, skipping", t.id),
        }
    }
    let index = ProfileIndex {
        version: 1,
        takes: kept_takes,
    };
    let model = retrain(&base_items, &user_items).unwrap_or_else(base_only_fallback);
    let mut inner = ProfileInner {
        index,
        base_items,
        user_items,
        model,
    };
    recompute_take_periods(&mut inner);
    inner
}

impl ProfileState {
    /// Loads synchronously (tests, desktop tools).
    pub fn new() -> Self {
        let s = Self::empty();
        s.load();
        s
    }

    /// Not loaded yet: call `load` (typically from a background thread).
    pub fn empty() -> Self {
        Self {
            inner: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    /// Loads (or reloads) the model from the base clips and stored takes.
    pub fn load(&self) {
        let inner = load_inner();
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = Some(inner);
        self.ready.notify_all();
    }

    pub fn is_ready(&self) -> bool {
        self.inner.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Locks the state, waiting for the first load to finish.
    fn guard(&self) -> MutexGuard<'_, Option<ProfileInner>> {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.ready
            .wait_while(g, |v| v.is_none())
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn snapshot(&self) -> PersonalProfile {
        let g = self.guard();
        let g = g.as_ref().expect("loaded");
        PersonalProfile {
            version: g.index.version.max(1),
            takes: g.index.takes.clone(),
            base_clip_count: BASE_CLIPS.len(),
            ref_llr: g.model.ref_llr,
            min_cycle_ms: g.model.min_cycle_frames as f32 * 10.0,
            max_cycle_ms: g.model.max_cycle_frames as f32 * 10.0,
            model: g.model.clone(),
        }
    }

    /// Adds a new take (raw audio + how many Daimoku it contains),
    /// persists it, and retrains the model from base + all stored takes.
    /// Returns the new take's id.
    pub fn add_take(&self, samples: &[f32], sample_rate: u32, n_daimoku: u32) -> Result<usize, String> {
        if n_daimoku == 0 {
            return Err("n_daimoku must be >= 1".to_string());
        }
        if sample_rate == 0 {
            return Err("sample_rate must be > 0".to_string());
        }
        let duration_secs = samples.len() as f32 / sample_rate as f32;
        if duration_secs < MIN_TAKE_SECS {
            return Err(format!("take too short ({duration_secs:.2}s, minimum {MIN_TAKE_SECS}s)"));
        }
        let item = build_item(samples, sample_rate, n_daimoku as usize, USER_WEIGHT)
            .ok_or_else(|| "could not find enough voiced audio in this take".to_string())?;

        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        // never reuse an id: its WAV file would be overwritten
        let id = g.index.takes.iter().map(|t| t.id + 1).max().unwrap_or(0);
        if let Some(path) = take_wav_path(id) {
            write_wav_f32(&path, samples, sample_rate)?;
        }
        g.index.takes.push(TakeRecord {
            id,
            n_daimoku,
            duration_secs,
            period_ms: None,
            created_at: Utc::now(),
        });
        g.user_items.push(item);
        g.model = retrain(&g.base_items, &g.user_items).unwrap_or_else(|| g.model.clone());
        recompute_take_periods(g);
        write_index(&g.index)?;
        Ok(id)
    }

    /// Removes one stored take (and its audio) and retrains.
    pub fn delete_take(&self, id: usize) -> Result<(), String> {
        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        let pos = g
            .index
            .takes
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| format!("take {id} not found"))?;
        g.index.takes.remove(pos);
        g.user_items.remove(pos);
        if let Some(path) = take_wav_path(id) {
            let _ = fs::remove_file(path);
        }
        g.model = retrain(&g.base_items, &g.user_items).unwrap_or_else(base_only_fallback);
        recompute_take_periods(g);
        write_index(&g.index)
    }

    /// Every stored take with its audio (for a backup).
    pub fn export_takes(&self) -> Vec<(TakeRecord, Vec<f32>, u32)> {
        let guard = self.guard();
        let g = guard.as_ref().expect("loaded");
        g.index
            .takes
            .iter()
            .filter_map(|t| {
                let bytes = fs::read(take_wav_path(t.id)?).ok()?;
                let (samples, sr) = decode_wav(&bytes).ok()?;
                Some((t.clone(), samples, sr))
            })
            .collect()
    }

    /// Replaces all stored takes (restore from a backup) and retrains.
    /// Takes that cannot be analysed are skipped; returns how many were kept.
    pub fn replace_takes(&self, takes: Vec<(TakeRecord, Vec<f32>, u32)>) -> Result<usize, String> {
        // analyse first, outside the lock: a bad backup leaves everything as it was
        let mut kept: Vec<(TakeRecord, Vec<f32>, u32, TrainItem)> = Vec::new();
        for (rec, samples, sr) in takes {
            if sr == 0 || rec.n_daimoku == 0 || (samples.len() as f32 / sr as f32) < MIN_TAKE_SECS {
                continue;
            }
            if let Some(item) = build_item(&samples, sr, rec.n_daimoku as usize, USER_WEIGHT) {
                kept.push((rec, samples, sr, item));
            }
        }
        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        if let Some(dir) = takes_dir() {
            let _ = fs::remove_dir_all(&dir);
        }
        let mut index = ProfileIndex { version: 1, takes: Vec::new() };
        let mut user_items = Vec::new();
        for (id, (rec, samples, sr, item)) in kept.into_iter().enumerate() {
            if let Some(path) = take_wav_path(id) {
                write_wav_f32(&path, &samples, sr)?;
            }
            index.takes.push(TakeRecord {
                id,
                duration_secs: samples.len() as f32 / sr as f32,
                period_ms: None,
                ..rec
            });
            user_items.push(item);
        }
        g.index = index;
        g.user_items = user_items;
        g.model = retrain(&g.base_items, &g.user_items).unwrap_or_else(base_only_fallback);
        recompute_take_periods(g);
        write_index(&g.index)?;
        Ok(g.index.takes.len())
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        if let Some(dir) = takes_dir() {
            let _ = fs::remove_dir_all(&dir);
        }
        g.index = ProfileIndex { version: 1, takes: Vec::new() };
        g.user_items.clear();
        g.model = retrain(&g.base_items, &g.user_items).unwrap_or_else(base_only_fallback);
        write_index(&g.index)
    }
}

impl Default for ProfileState {
    fn default() -> Self {
        Self::new()
    }
}

/// A model trained on just the 4 shipped base clips, with no user takes.
/// Cheap-ish (retrains from embedded assets, no disk I/O) fallback for
/// callers that don't have a [`PersonalProfile`] on hand.
pub fn base_only_model() -> Model {
    let base = load_base_items();
    retrain(&base, &[]).unwrap_or_else(base_only_fallback)
}

fn load_base_items() -> Vec<TrainItem> {
    let mut items = Vec::with_capacity(BASE_CLIPS.len());
    for c in BASE_CLIPS.iter() {
        match decode_wav(c.wav) {
            Ok((samples, sr)) => match build_item(&samples, sr, c.n_cycles, BASE_WEIGHT) {
                Some(item) => items.push(item),
                None => eprintln!("[profile] base clip '{}' failed to analyse - skipped", c.name),
            },
            Err(e) => eprintln!("[profile] base clip '{}' failed to decode: {e}", c.name),
        }
    }
    items
}

fn retrain(base: &[TrainItem], user: &[TrainItem]) -> Option<Model> {
    let mut all: Vec<TrainItem> = Vec::with_capacity(base.len() + user.len());
    all.extend(base.iter().cloned());
    all.extend(user.iter().cloned());
    train_model(&all, &[], None, 8).map(|t| t.model)
}

/// Cheap model that never recognises anything: used by the live counter
/// until the real one has been loaded.
pub fn placeholder_model() -> Model {
    base_only_fallback()
}

/// Fallback for the pathological case where even the base clips fail to
/// train (e.g. corrupted assets): an unusable-but-safe placeholder that
/// simply never recognises anything, rather than panicking the app.
fn base_only_fallback() -> Model {
    use crate::features::FEAT_DIM;
    use crate::model::{Gmm, MIX, N_CHAIN};
    Model {
        mu: vec![0.0; N_CHAIN * MIX * FEAT_DIM],
        var: vec![1.0; N_CHAIN * MIX * FEAT_DIM],
        log_w: vec![-(MIX as f32).ln(); N_CHAIN * MIX],
        garb: Gmm {
            log_w: vec![0.0],
            mu: vec![0.0; FEAT_DIM],
            var: vec![1.0; FEAT_DIM],
        },
        cep_mean: vec![0.0; 12],
        ref_llr: 100.0, // unreachable threshold: nothing will ever be accepted
        min_cycle_frames: 40,
        max_cycle_frames: 800,
    }
}

fn recompute_take_periods(inner: &mut ProfileInner) {
    use crate::model::Scorer;
    let sc = Scorer::new(&inner.model);
    for (rec, item) in inner.index.takes.iter_mut().zip(inner.user_items.iter()) {
        rec.period_ms = align_item(item, &sc).and_then(|al| {
            let spans = al.cycle_spans(item.n_cycles);
            if spans.is_empty() {
                None
            } else {
                let mut s: Vec<f32> = spans.iter().map(|&x| x as f32 * 10.0).collect();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                Some(s[s.len() / 2])
            }
        });
    }
}

/// Tests that point `dirs::data_dir()` at a private temp dir via an env
/// var must hold this lock: env vars are process-wide, so two such tests
/// running concurrently could otherwise read each other's temp dir.
#[cfg(test)]
pub(crate) static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_usable_from_base_clips_alone() {
        let s = ProfileState::new();
        let snap = s.snapshot();
        assert!(snap.is_usable());
        assert_eq!(snap.base_clip_count, 4);
        assert_eq!(snap.n_takes_helper(), 0);
        assert!(snap.ref_llr > 1.0);
    }

    impl PersonalProfile {
        fn n_takes_helper(&self) -> usize {
            self.takes.len()
        }
    }


    #[test]
    fn add_take_persists_and_updates_model() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        // isolate this test's data dir
        let tmp = std::env::temp_dir().join(format!("daimuku-test-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let _ = std::fs::remove_dir_all(&tmp);

        let s = ProfileState::new();
        let before = s.snapshot();
        assert_eq!(before.n_takes_helper(), 0);

        // Reuse a base clip's audio as a stand-in "user recording".
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        let id = s.add_take(&samples, sr, 1).expect("add_take");
        assert_eq!(id, 0);

        let after = s.snapshot();
        assert_eq!(after.n_takes_helper(), 1);
        assert!(after.takes[0].period_ms.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn delete_single_take() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("daimuku-test-del-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let _ = std::fs::remove_dir_all(&tmp);

        let s = ProfileState::new();
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        let a = s.add_take(&samples, sr, 1).unwrap();
        let b = s.add_take(&samples, sr, 1).unwrap();
        s.delete_take(a).unwrap();
        assert!(s.delete_take(a).is_err());
        let c = s.add_take(&samples, sr, 1).unwrap();
        assert!(c != b, "ids are never reused");
        let ids: Vec<usize> = s.snapshot().takes.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![b, c]);
        // survives a reload from disk
        let s2 = ProfileState::new();
        assert_eq!(s2.snapshot().takes.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn empty_state_waits_for_load() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let s = std::sync::Arc::new(ProfileState::empty());
        assert!(!s.is_ready());
        let s2 = std::sync::Arc::clone(&s);
        let h = std::thread::spawn(move || s2.snapshot().base_clip_count);
        std::thread::sleep(std::time::Duration::from_millis(50));
        s.load();
        assert_eq!(h.join().unwrap(), 4);
    }

    #[test]
    fn clear_resets_to_base_only() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("daimuku-test-clear-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let _ = std::fs::remove_dir_all(&tmp);

        let s = ProfileState::new();
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        s.add_take(&samples, sr, 1).unwrap();
        assert_eq!(s.snapshot().n_takes_helper(), 1);

        s.clear().unwrap();
        assert_eq!(s.snapshot().n_takes_helper(), 0);
        assert!(s.snapshot().is_usable());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
