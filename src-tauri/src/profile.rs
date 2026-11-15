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
use crate::features::VoiceBand;
use crate::train::build_item_with_band;
use crate::train::{align_item, build_item, train_model, user_take_weight, TrainItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard};

/// Base clips weigh 1 each in training. The user's own recordings share a
/// fixed total (`train::user_take_weight`): `retrain` sets their weight
/// from how many there are, so they are analysed with a neutral one.
const BASE_WEIGHT: f32 = 1.0;
const TAKE_WEIGHT: f32 = 1.0;
const MIN_TAKE_SECS: f32 = 0.5;

// ===========================================================================
// Data model
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingNoise {
    Juzu,
    Bell,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeRecord {
    /// Missing on existing takes, which retain their clean-training behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise: Option<TrainingNoise>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_band: Option<VoiceBand>,
    pub id: usize,
    pub n_daimoku: u32,
    pub duration_secs: f32,
    /// Median time from one Daimoku to the next in this take, once
    /// aligned against the current model. `None` until the first
    /// successful retrain.
    pub period_ms: Option<f32>,
    pub created_at: DateTime<Utc>,
    /// Speed the user chose when recording ("slow" | "medium" | "fast").
    /// Shown as is; older takes without it are labelled from their timing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// What actually gets written to disk (audio lives in sibling .wav files).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProfileIndex {
    version: u32,
    takes: Vec<TakeRecord>,
    /// Partials (Hz) of the user's own bell, learned by the guided
    /// recording (`bell::learn_partials`); empty until then.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    bell: Vec<f32>,
}

/// One recording of the guided session, ready to join the profile.
pub struct NewTake {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub n_daimoku: u32,
    pub speed: Option<String>,
    pub voice_band: Option<VoiceBand>,
    pub noise: Option<TrainingNoise>,
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
    /// For the slow controller (not sent to the app's screens).
    #[serde(skip)]
    pub slow_model: Option<Model>,
    /// Partials (Hz) of the user's own bell: the counter cuts them out of
    /// the microphone while it rings (`bell::BellFilter::set_partials`).
    #[serde(default)]
    pub bell: Vec<f32>,
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
pub(crate) fn take_wav_path(id: usize) -> Option<PathBuf> {
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
    /// Model trained only on slow Daimoku, for the slow controller.
    slow_model: Option<Model>,
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
            Some((samples, sr)) => match build_item(&samples, sr, t.n_daimoku as usize, TAKE_WEIGHT) {
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
        bell: index.bell.clone(),
    };
    // refresh_model below trains once, with the calibrated band if present.
    let model = base_only_fallback();
    let slow_model = None;
    let mut inner = ProfileInner {
        index,
        base_items,
        user_items,
        model,
        slow_model,
    };
    refresh_model(&mut inner);
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
            slow_model: g.slow_model.clone(),
            bell: g.index.bell.clone(),
        }
    }

    /// Adds the recordings of a guided session at once (one retraining)
    /// and, if a bell was heard in it, remembers the bell's partials.
    /// Returns how many recordings were added.
    pub fn add_guided(&self, takes: Vec<NewTake>, bell: Option<Vec<f32>>) -> Result<usize, String> {
        // analyse first, outside the lock: an unusable recording is left out
        let usable: Vec<NewTake> = takes
            .into_iter()
            .filter(|t| {
                t.n_daimoku > 0
                    && t.sample_rate > 0
                    && t.samples.len() as f32 / t.sample_rate as f32 >= MIN_TAKE_SECS
                    && build_item(&t.samples, t.sample_rate, t.n_daimoku as usize, TAKE_WEIGHT).is_some()
            })
            .collect();
        if usable.is_empty() && bell.as_ref().map_or(true, |b| b.is_empty()) {
            return Err("could not find enough voiced audio in these takes".to_string());
        }
        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        let added = usable.len();
        for t in usable {
            // never reuse an id: its WAV file would be overwritten
            let id = g.index.takes.iter().map(|t| t.id + 1).max().unwrap_or(0);
            if let Some(path) = take_wav_path(id) {
                write_wav_f32(&path, &t.samples, t.sample_rate)?;
            }
            g.index.takes.push(TakeRecord {
                noise: t.noise,
                voice_band: if t.noise.is_none() { t.voice_band.and_then(VoiceBand::valid) } else { None },
                id,
                n_daimoku: t.n_daimoku,
                duration_secs: t.samples.len() as f32 / t.sample_rate as f32,
                period_ms: None,
                created_at: Utc::now(),
                speed: t.speed.filter(|s| matches!(s.as_str(), "slow" | "medium" | "fast")),
            });
        }
        if let Some(b) = bell.filter(|b| !b.is_empty()) {
            g.index.bell = b;
        }
        refresh_model(g);
        recompute_take_periods(g);
        write_index(&g.index)?;
        Ok(added)
    }

    /// Adds a new take (raw audio + how many Daimoku it contains),
    /// persists it, and retrains the model from base + all stored takes.
    /// Returns the new take's id.
    pub fn add_take(&self, samples: &[f32], sample_rate: u32, n_daimoku: u32) -> Result<usize, String> {
        self.add_take_with_speed(samples, sample_rate, n_daimoku, None)
    }

    /// Like `add_take`, remembering the speed the user chose for it.
    pub fn add_take_with_speed(
        &self,
        samples: &[f32],
        sample_rate: u32,
        n_daimoku: u32,
        speed: Option<String>,
    ) -> Result<usize, String> {
        self.add_calibrated_take(samples, sample_rate, n_daimoku, speed, None)
    }

    pub fn add_calibrated_take(
        &self, samples: &[f32], sample_rate: u32, n_daimoku: u32,
        speed: Option<String>, voice_band: Option<VoiceBand>,
    ) -> Result<usize, String> {
        self.add_conditioned_take(samples, sample_rate, n_daimoku, speed, voice_band, None)
    }

    pub fn add_conditioned_take(
        &self, samples: &[f32], sample_rate: u32, n_daimoku: u32,
        speed: Option<String>, voice_band: Option<VoiceBand>, noise: Option<TrainingNoise>,
    ) -> Result<usize, String> {
        let speed = speed.filter(|s| matches!(s.as_str(), "slow" | "medium" | "fast"));
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
        let item = build_item(samples, sample_rate, n_daimoku as usize, TAKE_WEIGHT)
            .ok_or_else(|| "could not find enough voiced audio in this take".to_string())?;

        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        // never reuse an id: its WAV file would be overwritten
        let id = g.index.takes.iter().map(|t| t.id + 1).max().unwrap_or(0);
        if let Some(path) = take_wav_path(id) {
            write_wav_f32(&path, samples, sample_rate)?;
        }
        g.index.takes.push(TakeRecord {
            noise,
            voice_band: if noise.is_none() { voice_band.and_then(VoiceBand::valid) } else { None },
            id,
            n_daimoku,
            duration_secs,
            period_ms: None,
            created_at: Utc::now(),
            speed,
        });
        g.user_items.push(item);
        refresh_model(g);
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
        refresh_model(g);
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
            if let Some(item) = build_item(&samples, sr, rec.n_daimoku as usize, TAKE_WEIGHT) {
                kept.push((rec, samples, sr, item));
            }
        }
        let mut guard = self.guard();
        let g = guard.as_mut().expect("loaded");
        if let Some(dir) = takes_dir() {
            let _ = fs::remove_dir_all(&dir);
        }
        // the bell is the user's, not the recordings': it stays
        let mut index = ProfileIndex { version: 1, takes: Vec::new(), bell: g.index.bell.clone() };
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
        refresh_model(g);
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
        g.index = ProfileIndex { version: 1, takes: Vec::new(), bell: Vec::new() };
        g.user_items.clear();
        refresh_model(g);
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
    retrain(&base, &[], &[]).unwrap_or_else(base_only_fallback)
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

/// Model for the slow checker (see `engine::build_slow_model`): the slow
/// built-in examples and the user's slow takes.
fn retrain_slow(base: &[TrainItem], user: &[TrainItem], takes: &[TakeRecord]) -> Option<Model> {
    let user: Vec<(TrainItem, f32)> = weighted(user, takes)
        .into_iter()
        .zip(takes)
        .map(|(item, take)| (item, take.duration_secs / take.n_daimoku.max(1) as f32))
        .collect();
    crate::engine::build_slow_model(base, &user)
}

fn retrain(base: &[TrainItem], user: &[TrainItem], takes: &[TakeRecord]) -> Option<Model> {
    let mut all: Vec<TrainItem> = Vec::with_capacity(base.len() + user.len());
    all.extend(base.iter().cloned());
    all.extend(weighted(user, takes));
    train_model(&all, &[], None, 8).map(|t| t.model)
}

/// The user's takes with their share of the training weight.
fn weighted(user: &[TrainItem], takes: &[TakeRecord]) -> Vec<TrainItem> {
    let noisy = takes.iter().filter(|t| t.noise.is_some()).count();
    let w = user_take_weight(user.len().saturating_sub(noisy));
    // Noisy takes supplement the clean references without diluting their
    // weights. Bound their total influence even if many noisy takes are added.
    let noise_weight = (1.25 / noisy.max(1) as f32).min(0.5);
    user.iter()
        .cloned()
        .enumerate()
        .map(|(i, mut item)| {
            item.weight = if takes.get(i).is_some_and(|t| t.noise.is_some()) { noise_weight } else { w };
            item
        })
        .collect()
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
        voice_band: None,
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

/// Rebuild all feature sets with one common band. Read one saved recording at
/// a time, rather than retaining full-rate WAV buffers for the app lifetime.
fn refresh_model(g: &mut ProfileInner) {
    let requested = VoiceBand::union(g.index.takes.iter().filter(|t| t.noise.is_none()).filter_map(|t| t.voice_band));
    for band in [requested, None] {
        let base: Option<Vec<TrainItem>> = BASE_CLIPS.iter().map(|clip| {
            let (samples, sr) = decode_wav(clip.wav).ok()?;
            build_item_with_band(&samples, sr, clip.n_cycles, BASE_WEIGHT, band)
        }).collect();
        // the takes as the counter will hear them: without the user's bell
        let bell = &g.index.bell;
        let user: Option<Vec<TrainItem>> = g.index.takes.iter().map(|take| {
            let bytes = fs::read(take_wav_path(take.id)?).ok()?;
            let (samples, sr) = decode_wav(&bytes).ok()?;
            let samples = if bell.is_empty() { samples } else { crate::bell::without_bell(&samples, sr, bell) };
            build_item_with_band(&samples, sr, take.n_daimoku as usize, TAKE_WEIGHT, band)
        }).collect();
        if let (Some(base), Some(user)) = (base, user) {
            if let Some(mut model) = retrain(&base, &user, &g.index.takes) {
                model.voice_band = band;
                let mut slow = retrain_slow(&base, &user, &g.index.takes);
                if let Some(slow) = &mut slow { slow.voice_band = band; }
                g.base_items = base;
                g.user_items = user;
                g.model = model;
                g.slow_model = slow;
                return;
            }
        }
    }
    // An unusable imported take must not install a model/filter mismatch.
    g.base_items = load_base_items();
    g.model = retrain(&g.base_items, &[], &[]).unwrap_or_else(base_only_fallback);
    g.slow_model = retrain_slow(&g.base_items, &[], &[]);
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
    fn slow_model_is_always_available() {
        let s = ProfileState::new();
        // built from the slow built-in examples even without user takes
        assert!(s.snapshot().slow_model.is_some());
    }

    #[test]
    fn starts_usable_from_base_clips_alone() {
        let s = ProfileState::new();
        let snap = s.snapshot();
        assert!(snap.is_usable());
        assert_eq!(snap.base_clip_count, crate::base::BASE_CLIPS.len());
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
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    fn guided_takes_join_the_profile_at_once_with_the_bell() {
        let s = ProfileState::new();
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        let take = |noise| NewTake {
            samples: samples.clone(),
            sample_rate: sr,
            n_daimoku: 1,
            speed: Some("fast".to_string()),
            voice_band: None,
            noise,
        };
        let bell = vec![1875.0, 3480.0, 5456.0];
        let added = s.add_guided(vec![take(None), take(Some(TrainingNoise::Bell))], Some(bell.clone())).expect("guided");
        assert_eq!(added, 2);
        let snap = s.snapshot();
        assert_eq!(snap.n_takes_helper(), 2);
        assert_eq!(snap.bell, bell);
        assert_eq!(snap.takes[0].noise, None);
        assert_eq!(snap.takes[1].noise, Some(TrainingNoise::Bell));
        assert_eq!(snap.takes[1].speed.as_deref(), Some("fast"));
        assert_ne!(snap.takes[0].id, snap.takes[1].id);
        // nothing to add and no bell: refused; a bell alone is remembered
        assert!(s.add_guided(Vec::new(), None).is_err());
        assert_eq!(s.add_guided(Vec::new(), Some(vec![700.0, 1900.0, 3500.0])).unwrap(), 0);
        assert_eq!(s.snapshot().bell, vec![700.0, 1900.0, 3500.0]);
        // deleting a take keeps the bell; starting over forgets it
        s.delete_take(snap.takes[0].id).unwrap();
        assert_eq!(s.snapshot().bell.len(), 3);
        s.clear().unwrap();
        assert!(s.snapshot().bell.is_empty());
    }

    #[test]
    fn delete_single_take() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let s = std::sync::Arc::new(ProfileState::empty());
        assert!(!s.is_ready());
        let s2 = std::sync::Arc::clone(&s);
        let h = std::thread::spawn(move || s2.snapshot().base_clip_count);
        std::thread::sleep(std::time::Duration::from_millis(50));
        s.load();
        assert_eq!(h.join().unwrap(), crate::base::BASE_CLIPS.len());
    }

    #[test]
    fn clear_resets_to_base_only() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
