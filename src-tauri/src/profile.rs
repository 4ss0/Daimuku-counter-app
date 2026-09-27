//! Personal profile for Daimoku counting.
//!
//! After a few validated takes, this module produces:
//!   - a robust median period for the user's recitation speed
//!   - a normalized 128-sample template of one Daimoku
//!   - the min/max period ratio observed, used by the multi-scale
//!     matched filter in `dsp.rs` to follow speed changes
//!   - a rough zero-crossing-rate band, used by `dsp.rs` as a cheap
//!     "does this sound like the trained voice at all" content gate
//!
//! Persistence: JSON file in `dirs::data_dir()/daimuku-counter/profile.json`.

use crate::dsp_common::{
    compute_novelty, find_active_segments, median_f32, moving_average, rms_envelope,
    zero_crossing_rate,
};
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
    /// Median zero-crossing-rate of this take's Daimoku segments.
    /// `#[serde(default)]` so profile.json files saved before this
    /// field existed still load correctly.
    #[serde(default)]
    pub zcr: Option<f32>,
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
    #[serde(default)]
    pub zcr_median: f32,
    #[serde(default)]
    pub zcr_sigma: f32,
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
            zcr_median: 0.0,
            zcr_sigma: 0.0,
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

        let extracted = extract_period_and_template(samples, sample_rate, n_daimoku as usize)
            .ok_or_else(|| "could not extract period/template from this take".to_string())?;

        let id = self.takes.len();
        let record = TakeRecord {
            id,
            n_daimoku,
            duration_secs,
            period_ms: extracted.period_ms,
            template: extracted.template,
            zcr: extracted.zcr,
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
            self.zcr_median = 0.0;
            self.zcr_sigma = 0.0;
            return;
        }

        let mut periods: Vec<f32> = self.takes.iter().map(|t| t.period_ms).collect();
        periods.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = median_f32(&periods);
        self.natural_period_ms = median;

        let mean = periods.iter().sum::<f32>() / periods.len() as f32;
        let var = periods.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / periods.len() as f32;
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

        let zcr_vals: Vec<f32> = self.takes.iter().filter_map(|t| t.zcr).collect();
        if zcr_vals.is_empty() {
            self.zcr_median = 0.0;
            self.zcr_sigma = 0.0;
        } else {
            let zmed = median_f32(&zcr_vals);
            let zmean = zcr_vals.iter().sum::<f32>() / zcr_vals.len() as f32;
            let zvar =
                zcr_vals.iter().map(|v| (v - zmean).powi(2)).sum::<f32>() / zcr_vals.len() as f32;
            self.zcr_median = zmed;
            self.zcr_sigma = zvar.sqrt();
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
            Ok(s) => match serde_json::from_str(&s) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "[profile] could not parse {}: {e} — starting from an empty \
                         profile. The unreadable file was kept as a .bak instead of \
                         being silently discarded.",
                        path.display()
                    );
                    let backup = path.with_extension("json.bak");
                    let _ = fs::rename(&path, &backup);
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("[profile] could not read {}: {e}", path.display());
                Self::default()
            }
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

        // Write-then-rename so a crash mid-write can never leave a
        // half-written, corrupt profile.json behind (rename is atomic
        // on the same filesystem).
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| format!("failed to write {}: {e}", tmp_path.display()))?;
        fs::rename(&tmp_path, &path)
            .map_err(|e| format!("failed to finalize {}: {e}", path.display()))?;
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

/// Result of analyzing one training take.
pub struct ExtractResult {
    pub period_ms: f32,
    pub template: Option<Vec<f32>>,
    /// Median zero-crossing-rate of the individual Daimoku segments
    /// found in this take, if any could be extracted.
    pub zcr: Option<f32>,
}

pub fn extract_period_and_template(
    samples: &[f32],
    sample_rate: u32,
    n_daimoku: usize,
) -> Option<ExtractResult> {
    if n_daimoku == 0 || sample_rate == 0 {
        return None;
    }

    let hop_ms: u32 = 10;
    let hop_secs = hop_ms as f32 / 1000.0;
    let hop_samples = (hop_ms as usize * sample_rate as usize) / 1000;
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

    // Split into active runs, bridging brief dips (e.g. between
    // syllables of the same Daimoku) but splitting on real pauses — a
    // breath, a hesitation — so they don't get folded into the
    // per-Daimoku period as if they were part of it. This is the fix
    // for the breath-between-8th-and-9th-Daimoku case.
    let min_gap_frames = 15; // ~150ms of sustained silence = a real pause
    let segments = find_active_segments(&smoothed, thresh, min_gap_frames);
    if segments.is_empty() {
        return None;
    }

    let total_active_frames: usize = segments.iter().map(|&(s, e)| e - s).sum();
    let active_secs = total_active_frames as f32 * hop_secs;
    if active_secs < MIN_ACTIVE_SECS {
        return None;
    }

    // Period from *active* time only: any breaths between segments
    // are excluded, so they no longer inflate the estimated
    // per-Daimoku duration.
    let t_frames = total_active_frames as f32 / n_daimoku as f32;
    if t_frames < 15.0 {
        return None;
    }
    let period_secs = t_frames * hop_secs;
    let period_ms = period_secs * 1000.0;

    // Spread the n_daimoku across segments proportionally to how long
    // each one is; the longest segment absorbs any rounding remainder
    // so the total always matches exactly.
    let mut counts: Vec<usize> = segments
        .iter()
        .map(|&(s, e)| (((e - s) as f32) / t_frames).round() as usize)
        .collect();
    fixup_segment_counts(&mut counts, n_daimoku, &segments);

    let mut nov_segments: Vec<Vec<f32>> = Vec::with_capacity(n_daimoku);
    let mut raw_zcrs: Vec<f32> = Vec::with_capacity(n_daimoku);

    for (&(seg_start, seg_end), &seg_count) in segments.iter().zip(counts.iter()) {
        if seg_count == 0 {
            continue;
        }
        let seg_len = (seg_end - seg_start) as f32;
        let local_t = seg_len / seg_count as f32;
        if local_t < 4.0 {
            continue;
        }

        // Phase search restricted to this segment only, so a pause
        // elsewhere in the take can't shift where we think a Daimoku
        // in *this* segment starts.
        let steps = 200usize;
        let mut best_phi = seg_start as f32;
        let mut best_score = f32::MIN;
        for k in 0..steps {
            let phi = seg_start as f32 + (k as f32 / steps as f32) * local_t;
            let mut score = 0.0f32;
            for d in 0..seg_count {
                let pos = phi + d as f32 * local_t;
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

        let half = local_t / 2.0;
        for d in 0..seg_count {
            let center = best_phi + d as f32 * local_t;
            let lo = (center - half).round() as i64;
            let hi = (center + half).round() as i64;
            let lo = lo.max(seg_start as i64) as usize;
            let hi = (hi.min(seg_end as i64) as usize).max(lo + 2);
            if hi - lo < 4 {
                continue;
            }
            let seg = &nov[lo..hi];
            if let Some(norm) = z_score_resample(seg, TEMPLATE_LEN) {
                nov_segments.push(norm);
            }

            let raw_lo = (lo * hop_samples).min(samples.len());
            let raw_hi = (hi * hop_samples).min(samples.len());
            if raw_hi > raw_lo + 1 {
                raw_zcrs.push(zero_crossing_rate(&samples[raw_lo..raw_hi]));
            }
        }
    }

    let zcr = if raw_zcrs.is_empty() {
        None
    } else {
        Some(median_f32(&raw_zcrs))
    };

    if nov_segments.len() < n_daimoku / 2 + 1 {
        return Some(ExtractResult {
            period_ms,
            template: None,
            zcr,
        });
    }

    let template = samplewise_median(&nov_segments, TEMPLATE_LEN);
    Some(ExtractResult {
        period_ms,
        template: Some(template),
        zcr,
    })
}

/// Nudges rounded per-segment Daimoku counts so they sum to exactly
/// `n_daimoku`, adjusting the longest segment(s) first since a
/// rounding error is least noticeable there.
fn fixup_segment_counts(counts: &mut [usize], n_daimoku: usize, segments: &[(usize, usize)]) {
    if segments.is_empty() {
        return;
    }
    let sum: usize = counts.iter().sum();
    if sum == n_daimoku {
        return;
    }

    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(segments[i].1 - segments[i].0));

    if sum < n_daimoku {
        let mut remaining = n_daimoku - sum;
        for &i in &order {
            if remaining == 0 {
                break;
            }
            counts[i] += 1;
            remaining -= 1;
        }
    } else {
        let mut remaining = sum - n_daimoku;
        for &i in &order {
            if remaining == 0 {
                break;
            }
            let take = counts[i].min(remaining);
            counts[i] -= take;
            remaining -= take;
        }
    }
}

// ===========================================================================
// Low-level helpers (local to template extraction)
// ===========================================================================

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

    fn phrase_with_pause(
        bumps: usize,
        period_s: f32,
        pause_after: usize,
        pause_s: f32,
    ) -> Vec<f32> {
        let mut v = Vec::new();
        for b in 0..bumps {
            v.extend(phrase(1, period_s));
            if b + 1 == pause_after {
                v.extend(silence(pause_s));
            }
        }
        v
    }

    #[test]
    fn extract_returns_period_and_template() {
        let v = phrase(10, 1.20);
        let r = extract_period_and_template(&v, SR, 10).expect("extraction");
        assert!((1150.0..=1250.0).contains(&r.period_ms), "period {}", r.period_ms);
        let t = r.template.expect("template");
        assert_eq!(t.len(), TEMPLATE_LEN);
        let var: f32 = t.iter().map(|x| x * x).sum::<f32>() / t.len() as f32;
        assert!(var > 0.1);
    }

    #[test]
    fn extract_handles_slow_daimoku() {
        let v = phrase(2, 4.0);
        let r = extract_period_and_template(&v, SR, 2).expect("slow");
        assert!((3500.0..=4500.0).contains(&r.period_ms), "period {}", r.period_ms);
    }

    #[test]
    fn extract_handles_fast_daimoku() {
        let v = phrase(10, 0.80);
        let r = extract_period_and_template(&v, SR, 10).expect("fast");
        assert!((750.0..=850.0).contains(&r.period_ms), "period {}", r.period_ms);
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
        let rl = extract_period_and_template(&loud, SR, 5).unwrap();
        let rq = extract_period_and_template(&quiet, SR, 5).unwrap();
        let tl = rl.template.expect("t");
        let tq = rq.template.expect("t");
        let diff: f32 = tl
            .iter()
            .zip(tq.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / tl.len() as f32;
        assert!(diff < 0.5, "diff {diff}");
    }

    #[test]
    fn breath_pause_does_not_bias_period_estimate() {
        // Mirrors the real-world case: 10 Daimoku with a breath after
        // the 8th, like in the training screenshots.
        let true_period = 1.20_f32;
        let v = phrase_with_pause(10, true_period, 8, 1.0);
        let r = extract_period_and_template(&v, SR, 10).expect("extraction");
        let expected_ms = true_period * 1000.0;
        assert!(
            (r.period_ms - expected_ms).abs() < 80.0,
            "expected ~{expected_ms}ms, got {}ms — a breath pause should not bias \
             the period estimate",
            r.period_ms
        );
    }

    #[test]
    fn extract_reports_a_zcr_estimate() {
        let v = phrase(10, 1.0);
        let r = extract_period_and_template(&v, SR, 10).expect("extraction");
        let z = r.zcr.expect("zcr");
        assert!(z > 0.0 && z < 1.0, "zcr {z} out of plausible range");
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
        assert!(p.zcr_median > 0.0);
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

    #[test]
    fn profile_deserializes_pre_zcr_json_without_the_new_fields() {
        // Simulates a profile.json saved by a version of the app from
        // before the zcr fields existed, so an update doesn't strand
        // the user's already-trained profile.
        let old_json = r#"{
            "version": 1,
            "takes": [],
            "natural_period_ms": 0.0,
            "period_sigma_ms": 0.0,
            "template": null,
            "period_ratio_range": [1.0, 1.0]
        }"#;
        let p: PersonalProfile = serde_json::from_str(old_json).expect("should deserialize");
        assert_eq!(p.zcr_median, 0.0);
        assert_eq!(p.zcr_sigma, 0.0);
    }
}