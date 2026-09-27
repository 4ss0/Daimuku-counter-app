//! Personal profile for Daimoku counting.
//!
//! After a few validated takes, this module produces:
//!   - a robust median period for the user's recitation speed
//!   - a normalized 128-sample template of one Daimoku
//!   - the min/max period ratio observed, used by the multi-scale
//!     matched filter in `dsp.rs` to follow speed changes
//!
//! Persistence: JSON file in `dirs::data_dir()/daimuku-counter/profile.json`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub const TEMPLATE_LEN: usize = 128;
const MIN_TAKE_SECS: f32 = 0.5;
const MIN_ACTIVE_SECS: f32 = 0.3;

// ===========================================================================
// Data model
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeRecord {
    pub id: usize,
    pub n_daimoku: u32,
    pub duration_secs: f32,
    pub period_ms: f32,
    pub template: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalProfile {
    pub version: u32,
    pub takes: Vec<TakeRecord>,
    pub natural_period_ms: f32,
    pub period_sigma_ms: f32,
    pub template: Option<Vec<f32>>,
    pub period_ratio_range: (f32, f32),
}

impl Default for PersonalProfile {
    fn default() -> Self {
        Self {
            version: 1,
            takes: Vec::new(),
            natural_period_ms: 0.0,
            period_sigma_ms: 0.0,
            template: None,
            period_ratio_range: (1.0, 1.0),
        }
    }
}

impl PersonalProfile {
    pub fn n_takes(&self) -> usize {
        self.takes.len()
    }

    pub fn is_usable(&self) -> bool {
        !self.takes.is_empty() && self.natural_period_ms > 0.0
    }

    pub fn add_take(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        n_daimoku: u32,
    ) -> Result<usize, String> {
        if n_daimoku == 0 {
            return Err("n_daimoku must be >= 1".to_string());
        }
        if sample_rate == 0 {
            return Err("sample_rate must be > 0".to_string());
        }
        let duration_secs = samples.len() as f32 / sample_rate as f32;
        if duration_secs < MIN_TAKE_SECS {
            return Err(format!(
                "take too short ({duration_secs:.2}s, minimum {MIN_TAKE_SECS}s)"
            ));
        }

        let (period_ms, template) =
            extract_period_and_template(samples, sample_rate, n_daimoku as usize)
                .ok_or_else(|| "could not extract period/template from this take".to_string())?;

        let id = self.takes.len();
        let record = TakeRecord {
            id,
            n_daimoku,
            duration_secs,
            period_ms,
            template,
            created_at: Utc::now(),
        };
        self.takes.push(record);
        self.recompute();
        Ok(id)
    }

    fn recompute(&mut self) {
        if self.takes.is_empty() {
            self.natural_period_ms = 0.0;
            self.period_sigma_ms = 0.0;
            self.template = None;
            self.period_ratio_range = (1.0, 1.0);
            return;
        }

        let mut periods: Vec<f32> = self.takes.iter().map(|t| t.period_ms).collect();
        periods.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = median_f32(&periods);
        self.natural_period_ms = median;

        let mean = periods.iter().sum::<f32>() / periods.len() as f32;
        let var = periods
            .iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f32>()
            / periods.len() as f32;
        self.period_sigma_ms = var.sqrt();

        if median > 0.0 {
            let min_r = periods.first().copied().unwrap_or(median) / median;
            let max_r = periods.last().copied().unwrap_or(median) / median;
            let min_r = min_r.min(0.60);
            let max_r = max_r.max(1.60);
            self.period_ratio_range = (min_r, max_r);
        } else {
            self.period_ratio_range = (1.0, 1.0);
        }

        let templates: Vec<&Vec<f32>> = self
            .takes
            .iter()
            .filter_map(|t| t.template.as_ref())
            .collect();
        if templates.is_empty() {
            self.template = None;
        } else {
            let mut out = vec![0.0f32; TEMPLATE_LEN];
            let mut buf: Vec<f32> = Vec::with_capacity(templates.len());
            for j in 0..TEMPLATE_LEN {
                buf.clear();
                for t in &templates {
                    if j < t.len() {
                        buf.push(t[j]);
                    }
                }
                out[j] = median_f32(&buf);
            }
            self.template = Some(out);
        }
    }

    pub fn clear(&mut self) {
        *self = PersonalProfile::default();
    }
}

// ===========================================================================
// Persistence
// ===========================================================================

fn profile_path() -> Option<PathBuf> {
    let base = dirs::data_dir()?;
    Some(base.join("daimuku-counter").join("profile.json"))
}

impl PersonalProfile {
    pub fn load() -> Self {
        let Some(path) = profile_path() else {
            return Self::default();
        };
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path =
            profile_path().ok_or_else(|| "cannot determine data directory".to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize profile: {e}"))?;
        fs::write(&path, json)
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        Ok(())
    }
}

// ===========================================================================
// Thread-safe state holder
// ===========================================================================

pub struct ProfileState {
    inner: Mutex<PersonalProfile>,
}

impl ProfileState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PersonalProfile::load()),
        }
    }

    pub fn snapshot(&self) -> PersonalProfile {
        match self.inner.lock() {
            Ok(g) => g.clone(),
            Err(_) => PersonalProfile::default(),
        }
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut PersonalProfile) -> R) -> R {
        let mut g = self.inner.lock().unwrap();
        f(&mut g)
    }
}

impl Default for ProfileState {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Template extraction
// ===========================================================================

pub fn extract_period_and_template(
    samples: &[f32],
    sample_rate: u32,
    n_daimoku: usize,
) -> Option<(f32, Option<Vec<f32>>)> {
    if n_daimoku == 0 || sample_rate == 0 {
        return None;
    }

    let hop_ms: u32 = 10;
    let hop_secs = hop_ms as f32 / 1000.0;
    let smooth_frames = 20;
    let nov_lag_frames = 30;
    let nov_smooth_frames = 10;

    let env = rms_envelope(samples, sample_rate, 30, hop_ms);
    if env.len() < 50 {
        return None;
    }
    let smoothed = moving_average(&env, smooth_frames);
    let nov = compute_novelty(&smoothed, nov_lag_frames);
    let nov = moving_average(&nov, nov_smooth_frames);

    let max_smooth = smoothed.iter().cloned().fold(f32::MIN, f32::max);
    if max_smooth <= 1e-6 {
        return None;
    }
    let thresh = 0.15 * max_smooth;
    let first = smoothed.iter().position(|&x| x > thresh)?;
    let last = smoothed.iter().rposition(|&x| x > thresh)?;
    if last <= first {
        return None;
    }
    let active_frames = last - first + 1;
    let active_secs = active_frames as f32 * hop_secs;
    if active_secs < MIN_ACTIVE_SECS {
        return None;
    }

    let t_frames = active_frames as f32 / n_daimoku as f32;
    if t_frames < 15.0 {
        return None;
    }
    let period_secs = t_frames * hop_secs;
    let period_ms = period_secs * 1000.0;

    let phase_span = t_frames.max(1.0);
    let steps = 200usize;
    let mut best_phi = first as f32;
    let mut best_score = f32::MIN;
    for k in 0..steps {
        let phi = first as f32 + (k as f32 / steps as f32) * phase_span;
        let mut score = 0.0f32;
        for d in 0..n_daimoku {
            let pos = phi + d as f32 * t_frames;
            let idx = pos.round() as usize;
            if idx < nov.len() {
                score += nov[idx];
            }
        }
        if score > best_score {
            best_score = score;
            best_phi = phi;
        }
    }

    let half = t_frames / 2.0;
    let mut segments: Vec<Vec<f32>> = Vec::with_capacity(n_daimoku);
    for d in 0..n_daimoku {
        let center = best_phi + d as f32 * t_frames;
        let lo = (center - half).round() as i64;
        let hi = (center + half).round() as i64;
        let lo = lo.max(0) as usize;
        let hi = (hi.min(nov.len() as i64) as usize).max(lo + 2);
        if hi - lo < 4 {
            continue;
        }
        let seg = &nov[lo..hi];
        if let Some(norm) = z_score_resample(seg, TEMPLATE_LEN) {
            segments.push(norm);
        }
    }

    if segments.len() < n_daimoku / 2 + 1 {
        return Some((period_ms, None));
    }

    let template = samplewise_median(&segments, TEMPLATE_LEN);
    Some((period_ms, Some(template)))
}

// ===========================================================================
// Low-level helpers
// ===========================================================================

fn rms_envelope(samples: &[f32], sr: u32, window_ms: u32, hop_ms: u32) -> Vec<f32> {
    let w = (window_ms as usize * sr as usize) / 1000;
    let h = (hop_ms as usize * sr as usize) / 1000;
    if w == 0 || h == 0 || samples.len() < w {
        return Vec::new();
    }
    let mut out = Vec::with_capacity((samples.len() - w) / h + 1);
    let mut i = 0;
    while i + w <= samples.len() {
        let s: f32 = samples[i..i + w].iter().map(|x| x * x).sum();
        out.push((s / w as f32).sqrt());
        i += h;
    }
    out
}

fn moving_average(data: &[f32], window: usize) -> Vec<f32> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }
    let w = window.max(1).min(n);
    let half = w / 2;
    let mut out = vec![0.0f32; n];
    let mut sum = 0.0f32;
    let mut lo = 0usize;
    let mut hi = 0usize;
    for i in 0..n {
        let want_lo = i.saturating_sub(half);
        let want_hi = (i + half + 1).min(n);
        while lo < want_lo {
            sum -= data[lo];
            lo += 1;
        }
        while hi < want_hi {
            sum += data[hi];
            hi += 1;
        }
        out[i] = sum / (hi - lo).max(1) as f32;
    }
    out
}

fn compute_novelty(sig: &[f32], lag: usize) -> Vec<f32> {
    let n = sig.len();
    let mut out = vec![0.0f32; n];
    if lag == 0 || lag >= n {
        return out;
    }
    for i in lag..n {
        let d = sig[i] - sig[i - lag];
        if d > 0.0 {
            out[i] = d;
        }
    }
    out
}

fn z_score_resample(seg: &[f32], target_len: usize) -> Option<Vec<f32>> {
    if seg.len() < 2 || target_len == 0 {
        return None;
    }
    let n = seg.len() as f32;
    let mean = seg.iter().sum::<f32>() / n;
    let var = seg.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
    let std = var.sqrt();
    if std <= 1e-6 {
        return None;
    }
    let normalized: Vec<f32> = seg.iter().map(|x| (x - mean) / std).collect();

    let src_len = normalized.len();
    let mut out = Vec::with_capacity(target_len);
    let step = src_len as f32 / target_len as f32;
    for k in 0..target_len {
        let pos = k as f32 * step;
        let i = pos.floor() as usize;
        let frac = pos - i as f32;
        let a = normalized[i.min(src_len - 1)];
        let b = normalized[(i + 1).min(src_len - 1)];
        out.push(a + (b - a) * frac);
    }
    Some(out)
}

fn samplewise_median(segments: &[Vec<f32>], len: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; len];
    let mut buf: Vec<f32> = Vec::with_capacity(segments.len());
    for j in 0..len {
        buf.clear();
        for s in segments {
            if j < s.len() {
                buf.push(s[j]);
            }
        }
        out[j] = median_f32(&buf);
    }
    out
}

fn median_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut v = data.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    if v.len() % 2 == 0 {
        (v[m - 1] + v[m]) * 0.5
    } else {
        v[m]
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const SR: u32 = 48_000;

    fn phrase(bumps: usize, period_s: f32) -> Vec<f32> {
        let n = (bumps as f32 * period_s * SR as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let car = (2.0 * PI * 220.0 * t).sin() + 0.4 * (2.0 * PI * 660.0 * t).sin();
            let phase = (t / period_s).fract();
            let bump = 0.55 + 0.45 * (PI * phase).sin().powf(0.6);
            v.push(car * bump * 0.4);
        }
        v
    }

    fn silence(secs: f32) -> Vec<f32> {
        vec![0.0; (secs * SR as f32) as usize]
    }

    #[test]
    fn extract_returns_period_and_template() {
        let v = phrase(10, 1.20);
        let (period_ms, tmpl) = extract_period_and_template(&v, SR, 10).expect("extraction");
        assert!((1150.0..=1250.0).contains(&period_ms), "period {period_ms}");
        let t = tmpl.expect("template");
        assert_eq!(t.len(), TEMPLATE_LEN);
        let var: f32 = t.iter().map(|x| x * x).sum::<f32>() / t.len() as f32;
        assert!(var > 0.1);
    }

    #[test]
    fn extract_handles_slow_daimoku() {
        let v = phrase(2, 4.0);
        let (period_ms, _) = extract_period_and_template(&v, SR, 2).expect("slow");
        assert!((3500.0..=4500.0).contains(&period_ms), "period {period_ms}");
    }

    #[test]
    fn extract_handles_fast_daimoku() {
        let v = phrase(10, 0.80);
        let (period_ms, _) = extract_period_and_template(&v, SR, 10).expect("fast");
        assert!((750.0..=850.0).contains(&period_ms), "period {period_ms}");
    }

    #[test]
    fn extract_rejects_empty_and_silence() {
        assert!(extract_period_and_template(&[], SR, 10).is_none());
        assert!(extract_period_and_template(&silence(5.0), SR, 10).is_none());
    }

    #[test]
    fn extract_rejects_zero_daimoku() {
        let v = phrase(10, 1.0);
        assert!(extract_period_and_template(&v, SR, 0).is_none());
    }

    #[test]
    fn template_is_amplitude_invariant() {
        let loud = phrase(5, 1.0);
        let quiet: Vec<f32> = loud.iter().map(|x| x * 0.5).collect();
        let (_, tl) = extract_period_and_template(&loud, SR, 5).unwrap();
        let (_, tq) = extract_period_and_template(&quiet, SR, 5).unwrap();
        let tl = tl.expect("t");
        let tq = tq.expect("t");
        let diff: f32 = tl
            .iter()
            .zip(tq.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / tl.len() as f32;
        assert!(diff < 0.5, "diff {diff}");
    }

    #[test]
    fn profile_starts_empty_and_unusable() {
        let p = PersonalProfile::default();
        assert_eq!(p.n_takes(), 0);
        assert!(!p.is_usable());
        assert!(p.template.is_none());
    }

    #[test]
    fn profile_rejects_too_short_take() {
        let mut p = PersonalProfile::default();
        let v = phrase(1, 0.20);
        assert!(p.add_take(&v, SR, 1).is_err());
        assert_eq!(p.n_takes(), 0);
    }

    #[test]
    fn profile_rejects_zero_daimoku() {
        let mut p = PersonalProfile::default();
        let v = phrase(5, 1.0);
        assert!(p.add_take(&v, SR, 0).is_err());
    }

    #[test]
    fn profile_adds_takes_and_computes_median() {
        let mut p = PersonalProfile::default();
        p.add_take(&phrase(10, 1.20), SR, 10).expect("natural");
        p.add_take(&phrase(3, 3.00), SR, 3).expect("slow");
        p.add_take(&phrase(10, 0.85), SR, 10).expect("fast");
        assert_eq!(p.n_takes(), 3);
        assert!(p.is_usable());
        assert!((1100.0..=1300.0).contains(&p.natural_period_ms));
        assert!(p.period_sigma_ms > 200.0);
        assert!(p.period_ratio_range.0 <= 0.75);
        assert!(p.period_ratio_range.1 >= 2.0);
        assert!(p.template.is_some());
    }

    #[test]
    fn profile_expands_ratio_range_floor() {
        let mut p = PersonalProfile::default();
        p.add_take(&phrase(10, 1.00), SR, 10).unwrap();
        p.add_take(&phrase(10, 1.02), SR, 10).unwrap();
        assert!(p.period_ratio_range.0 <= 0.60);
        assert!(p.period_ratio_range.1 >= 1.60);
    }

    #[test]
    fn profile_clear_resets_everything() {
        let mut p = PersonalProfile::default();
        p.add_take(&phrase(5, 1.0), SR, 5).unwrap();
        assert!(p.is_usable());
        p.clear();
        assert!(!p.is_usable());
        assert_eq!(p.n_takes(), 0);
    }

    #[test]
    fn profile_json_roundtrip() {
        let mut p = PersonalProfile::default();
        p.add_take(&phrase(10, 1.0), SR, 10).unwrap();
        p.add_take(&phrase(3, 2.5), SR, 3).unwrap();
        let json = serde_json::to_string(&p).expect("ser");
        let mut q: PersonalProfile = serde_json::from_str(&json).expect("de");
        q.recompute();
        assert_eq!(q.n_takes(), p.n_takes());
        assert!((q.natural_period_ms - p.natural_period_ms).abs() < 1.0);
        assert_eq!(q.template.is_some(), p.template.is_some());
    }
}