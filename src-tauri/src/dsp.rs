//! Daimoku counter.
//!
//! Two batch paths:
//!   1. **Fallback** (`count_daimoku`, `count_daimoku_default`): no profile,
//!      runs the streaming counter over the whole signal.
//!   2. **Profile-based** (`count_daimoku_with_profile`): uses a personal
//!      template and a multi-scale matched filter, plus a lightweight
//!      content gate. Robust to speed changes from 0.4x to 3x the
//!      natural rate of the user.

use crate::dsp_common::{
    compute_novelty, find_active_segments, moving_average, percentile, rms_envelope,
    zero_crossing_rate,
};
use crate::profile::PersonalProfile;
use serde::Serialize;
use std::cmp::Ordering;

// ===========================================================================
// Public API
// ===========================================================================

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub window_ms: u32,
    pub hop_ms: u32,
    pub smooth_secs: f32,
    pub novelty_lag_secs: f32,
    pub novelty_smooth_secs: f32,
    pub silence_rel: f32,
    pub min_pause_secs: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_ms: 30,
            hop_ms: 10,
            smooth_secs: 0.20,
            novelty_lag_secs: 0.30,
            novelty_smooth_secs: 0.10,
            silence_rel: 0.10,
            min_pause_secs: 0.30,
        }
    }
}

pub type DaimokuCountConfig = Config;

#[derive(Debug, Clone, Serialize)]
pub struct DaimokuCountResult {
    pub count: usize,
    pub confidence: f32,
    pub phrase_count: usize,
    pub mean_period_ms: f32,
    pub method: String,
    pub period_ms: f32,
    pub segment_count: usize,
    pub duration_secs: f32,
    pub active_duration_secs: f32,
}

pub fn count_daimoku(
    samples: &[f32],
    sample_rate: u32,
    config: DaimokuCountConfig,
) -> Option<DaimokuCountResult> {
    count_daimoku_with(samples, sample_rate, &config)
}

pub fn count_daimoku_default(samples: &[f32], sample_rate: u32) -> Option<DaimokuCountResult> {
    count_daimoku_with(samples, sample_rate, &Config::default())
}

pub fn count_daimoku_with(
    samples: &[f32],
    sample_rate: u32,
    cfg: &Config,
) -> Option<DaimokuCountResult> {
    let sr = sample_rate as usize;
    if sr == 0 || samples.len() < sr / 4 {
        return None;
    }

    let env = rms_envelope(samples, sample_rate, cfg.window_ms, cfg.hop_ms);
    if env.len() < 50 {
        return None;
    }
    let p95 = percentile(&env, 0.95);
    if p95 < 1e-5 {
        return None;
    }
    let p10 = percentile(&env, 0.10);
    if p10 > 0.90 * p95 {
        return None;
    }

    let hop_secs = cfg.hop_ms as f32 / 1000.0;
    let sm_frames = ((cfg.smooth_secs / hop_secs).round() as usize).max(1);
    let smoothed = moving_average(&env, sm_frames);
    let silence_floor = cfg.silence_rel * p95;
    let min_gap_frames = ((cfg.min_pause_secs / hop_secs).round() as usize).max(2);
    let phrases = find_active_segments(&smoothed, silence_floor, min_gap_frames);
    let phrase_count = phrases.len();

    let mut sc = StreamingCounter::new(sample_rate);
    let warmup = vec![0.0f32; sample_rate as usize];
    for chunk in warmup.chunks(4096) {
        sc.push(chunk);
    }
    for chunk in samples.chunks(4096) {
        sc.push(chunk);
    }

    let peaks = sc.accepted_peaks();
    let final_count = post_process_count(peaks);

    let duration_secs = samples.len() as f32 / sample_rate as f32;
    let mean_period_frames = robust_period_frames(peaks).unwrap_or(0.0);
    let mean_period_ms = (mean_period_frames as f32) * hop_secs * 1000.0;

    if final_count == 0 {
        return None;
    }

    let conf = if mean_period_ms > 0.0 { 0.85 } else { 0.3 };

    Some(DaimokuCountResult {
        count: final_count,
        confidence: conf,
        phrase_count,
        mean_period_ms,
        method: "novelty+post".to_string(),

        period_ms: mean_period_ms,
        segment_count: phrase_count,
        duration_secs,
        active_duration_secs: duration_secs,
    })
}

// ---------------------------------------------------------------------------
// Profile-based batch (matched filter, multi-scale, adaptive filtering)
// ---------------------------------------------------------------------------

pub fn count_daimoku_with_profile(
    samples: &[f32],
    sample_rate: u32,
    profile: Option<&PersonalProfile>,
) -> Option<DaimokuCountResult> {
    let Some(p) = profile else {
        return count_daimoku_default(samples, sample_rate);
    };
    if !p.is_usable() {
        return count_daimoku_default(samples, sample_rate);
    }
    let Some(template) = p.template.as_ref() else {
        return count_daimoku_default(samples, sample_rate);
    };

    let sr = sample_rate as usize;
    if sr == 0 || samples.len() < sr / 4 {
        return None;
    }

    let env = rms_envelope(samples, sample_rate, 30, 10);
    if env.len() < 50 {
        return None;
    }
    let p95 = percentile(&env, 0.95);
    if p95 < 1e-5 {
        return None;
    }
    let smoothed = moving_average(&env, 20);
    let nov = compute_novelty(&smoothed, 30);
    let nov = moving_average(&nov, 10);

    let nov_max = nov.iter().cloned().fold(f32::MIN, f32::max);
    if nov_max < 1e-6 {
        return None;
    }

    let hop_secs = 0.010_f32;
    let natural_frames = p.natural_period_ms / 1000.0 / hop_secs;
    if natural_frames < 15.0 {
        return count_daimoku_default(samples, sample_rate);
    }

    let score = multi_scale_correlation(&nov, template, natural_frames, p.period_ratio_range);
    if score.is_empty() {
        return count_daimoku_default(samples, sample_rate);
    }

    // Pass 1: raw detection. Low min_dist so nothing is missed.
    let initial_min_dist = (natural_frames * 0.35).round() as usize;
    let raw = pick_peaks(&score, initial_min_dist, 0.30);
    if raw.len() < 2 {
        return count_daimoku_default(samples, sample_rate);
    }

    // Pass 2: dominant period from raw peaks, robust to attack→body.
    let t_dom_frames = robust_period_from_peaks(&raw)
        .map(|f| f as f32)
        .unwrap_or(natural_frames);
    let t_dom_frames = t_dom_frames
        .max(natural_frames * 0.40)
        .min(natural_frames * 4.00);

    // Pass 3: adaptive local-max filter (radius scales with t_dom).
    let radius = ((t_dom_frames * 0.55).round() as usize).max(5);
    let filtered = filter_local_maxima(&score, &raw, radius, 0.65);
    let peaks = if filtered.len() >= 2 { filtered } else { raw };

    // Pass 4: final NMS with adaptive min_dist.
    let final_min_dist = ((t_dom_frames * 0.60).round() as usize).max(5);
    let peaks = nms_by_position(&peaks, final_min_dist);
    if peaks.len() < 2 {
        return count_daimoku_default(samples, sample_rate);
    }

    // Pass 5: content gate. This only ever *removes* peaks, and only
    // if doing so still leaves at least 2 — a gate that talks itself
    // down to "not enough to count" is more likely wrong than the
    // audio is, so in that case we keep the ungated peaks rather than
    // risk rejecting a real Daimoku because of one noisy feature.
    let gated = apply_content_gate(&peaks, samples, sample_rate, t_dom_frames, hop_secs, p);
    let peaks = if gated.len() >= 2 { gated } else { peaks };

    let count = peaks.len();
    let conf = peaks.iter().map(|&i| score[i]).sum::<f32>() / peaks.len() as f32;

    let silence_floor = 0.10 * p95;
    let min_gap_frames = 30usize;
    let phrases = find_active_segments(&smoothed, silence_floor, min_gap_frames);
    let phrase_count = phrases.len();

    let duration_secs = samples.len() as f32 / sample_rate as f32;
    let mean_period_ms = if count > 1 {
        let span = (peaks[count - 1] - peaks[0]) as f32 * hop_secs;
        span / (count - 1) as f32 * 1000.0
    } else {
        p.natural_period_ms
    };

    Some(DaimokuCountResult {
        count,
        confidence: conf.clamp(0.0, 1.0),
        phrase_count,
        mean_period_ms,
        method: "profile+matched".to_string(),

        period_ms: mean_period_ms,
        segment_count: phrase_count,
        duration_secs,
        active_duration_secs: duration_secs,
    })
}

// ---------------------------------------------------------------------------
// Content gate
// ---------------------------------------------------------------------------

/// Cheap "does this still sound like the trained voice" check applied
/// to the peaks the matched filter accepted. Compares the
/// zero-crossing rate of the raw audio around each peak to the band
/// learned from the user's own training takes (see `profile.rs`).
///
/// The margin is deliberately wide (±4 standard deviations, with a
/// floor so a very consistent profile doesn't end up with an
/// unrealistically narrow band): the goal is only to catch sounds
/// that are grossly different from recitation — a knock, a clap,
/// background speech at a very different pitch/timbre — not to act as
/// a strict per-syllable classifier, which a single scalar feature
/// like ZCR isn't precise enough to be.
fn apply_content_gate(
    peaks: &[usize],
    samples: &[f32],
    sample_rate: u32,
    t_dom_frames: f32,
    hop_secs: f32,
    profile: &PersonalProfile,
) -> Vec<usize> {
    if profile.zcr_median <= 0.0 && profile.zcr_sigma <= 0.0 {
        // Profile predates the zcr fields, or no take produced a
        // usable estimate — nothing to gate against.
        return peaks.to_vec();
    }

    let spread = profile
        .zcr_sigma
        .max(0.03 * profile.zcr_median.max(0.01))
        .max(0.02);
    let lo = (profile.zcr_median - 4.0 * spread).max(0.0);
    let hi = profile.zcr_median + 4.0 * spread;

    let half_samples =
        ((t_dom_frames * hop_secs * sample_rate as f32) / 2.0).round().max(1.0) as i64;

    peaks
        .iter()
        .copied()
        .filter(|&frame_idx| {
            let center_sample = (frame_idx as f32 * hop_secs * sample_rate as f32).round() as i64;
            let lo_s = (center_sample - half_samples).max(0) as usize;
            let hi_s = ((center_sample + half_samples).max(0) as usize).min(samples.len());
            if hi_s <= lo_s + 1 {
                // Not enough audio around this peak to judge — don't
                // reject blind.
                return true;
            }
            let z = zero_crossing_rate(&samples[lo_s..hi_s]);
            z >= lo && z <= hi
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Matched-filter helpers
// ---------------------------------------------------------------------------

fn resample_template(tmpl: &[f32], target_len: usize) -> Vec<f32> {
    if tmpl.is_empty() || target_len == 0 {
        return Vec::new();
    }
    let src_len = tmpl.len();
    if src_len == 1 || target_len == 1 {
        return vec![tmpl[0]; target_len];
    }
    let step = (src_len - 1) as f32 / (target_len - 1) as f32;
    let mut out = Vec::with_capacity(target_len);
    for k in 0..target_len {
        let pos = k as f32 * step;
        let i = pos.floor() as usize;
        let frac = pos - i as f32;
        let a = tmpl[i.min(src_len - 1)];
        let b = tmpl[(i + 1).min(src_len - 1)];
        out.push(a + (b - a) * frac);
    }
    out
}

fn normalized_xcorr(sig: &[f32], tmpl: &[f32]) -> Vec<f32> {
    let n = sig.len();
    let l = tmpl.len();
    if n < l || l < 4 {
        return Vec::new();
    }
    let lf = l as f32;

    let mu_t: f32 = tmpl.iter().sum::<f32>() / lf;
    let var_t: f32 = tmpl.iter().map(|x| (x - mu_t).powi(2)).sum::<f32>() / lf;
    let sigma_t = var_t.sqrt();
    if sigma_t < 1e-6 {
        return Vec::new();
    }

    let out_len = n - l + 1;
    let mut out = vec![0.0f32; out_len];

    let mut s: f32 = sig[..l].iter().sum();
    let mut s2: f32 = sig[..l].iter().map(|x| x * x).sum();

    for i in 0..out_len {
        let mu_s = s / lf;
        let var_s = (s2 / lf - mu_s * mu_s).max(0.0);
        let sigma_s = var_s.sqrt();

        let mut dot: f32 = 0.0;
        for j in 0..l {
            dot += sig[i + j] * tmpl[j];
        }

        let num = dot - lf * mu_s * mu_t;
        let den = lf * sigma_s * sigma_t;
        out[i] = if den > 1e-9 { num / den } else { 0.0 };

        if i + l < n {
            let out_old = sig[i];
            let in_new = sig[i + l];
            s += in_new - out_old;
            s2 += in_new * in_new - out_old * out_old;
        }
    }
    out
}

fn multi_scale_correlation(
    novelty: &[f32],
    template: &[f32],
    natural_frames: f32,
    ratio_range: (f32, f32),
) -> Vec<f32> {
    let n = novelty.len();
    if n < 30 || template.is_empty() || natural_frames < 10.0 {
        return Vec::new();
    }

    const SCALES: [f32; 8] = [0.40, 0.60, 0.80, 1.00, 1.30, 1.70, 2.20, 3.00];

    let lo = (ratio_range.0 * 0.70).max(0.30);
    let hi = (ratio_range.1 * 1.30).min(4.00);
    let mut scales: Vec<f32> = SCALES
        .iter()
        .copied()
        .filter(|s| *s >= lo && *s <= hi)
        .collect();
    if scales.is_empty() {
        scales.push(1.0);
    }

    let mut best = vec![0.0f32; n];

    for s in scales {
        let l = (natural_frames * s).round() as usize;
        if l < 10 || l > n / 2 {
            continue;
        }
        let tmpl_s = resample_template(template, l);
        if tmpl_s.len() != l {
            continue;
        }
        let corr = normalized_xcorr(novelty, &tmpl_s);
        if corr.is_empty() {
            continue;
        }
        let half = l / 2;
        for i in 0..corr.len() {
            let center = i + half;
            if center < n && corr[i] > best[center] {
                best[center] = corr[i];
            }
        }
    }
    best
}

fn pick_peaks(score: &[f32], min_dist: usize, threshold: f32) -> Vec<usize> {
    let n = score.len();
    if n < 3 {
        return Vec::new();
    }
    let mut candidates: Vec<usize> = Vec::new();
    for i in 1..n - 1 {
        if score[i] > score[i - 1] && score[i] >= score[i + 1] && score[i] >= threshold {
            candidates.push(i);
        }
    }
    candidates.sort_by(|&a, &b| {
        score[b]
            .partial_cmp(&score[a])
            .unwrap_or(Ordering::Equal)
    });
    let min_dist = min_dist.max(2);
    let mut kept: Vec<usize> = Vec::new();
    for p in candidates {
        let ok = kept.iter().all(|&q| {
            let d = if p > q { p - q } else { q - p };
            d >= min_dist
        });
        if ok {
            kept.push(p);
        }
    }
    kept.sort();
    kept
}

fn filter_local_maxima(
    score: &[f32],
    peaks: &[usize],
    radius: usize,
    rel_floor: f32,
) -> Vec<usize> {
    let n = score.len();
    let mut out = Vec::with_capacity(peaks.len());
    for &p in peaks {
        let lo = p.saturating_sub(radius);
        let hi = (p + radius + 1).min(n);
        let local_max = score[lo..hi].iter().cloned().fold(f32::MIN, f32::max);
        if score[p] >= rel_floor * local_max {
            out.push(p);
        }
    }
    out
}

/// Robust period from a peak-index list: sort intervals, drop the
/// shortest 40%, take the median. Removes attack→body intervals.
fn robust_period_from_peaks(peaks: &[usize]) -> Option<f32> {
    if peaks.len() < 3 {
        return None;
    }
    let mut dists: Vec<f32> = peaks
        .windows(2)
        .map(|w| (w[1] - w[0]) as f32)
        .collect();
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = (dists.len() as f32 * 0.40).floor() as usize;
    let hi = (dists.len() as f32 * 0.90).ceil() as usize;
    let lo = lo.min(dists.len().saturating_sub(1));
    let hi = hi.min(dists.len()).max(lo + 1);
    let slice = &dists[lo..hi];
    if slice.is_empty() {
        return None;
    }
    let mut v = slice.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(v[v.len() / 2])
}

/// NMS by position only, keeps peaks in order.
fn nms_by_position(peaks: &[usize], min_dist: usize) -> Vec<usize> {
    if peaks.len() < 2 {
        return peaks.to_vec();
    }
    let min_dist = min_dist.max(2);
    let mut out: Vec<usize> = Vec::with_capacity(peaks.len());
    for &p in peaks {
        if out.last().map_or(true, |&q| p.saturating_sub(q) >= min_dist) {
            out.push(p);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Post-processing (batch fallback path)
// ---------------------------------------------------------------------------

fn post_process_count(peaks: &[(u64, f32)]) -> usize {
    if peaks.is_empty() {
        return 0;
    }
    if peaks.len() < 3 {
        return peaks.len();
    }
    let t = robust_period_frames(peaks).unwrap_or(0.0);
    if t <= 0.0 {
        return peaks.len();
    }
    let min_dist = (0.55 * t) as u64;

    let mut idx: Vec<usize> = (0..peaks.len()).collect();
    idx.sort_by(|&a, &b| {
        peaks[b]
            .1
            .partial_cmp(&peaks[a].1)
            .unwrap_or(Ordering::Equal)
    });

    let mut kept: Vec<usize> = Vec::new();
    for i in idx {
        let f = peaks[i].0;
        let ok = kept.iter().all(|&j| {
            let q = peaks[j].0;
            let d = if f > q { f - q } else { q - f };
            d >= min_dist
        });
        if ok {
            kept.push(i);
        }
    }
    kept.len()
}

fn robust_period_frames(peaks: &[(u64, f32)]) -> Option<f64> {
    if peaks.len() < 3 {
        return None;
    }
    let mut dists: Vec<f64> = peaks
        .windows(2)
        .map(|w| (w[1].0 - w[0].0) as f64)
        .collect();
    dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let lo = (dists.len() as f64 * 0.40).floor() as usize;
    let hi = (dists.len() as f64 * 0.90).ceil() as usize;
    let lo = lo.min(dists.len().saturating_sub(1));
    let hi = hi.min(dists.len()).max(lo + 1);
    let slice = &dists[lo..hi];
    if slice.is_empty() {
        return None;
    }
    let mut v = slice.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    Some(v[v.len() / 2])
}

// ===========================================================================
// Streaming counter (used by tests and legacy paths)
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    Warming,
    Locked,
    Idle,
}

pub struct StreamingCounter {
    sr: u32,
    hop_samples: usize,
    pcm_ring: Vec<f32>,
    pcm_pos: usize,
    samples_until_hop: usize,

    smooth_val: f32,
    smooth_init: bool,
    alpha_smooth: f32,

    nov_s_val: f32,
    nov_init: bool,
    alpha_nov: f32,

    smooth_ring: Vec<f32>,
    nov_s_ring: Vec<f32>,
    ring_size: usize,
    ring_pos: usize,

    lag_frames: usize,
    lookahead: usize,
    local_win: usize,
    min_start_frames: u64,

    total_frames: u64,
    count: usize,
    last_peak_frame: Option<u64>,
    recent_intervals: Vec<f64>,

    accepted: Vec<(u64, f32)>,
    robust_count: usize,

    state: StreamingState,
}

impl StreamingCounter {
    pub fn new(sample_rate: u32) -> Self {
        let hop_ms = 10u32;
        let win_ms = 30u32;
        let hop_samples = (hop_ms as usize * sample_rate as usize) / 1000;
        let win_samples = (win_ms as usize * sample_rate as usize) / 1000;

        let alpha_smooth = 1.0 - (-(hop_ms as f32) / 200.0).exp();
        let alpha_nov = 1.0 - (-(hop_ms as f32) / 100.0).exp();

        let lag_frames = 30;
        let lookahead = 30;
        let local_win = 15;
        let ring_size = 400;
        let min_start_frames = (lag_frames + lookahead + local_win + 1) as u64;

        Self {
            sr: sample_rate,
            hop_samples: hop_samples.max(1),
            pcm_ring: vec![0.0; win_samples.max(1)],
            pcm_pos: 0,
            samples_until_hop: hop_samples.max(1),

            smooth_val: 0.0,
            smooth_init: false,
            alpha_smooth,

            nov_s_val: 0.0,
            nov_init: false,
            alpha_nov,

            smooth_ring: vec![0.0; ring_size],
            nov_s_ring: vec![0.0; ring_size],
            ring_size,
            ring_pos: 0,

            lag_frames,
            lookahead,
            local_win,
            min_start_frames,

            total_frames: 0,
            count: 0,
            last_peak_frame: None,
            recent_intervals: Vec::with_capacity(6),

            accepted: Vec::new(),
            robust_count: 0,

            state: StreamingState::Warming,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn robust_count(&self) -> usize {
        self.robust_count
    }
    pub fn state(&self) -> StreamingState {
        self.state
    }
    pub fn accepted_peaks(&self) -> &[(u64, f32)] {
        &self.accepted
    }

    pub fn period_secs(&self) -> Option<f32> {
        self.period_median_frames().map(|f| (f * 0.01) as f32)
    }

    fn period_median_frames(&self) -> Option<f64> {
        if self.recent_intervals.is_empty() {
            return None;
        }
        let mut v = self.recent_intervals.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let lo = (v.len() as f64 * 0.40).floor() as usize;
        let lo = lo.min(v.len().saturating_sub(1));
        let slice = &v[lo..];
        if slice.is_empty() {
            return None;
        }
        Some(slice[slice.len() / 2])
    }

    pub fn reset(&mut self) {
        let sr = self.sr;
        *self = StreamingCounter::new(sr);
    }

    pub fn push(&mut self, chunk: &[f32]) -> usize {
        for &s in chunk {
            self.pcm_ring[self.pcm_pos] = s;
            self.pcm_pos = (self.pcm_pos + 1) % self.pcm_ring.len();
            self.samples_until_hop = self.samples_until_hop.saturating_sub(1);
            if self.samples_until_hop == 0 {
                self.samples_until_hop = self.hop_samples;
                self.emit_frame();
            }
        }
        self.count
    }

    fn emit_frame(&mut self) {
        let mut sum_sq = 0.0f32;
        for &x in &self.pcm_ring {
            sum_sq += x * x;
        }
        let rms = (sum_sq / self.pcm_ring.len() as f32).sqrt();

        if !self.smooth_init {
            self.smooth_val = rms;
            self.smooth_init = true;
        } else {
            self.smooth_val =
                self.alpha_smooth * rms + (1.0 - self.alpha_smooth) * self.smooth_val;
        }
        self.smooth_ring[self.ring_pos] = self.smooth_val;

        let old_pos = (self.ring_pos + self.ring_size - self.lag_frames) % self.ring_size;
        let old_val = self.smooth_ring[old_pos];
        let nov = (self.smooth_val - old_val).max(0.0);

        if !self.nov_init {
            self.nov_s_val = nov;
            self.nov_init = true;
        } else {
            self.nov_s_val =
                self.alpha_nov * nov + (1.0 - self.alpha_nov) * self.nov_s_val;
        }
        self.nov_s_ring[self.ring_pos] = self.nov_s_val;

        self.ring_pos = (self.ring_pos + 1) % self.ring_size;
        self.total_frames += 1;

        if self.total_frames < self.min_start_frames {
            self.state = StreamingState::Warming;
            return;
        }
        if self.smooth_val < 1e-4 {
            self.state = StreamingState::Idle;
        } else if self.count >= 2 {
            self.state = StreamingState::Locked;
        } else {
            self.state = StreamingState::Warming;
        }

        self.check_candidate();
    }

    fn check_candidate(&mut self) {
        let n = self.ring_size;
        let cur = (self.ring_pos + n - 1) % n;
        let cand = (cur + n - self.lookahead) % n;

        let v = self.nov_s_ring[cand];
        if v <= 1e-6 {
            return;
        }

        let lw = self.local_win;
        for off in 1..=lw {
            let older = (cand + n - off) % n;
            let newer = (cand + off) % n;
            if self.nov_s_ring[older] > v {
                return;
            }
            if self.nov_s_ring[newer] >= v {
                return;
            }
        }

        let frame_idx = self.total_frames - 1 - self.lookahead as u64;
        if let Some(prev) = self.last_peak_frame {
            let dist = frame_idx.saturating_sub(prev);
            let min_dist = self
                .period_median_frames()
                .map(|p| (0.55 * p).max(50.0) as u64)
                .unwrap_or(50);
            if dist < min_dist {
                return;
            }
        }

        if let Some(prev) = self.last_peak_frame {
            let dist = (frame_idx - prev) as f64;
            self.recent_intervals.push(dist);
            if self.recent_intervals.len() > 6 {
                self.recent_intervals.remove(0);
            }
        }
        self.accepted.push((frame_idx, v));
        self.last_peak_frame = Some(frame_idx);
        self.count += 1;

        let rc = post_process_count(&self.accepted);
        if rc > self.robust_count {
            self.robust_count = rc;
        }
    }

    pub fn finish(&self) -> Option<DaimokuCountResult> {
        if self.robust_count == 0 {
            return None;
        }
        let period_secs = self.period_secs();
        let mean_period_ms = period_secs.map(|p| p * 1000.0).unwrap_or(0.0);
        let conf = match period_secs {
            Some(p) if p >= 0.5 && p <= 5.0 => 0.8,
            Some(_) => 0.4,
            None => 0.3,
        };
        Some(DaimokuCountResult {
            count: self.robust_count,
            confidence: conf,
            phrase_count: 0,
            mean_period_ms,
            method: "streaming".to_string(),

            period_ms: mean_period_ms,
            segment_count: 0,
            duration_secs: self.total_frames as f32 * 0.01,
            active_duration_secs: 0.0,
        })
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

    fn phrase_with_bumps(
        bumps: usize,
        period_s: f32,
        strong_first: bool,
        weak: &[usize],
        rate: u32,
    ) -> Vec<f32> {
        let n = (bumps as f32 * period_s * rate as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / rate as f32;
            let car = (2.0 * PI * 220.0 * t).sin() + 0.4 * (2.0 * PI * 660.0 * t).sin();
            let phase = (t / period_s).fract();
            let bump = 0.55 + 0.45 * (PI * phase).sin().powf(0.6);
            let ripple = 1.0 + 0.10 * (2.0 * PI * t / 0.15).sin();
            let attack = if strong_first && t < period_s { 1.7 } else { 1.0 };
            let k = (t / period_s) as usize;
            let atten = if weak.contains(&k) { 0.55 } else { 1.0 };
            v.push(car * bump * ripple * attack * atten * 0.4);
        }
        v
    }

    fn phrase_with_sub_peaks(bumps: usize, period_s: f32, rate: u32) -> Vec<f32> {
        let n = (bumps as f32 * period_s * rate as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / rate as f32;
            let phase = (t / period_s).fract();
            let car = (2.0 * PI * 220.0 * t).sin() + 0.3 * (2.0 * PI * 660.0 * t).sin();
            let primary = if phase < 0.15 {
                1.0
            } else if phase < 0.55 {
                0.60
            } else if phase < 0.75 {
                0.75
            } else {
                0.55
            };
            v.push(car * primary * 0.4);
        }
        v
    }

    fn build_profile() -> PersonalProfile {
        let mut p = PersonalProfile::default();
        p.add_take(&phrase_with_bumps(8, 1.20, true, &[], SR), SR, 8)
            .expect("natural take");
        p.add_take(&phrase_with_bumps(3, 3.50, false, &[], SR), SR, 3)
            .expect("slow take");
        p.add_take(&phrase_with_bumps(10, 0.90, false, &[], SR), SR, 10)
            .expect("fast take");
        p
    }

    #[test]
    fn no_profile_falls_back() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let r = count_daimoku_with_profile(&v, SR, None).expect("fallback count");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
        assert!(r.method.starts_with("novelty"));
    }

    #[test]
    fn empty_profile_falls_back() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let p = PersonalProfile::default();
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("fallback");
        assert!(r.method.starts_with("novelty"));
    }

    #[test]
    fn profile_counts_natural_take() {
        let p = build_profile();
        let v = phrase_with_bumps(10, 1.20, true, &[], SR);
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert_eq!(r.method, "profile+matched");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn profile_counts_slow_take() {
        let p = build_profile();
        let v = phrase_with_bumps(2, 4.0, false, &[], SR);
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert!(
            (1..=3).contains(&r.count),
            "2 slow Daimoku: got {}",
            r.count
        );
    }

    #[test]
    fn profile_counts_fast_take() {
        let p = build_profile();
        let v = phrase_with_bumps(10, 0.85, false, &[], SR);
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn profile_handles_sub_peaks() {
        let p = build_profile();
        let v = phrase_with_sub_peaks(10, 1.20, SR);
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn profile_handles_speed_change_within_take() {
        let p = build_profile();
        let mut v = phrase_with_bumps(5, 0.90, false, &[], SR);
        v.extend(phrase_with_bumps(5, 2.80, false, &[], SR));
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert!((8..=12).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn profile_handles_strong_first_attack() {
        let p = build_profile();
        let n_bumps = 10;
        let period_s = 1.20_f32;
        let n = (n_bumps as f32 * period_s * SR as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / SR as f32;
            let car = (2.0 * PI * 220.0 * t).sin() + 0.4 * (2.0 * PI * 660.0 * t).sin();
            let phase = (t / period_s).fract();
            let bump = 0.55 + 0.45 * (PI * phase).sin().powf(0.6);
            let attack = if t < period_s { 5.0 } else { 1.0 };
            v.push(car * bump * attack * 0.4);
        }
        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn content_gate_rejects_noise_bursts_with_different_timbre() {
        // Train on the usual tonal Daimoku phrase, then feed a take
        // where a burst of broadband noise (very different ZCR) is
        // spliced in at plausible daimoku spacing. The gate should not
        // let the counter blindly accept it as extra Daimoku.
        let p = build_profile();
        let mut v = phrase_with_bumps(10, 1.20, true, &[], SR);

        // Simple deterministic pseudo-noise burst (no external crate
        // needed): a fast alternating high/low signal, which has a
        // much higher zero-crossing-rate than the tonal phrase.
        let burst_len = (0.4 * SR as f32) as usize;
        let noise: Vec<f32> = (0..burst_len)
            .map(|i| if i % 2 == 0 { 0.8 } else { -0.8 })
            .collect();
        v.extend(noise);

        let r = count_daimoku_with_profile(&v, SR, Some(&p)).expect("count");
        // Without the gate this could plausibly read as 11; the gate
        // should keep it close to the true 10.
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn resample_template_length() {
        let t: Vec<f32> = (0..128).map(|i| i as f32).collect();
        for len in [16usize, 32, 64, 128, 200, 384] {
            assert_eq!(resample_template(&t, len).len(), len);
        }
    }

    #[test]
    fn resample_template_endpoints() {
        let t = [0.0f32, 1.0, 2.0, 3.0];
        let r = resample_template(&t, 7);
        assert_eq!(r.len(), 7);
        assert!((r[0] - 0.0).abs() < 1e-4);
        assert!((r[6] - 3.0).abs() < 1e-4);
    }

    #[test]
    fn xcorr_self_is_one() {
        let sig: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        let c = normalized_xcorr(&sig, &sig);
        assert!(!c.is_empty());
        assert!((c[0] - 1.0).abs() < 1e-3, "self {}", c[0]);
    }

    #[test]
    fn xcorr_flat_signal_is_zero() {
        let sig = vec![0.5f32; 100];
        let tmpl: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let c = normalized_xcorr(&sig, &tmpl);
        assert!(c.iter().all(|&x| x.abs() < 1e-6));
    }

    #[test]
    fn pick_peaks_enforces_min_distance() {
        let mut score = vec![0.0f32; 100];
        score[10] = 0.9;
        score[20] = 0.8;
        score[50] = 0.85;
        assert_eq!(pick_peaks(&score, 15, 0.5), vec![10, 50]);
    }

    #[test]
    fn pick_peaks_respects_threshold() {
        let mut score = vec![0.0f32; 100];
        score[10] = 0.2;
        score[50] = 0.9;
        assert_eq!(pick_peaks(&score, 15, 0.5), vec![50]);
    }
}