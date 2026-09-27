//! Daimoku counter: batch + streaming.
//!
//! Batch:
//!   1. RMS envelope → smoothing 200 ms.
//!   2. Phrase segmentation by silence gaps (≥ 300 ms).
//!   3. Per phrase: ACF period estimate (fundamental, not octave),
//!      peak detection, NMS 0.55·T, gap recovery, strict verify.
//!   4. Sum across phrases. Fallback period = median of phrase estimates.
//!
//! Streaming:
//!   `StreamingCounter::push(chunk)` → running count.
//!   Silence-aware: baseline snaps down during gaps, so the restart
//!   after a breath doesn't produce a spurious peak.

use serde::Serialize;
use std::cmp::Ordering;

// ===========================================================================
// Batch API
// ===========================================================================

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub window_ms: u32,
    pub hop_ms: u32,
    pub smooth_secs: f32,
    pub detrend_secs: f32,

    pub silence_rel: f32,
    pub min_pause_secs: f32,

    pub peak_prom_rel: f32,
    pub peak_prom_win_secs: f32,

    pub nms_ratio: f32,
    pub min_period_secs: f32,
    pub max_period_secs: f32,
    pub acf_min_peak: f32,

    pub max_cv: f32,
    pub max_gap_ratio: f32,
    pub min_confidence: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_ms: 30,
            hop_ms: 10,
            smooth_secs: 0.20,
            detrend_secs: 4.0,

            silence_rel: 0.10,
            min_pause_secs: 0.30,

            peak_prom_rel: 0.20,
            peak_prom_win_secs: 0.75,

            nms_ratio: 0.55,
            min_period_secs: 0.50,
            max_period_secs: 5.00,
            acf_min_peak: 0.08,

            max_cv: 0.35,
            max_gap_ratio: 1.80,
            min_confidence: 0.25,
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
    // legacy fields (front-end compatibility)
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
    let hop_secs = cfg.hop_ms as f32 / 1000.0;
    let n = env.len();
    if n < 30 {
        return None;
    }

    let p95 = percentile(&env, 0.95);
    if p95 < 1e-5 {
        return None;
    }
    let p10 = percentile(&env, 0.10);
    if p10 > 0.90 * p95 {
        return None; // steady tone, no modulation
    }

    let sm_frames = ((cfg.smooth_secs / hop_secs).round() as usize).max(1);
    let smoothed = moving_average(&env, sm_frames);

    // --- Phrase segmentation by silence --------------------------------
    let silence_floor = cfg.silence_rel * p95;
    let min_gap_frames = ((cfg.min_pause_secs / hop_secs).round() as usize).max(2);
    let phrases = find_active_segments(&smoothed, silence_floor, min_gap_frames);
    if phrases.is_empty() {
        return None;
    }

    // --- Per-phrase period estimation ----------------------------------
    let mut per_phrase_period: Vec<Option<f32>> = Vec::with_capacity(phrases.len());
    for (s, e) in &phrases {
        per_phrase_period.push(estimate_period_acf(&smoothed[*s..*e], hop_secs, cfg));
    }

    let mut valid_periods: Vec<f32> = per_phrase_period.iter().flatten().copied().collect();
    let fallback_period = if !valid_periods.is_empty() {
        median(&mut valid_periods)
    } else {
        // Last resort: ACF on the whole smoothed signal.
        estimate_period_acf(&smoothed, hop_secs, cfg)?
    };

    // --- Count bumps per phrase ----------------------------------------
    let mut total = 0usize;
    let mut used = 0usize;
    let mut conf_sum = 0.0f32;
    let mut period_sum = 0.0f32;
    let mut active_frames = 0usize;

    for ((s, e), per) in phrases.iter().zip(per_phrase_period.iter()) {
        let seg = &smoothed[*s..*e];
        let seg_len = seg.len();
        if seg_len < 10 {
            continue;
        }
        let period = per.unwrap_or(fallback_period);
        if period < cfg.min_period_secs || period > cfg.max_period_secs {
            continue;
        }

        // Detrend the phrase.
        let detrend_frames = ((cfg.detrend_secs / hop_secs).round() as usize)
            .max(3)
            .min(seg_len / 2);
        let trend = moving_average(seg, detrend_frames);
        let detrended: Vec<f32> = seg
            .iter()
            .zip(trend.iter())
            .map(|(a, b)| a - b)
            .collect();

        let cands = find_peaks(&detrended, hop_secs, cfg);
        if cands.len() < 2 {
            continue;
        }

        let kept = nms(&cands, &detrended, cfg.nms_ratio * period, hop_secs);
        if kept.len() < 2 {
            continue;
        }

        let recovered = recover_missing_peaks(kept, &detrended, period, hop_secs);
        if recovered.len() < 2 {
            continue;
        }

        if !verify_phrase(&recovered, period, hop_secs, seg_len, cfg) {
            continue;
        }

        // Phrase confidence from gap CV.
        let dists: Vec<f32> = recovered
            .windows(2)
            .map(|w| (w[1] - w[0]) as f32 * hop_secs)
            .collect();
        let mean_p = dists.iter().sum::<f32>() / dists.len() as f32;
        let cv = if dists.len() >= 2 {
            let var: f32 = dists
                .iter()
                .map(|d| (d - mean_p).powi(2))
                .sum::<f32>()
                / dists.len() as f32;
            var.sqrt() / mean_p.max(1e-6)
        } else {
            0.0
        };

        total += recovered.len();
        conf_sum += (1.0 - cv).clamp(0.0, 1.0);
        used += 1;
        period_sum += mean_p;
        active_frames += seg_len;
    }

    if total < 3 || used == 0 {
        return None;
    }

    let mean_conf = conf_sum / used as f32;
    if mean_conf < cfg.min_confidence {
        return None;
    }

    let mean_period_ms = (period_sum / used as f32) * 1000.0;
    let duration_secs = samples.len() as f32 / sample_rate as f32;
    let active_secs = active_frames as f32 * hop_secs;

    Some(DaimokuCountResult {
        count: total,
        confidence: mean_conf,
        phrase_count: used,
        mean_period_ms,
        method: "phrases+peaks".to_string(),

        period_ms: mean_period_ms,
        segment_count: used,
        duration_secs,
        active_duration_secs: active_secs,
    })
}

// ---------------------------------------------------------------------------
// Envelope / smoothing
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Phrase segmentation
// ---------------------------------------------------------------------------

fn find_active_segments(env: &[f32], floor: f32, min_gap: usize) -> Vec<(usize, usize)> {
    let n = env.len();
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut last_active = 0usize;
    let mut silent_run = 0usize;

    for i in 0..n {
        if env[i] > floor {
            if start.is_none() {
                start = Some(i);
            }
            last_active = i;
            silent_run = 0;
        } else {
            silent_run += 1;
            if silent_run >= min_gap {
                if let Some(s) = start.take() {
                    if last_active >= s {
                        out.push((s, last_active + 1));
                    }
                }
            }
        }
    }
    if let Some(s) = start {
        if last_active >= s {
            out.push((s, last_active + 1));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Peaks
// ---------------------------------------------------------------------------

fn find_peaks(sig: &[f32], hop: f32, cfg: &Config) -> Vec<usize> {
    let n = sig.len();
    if n < 3 {
        return Vec::new();
    }
    let gmax = sig.iter().cloned().fold(f32::MIN, f32::max);
    let gmin = sig.iter().cloned().fold(f32::MAX, f32::min);
    let range = gmax - gmin;
    if range < 1e-9 {
        return Vec::new();
    }
    let min_prom = cfg.peak_prom_rel * range;
    let win = ((cfg.peak_prom_win_secs / hop).round() as usize).max(3);

    let mut out = Vec::new();
    for i in 1..n - 1 {
        if sig[i] > sig[i - 1] && sig[i] >= sig[i + 1] {
            let lo = i.saturating_sub(win);
            let hi = (i + win + 1).min(n);
            let local_min = sig[lo..hi].iter().cloned().fold(f32::MAX, f32::min);
            if sig[i] - local_min >= min_prom {
                out.push(i);
            }
        }
    }
    out
}

fn nms(peaks: &[usize], values: &[f32], min_dist_secs: f32, hop: f32) -> Vec<usize> {
    let min_dist = ((min_dist_secs / hop).round() as usize).max(2);
    let mut sorted = peaks.to_vec();
    sorted.sort_by(|&a, &b| {
        values[b]
            .partial_cmp(&values[a])
            .unwrap_or(Ordering::Equal)
    });
    let mut kept: Vec<usize> = Vec::new();
    for p in sorted {
        if kept.iter().all(|&q| {
            let d = if p > q { p - q } else { q - p };
            d >= min_dist
        }) {
            kept.push(p);
        }
    }
    kept.sort();
    kept
}

fn recover_missing_peaks(
    mut peaks: Vec<usize>,
    sig: &[f32],
    period: f32,
    hop: f32,
) -> Vec<usize> {
    if peaks.len() < 2 {
        return peaks;
    }
    let t_frames = (period / hop).round() as usize;
    if t_frames < 3 {
        return peaks;
    }
    let gmin = sig.iter().cloned().fold(f32::MAX, f32::min);
    let gmax = sig.iter().cloned().fold(f32::MIN, f32::max);
    let range = (gmax - gmin).max(1e-9);
    let abs_floor = gmin + 0.10 * range;

    for _pass in 0..4 {
        let mut inserted = false;
        let mut out: Vec<usize> = Vec::with_capacity(peaks.len() + 2);
        out.push(peaks[0]);
        for w in peaks.windows(2) {
            let a = w[0];
            let b = w[1];
            let gap = b - a;
            if (gap as f32) <= 1.4 * t_frames as f32 {
                out.push(b);
                continue;
            }
            let n_expected = (gap as f32 / t_frames as f32).round() as usize;
            let to_insert = n_expected.saturating_sub(1);
            if to_insert >= 1 {
                let neighbor_min = sig[a].min(sig[b]);
                let soft_floor = 0.35 * neighbor_min;
                for k in 1..=to_insert {
                    let center =
                        a + (k as f32 * gap as f32 / (to_insert + 1) as f32).round() as usize;
                    let half = t_frames / 2;
                    let lo = center.saturating_sub(half).max(a + 1);
                    let hi = (center + half + 1).min(b).min(sig.len());
                    if lo + 1 >= hi {
                        continue;
                    }
                    let mut best_i: Option<usize> = None;
                    for i in lo..hi {
                        if i == 0 || i + 1 >= sig.len() {
                            continue;
                        }
                        if sig[i] > sig[i - 1] && sig[i] >= sig[i + 1] {
                            if best_i.map_or(true, |bi| sig[i] > sig[bi]) {
                                best_i = Some(i);
                            }
                        }
                    }
                    if let Some(i) = best_i {
                        if sig[i] >= abs_floor && sig[i] >= soft_floor {
                            out.push(i);
                            inserted = true;
                        }
                    }
                }
            }
            out.push(b);
        }
        out.sort();
        out.dedup();
        peaks = out;
        if !inserted {
            break;
        }
    }
    peaks
}

// ---------------------------------------------------------------------------
// Period estimation
// ---------------------------------------------------------------------------

fn estimate_period_acf(sig: &[f32], hop: f32, cfg: &Config) -> Option<f32> {
    let n = sig.len();
    if n < 60 {
        return None;
    }
    let min_lag = ((cfg.min_period_secs / hop).round() as usize).max(3);
    let max_lag = ((cfg.max_period_secs / hop).round() as usize).min(n / 2);
    if min_lag >= max_lag {
        return None;
    }

    let mean: f32 = sig.iter().sum::<f32>() / n as f32;
    let c: Vec<f32> = sig.iter().map(|x| x - mean).collect();

    let mut corr = vec![0.0f32; max_lag + 1];
    for lag in min_lag..=max_lag {
        let m = n - lag;
        if m == 0 {
            break;
        }
        let mut num = 0.0f32;
        let mut da = 0.0f32;
        let mut db = 0.0f32;
        for i in 0..m {
            num += c[i] * c[i + lag];
            da += c[i] * c[i];
            db += c[i + lag] * c[i + lag];
        }
        let den = (da * db).sqrt();
        if den > 1e-9 {
            corr[lag] = num / den;
        }
    }

    let mut lms: Vec<(usize, f32)> = Vec::new();
    for lag in (min_lag + 1)..max_lag {
        if corr[lag] > corr[lag - 1] && corr[lag] >= corr[lag + 1] {
            lms.push((lag, corr[lag]));
        }
    }
    if lms.is_empty() {
        return None;
    }
    let (g_lag, g_corr) = *lms
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))?;
    if g_corr < cfg.acf_min_peak {
        return None;
    }
    let thr = 0.60 * g_corr;
    let (lag, _) = lms
        .iter()
        .filter(|(_, c)| *c >= thr)
        .min_by_key(|(l, _)| *l)
        .copied()
        .unwrap_or((g_lag, g_corr));

    let refined = if lag > 0 && lag + 1 < corr.len() {
        let y0 = corr[lag - 1];
        let y1 = corr[lag];
        let y2 = corr[lag + 1];
        let den = 2.0 * y1 - y0 - y2;
        if den.abs() > 1e-9 {
            let d = ((y2 - y0) / (2.0 * den)).clamp(-0.5, 0.5);
            (lag as f32 + d).max(1.0)
        } else {
            lag as f32
        }
    } else {
        lag as f32
    };
    Some(refined * hop)
}

// ---------------------------------------------------------------------------
// Verification (per phrase, lenient)
// ---------------------------------------------------------------------------

fn verify_phrase(kept: &[usize], period: f32, hop: f32, seg_len: usize, cfg: &Config) -> bool {
    if kept.len() < 2 {
        return false;
    }
    let dists: Vec<f32> = kept
        .windows(2)
        .map(|w| (w[1] - w[0]) as f32 * hop)
        .collect();
    let mean = dists.iter().sum::<f32>() / dists.len() as f32;
    if mean < cfg.min_period_secs * 0.7 || mean > cfg.max_period_secs * 1.3 {
        return false;
    }
    if dists.len() >= 2 {
        let var: f32 = dists
            .iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f32>()
            / dists.len() as f32;
        let cv = var.sqrt() / mean.max(1e-6);
        if cv > cfg.max_cv * 1.5 {
            return false;
        }
        let d_min = dists.iter().cloned().fold(f32::MAX, f32::min);
        let d_max = dists.iter().cloned().fold(f32::MIN, f32::max);
        if d_min > 1e-6 && d_max / d_min > cfg.max_gap_ratio * 1.5 {
            return false;
        }
    }
    // Consistent with ACF period
    let ratio = mean / period.max(1e-6);
    if ratio < 0.65 || ratio > 1.50 {
        return false;
    }
    // Peaks span a reasonable fraction of the phrase
    if kept.len() >= 2 {
        let span = (kept[kept.len() - 1] - kept[0]) as f32 * hop;
        let total = seg_len as f32 * hop;
        if span < 0.30 * total {
            return false;
        }
    }
    true
}

fn percentile(data: &[f32], p: f32) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut s = data.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let idx = (((s.len() - 1) as f32) * p).round() as usize;
    s[idx]
}

fn median(data: &mut [f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let m = data.len() / 2;
    if data.len() % 2 == 0 {
        (data[m - 1] + data[m]) * 0.5
    } else {
        data[m]
    }
}

// ===========================================================================
// Streaming counter
// ===========================================================================
//
// Robust against the three freeze modes:
//   - amplitude drift (self-adapting level = max over a 2 s ring, not an EMA)
//   - temporary pause / volume drop (asymmetric baseline follower)
//   - speed change (period = median of the last 5 intervals, not an EMA)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    Warming,
    Locked,
    Idle,
}

pub struct StreamingCounter {
    sr: u32,

    // PCM / envelope
    hop_samples: usize,
    pcm_ring: Vec<f32>,
    pcm_pos: usize,
    samples_until_hop: usize,

    // Envelope smoothing (200 ms TC)
    smooth_val: f32,
    smooth_init: bool,
    alpha: f32,

    // Asymmetric baseline follower:
    //   fast down (100 ms): tracks troughs, follows volume drops
    //   slow up   (3 s):    ignores peaks, doesn't drift upward
    baseline_val: f32,
    baseline_init: bool,
    down_coef: f32,
    up_coef: f32,

    // Detrended ring of d = smooth - baseline (always >= 0)
    det_ring: Vec<f32>,
    det_pos: usize,
    det_filled: usize,
    ring_len: usize,
    lookahead: usize,
    local_win: usize,

    // Counters
    total_frames: u64,
    count: usize,
    last_peak_frame: Option<u64>,
    recent_intervals: Vec<f64>, // capped at 5, used for median period

    // State
    state: StreamingState,
    frames_since_signal: u64,
}

impl StreamingCounter {
    pub fn new(sample_rate: u32) -> Self {
        let hop_ms = 10u32;
        let win_ms = 30u32;
        let hop_samples = (hop_ms as usize * sample_rate as usize) / 1000;
        let win_samples = (win_ms as usize * sample_rate as usize) / 1000;

        // Time constants (per-frame blend coefficients).
        let alpha = 1.0 - (-(hop_ms as f32) / 200.0).exp();      // 200 ms smooth
        let down_coef = 1.0 - (-(hop_ms as f32) / 100.0).exp();  // 100 ms down
        let up_coef = 1.0 - (-(hop_ms as f32) / 3000.0).exp();   // 3 s up

        let ring_len = 200usize; // 2 s @ 10 ms hop
        let lookahead = 30usize; // 300 ms
        let local_win = 20usize; // ±200 ms

        Self {
            sr: sample_rate,
            hop_samples: hop_samples.max(1),
            pcm_ring: vec![0.0; win_samples.max(1)],
            pcm_pos: 0,
            samples_until_hop: hop_samples.max(1),

            smooth_val: 0.0,
            smooth_init: false,
            alpha,

            baseline_val: 0.0,
            baseline_init: false,
            down_coef,
            up_coef,

            det_ring: vec![0.0; ring_len],
            det_pos: 0,
            det_filled: 0,
            ring_len,
            lookahead,
            local_win,

            total_frames: 0,
            count: 0,
            last_peak_frame: None,
            recent_intervals: Vec::with_capacity(5),

            state: StreamingState::Warming,
            frames_since_signal: 0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn state(&self) -> StreamingState {
        self.state
    }

    pub fn period_secs(&self) -> Option<f32> {
        self.period_frames_median().map(|f| (f * 0.01) as f32)
    }

    fn period_frames_median(&self) -> Option<f64> {
        if self.recent_intervals.is_empty() {
            return None;
        }
        let mut v = self.recent_intervals.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        Some(v[v.len() / 2])
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
        // RMS over the PCM ring.
        let mut sum_sq = 0.0f32;
        for &x in &self.pcm_ring {
            sum_sq += x * x;
        }
        let rms = (sum_sq / self.pcm_ring.len() as f32).sqrt();

        // Smooth (200 ms TC).
        if !self.smooth_init {
            self.smooth_val = rms;
            self.smooth_init = true;
        } else {
            self.smooth_val = self.alpha * rms + (1.0 - self.alpha) * self.smooth_val;
        }

        // Asymmetric baseline follower.
        if !self.baseline_init {
            self.baseline_val = self.smooth_val;
            self.baseline_init = true;
        } else if self.smooth_val < self.baseline_val {
            let diff = self.smooth_val - self.baseline_val;
            self.baseline_val += self.down_coef * diff; // fast down
        } else {
            let diff = self.smooth_val - self.baseline_val;
            self.baseline_val += self.up_coef * diff; // slow up
        }

        // Non-negative deviation.
        let d = self.smooth_val - self.baseline_val;

        // Signal presence (for state reporting only).
        if self.baseline_val > 1e-4 && self.smooth_val > 0.15 * self.baseline_val {
            self.frames_since_signal = 0;
        } else {
            self.frames_since_signal = self.frames_since_signal.saturating_add(1);
        }

        // Push into detrended ring.
        self.det_ring[self.det_pos] = d;
        self.det_pos = (self.det_pos + 1) % self.ring_len;
        self.det_filled = (self.det_filled + 1).min(self.ring_len);
        self.total_frames += 1;

        // State transition.
        if self.frames_since_signal > 100 {
            self.state = StreamingState::Idle;
        } else if self.count >= 2 {
            self.state = StreamingState::Locked;
        } else {
            self.state = StreamingState::Warming;
        }

        // After a long pause, forget the period AND the last peak, so the
        // first new peak after the breath is not blocked by stale state.
        if self.frames_since_signal > 150 {
            self.recent_intervals.clear();
            self.last_peak_frame = None;
        }

        self.check_candidate();
    }

    fn check_candidate(&mut self) {
        let n = self.ring_len;
        let min_filled = self.lookahead + self.local_win + 1;
        if self.det_filled < min_filled {
            return;
        }

        let center = (self.det_pos + n - 1 - self.lookahead) % n;
        let v = self.det_ring[center];

        // Strict local max over [center - lw, center + lw].
        let lw = self.local_win;
        for off in 1..=lw {
            let i1 = (center + n - off) % n;
            let i2 = (center + off) % n;
            if self.det_ring[i1] >= v {
                return;
            }
            if self.det_ring[i2] >= v {
                return;
            }
        }

        // Level = max over the filled part of the ring (adapts to volume
        // changes in ~2 s, never freezes).
        let filled = self.det_filled;
        let start = (self.det_pos + n - filled) % n;
        let mut level = f32::MIN;
        for k in 0..filled {
            let i = (start + k) % n;
            if self.det_ring[i] > level {
                level = self.det_ring[i];
            }
        }
        if level <= 1e-6 {
            return;
        }
        if v < 0.30 * level {
            return;
        }

        let frame_idx = self.total_frames - 1 - self.lookahead as u64;

        // Min distance to previous peak: lenient (0.40 * period) so a
        // sudden speed-up is not blocked.
        if let Some(prev) = self.last_peak_frame {
            let dist = frame_idx.saturating_sub(prev);
            let min_dist = self
                .period_frames_median()
                .map(|p| (0.40 * p).max(20.0) as u64)
                .unwrap_or(20);
            if dist < min_dist {
                return;
            }
        }

        // Accept.
        if let Some(prev) = self.last_peak_frame {
            let dist = (frame_idx - prev) as f64;
            self.recent_intervals.push(dist);
            if self.recent_intervals.len() > 5 {
                self.recent_intervals.remove(0);
            }
        }

        self.last_peak_frame = Some(frame_idx);
        self.count += 1;
    }

    pub fn finish(&self) -> Option<DaimokuCountResult> {
        if self.count == 0 {
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
            count: self.count,
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

    fn silence(secs: f32) -> Vec<f32> {
        vec![0.0; (secs * SR as f32) as usize]
    }

    fn phrase_with_bumps(bumps: usize, period_s: f32, strong_first: bool, weak: &[usize], rate: u32) -> Vec<f32> {
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

    /// THE USER'S CASE: 8 Daimoku + 1s breath + 2 Daimoku, total 10.
    #[test]
    fn eight_plus_two_phrases() {
        let t1 = 1.05;
        let t2 = 1.50;
        let mut v = phrase_with_bumps(8, t1, true, &[], SR);
        v.extend(silence(1.0));
        v.extend(phrase_with_bumps(2, t2, false, &[], SR));
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default())
            .expect("must count 8+2");
        assert_eq!(r.count, 10, "expected 10, got {} (conf {:.2}, method {})",
                   r.count, r.confidence, r.method);
    }

        /// Volume drop: 5 loud + 5 quiet. Must count 10, not freeze at 5.
    #[test]
    fn streaming_handles_amplitude_drop() {
        let loud = phrase_with_bumps(5, 1.05, true, &[], SR);
        let quiet: Vec<f32> = phrase_with_bumps(5, 1.05, false, &[], SR)
            .iter()
            .map(|x| x * 0.35)
            .collect();
        let mut v = loud;
        v.extend(quiet);

        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let mut i = 0;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            c.push(&v[i..end]);
            i = end;
        }
        assert!(
            (9..=11).contains(&c.count()),
            "amplitude drop: expected ~10, got {}",
            c.count()
        );
    }

    /// Speed change: first 5 slow (1.5 s), then 5 fast (0.9 s). Must not freeze.
    #[test]
    fn streaming_handles_speed_change() {
        let mut v = phrase_with_bumps(5, 1.50, true, &[], SR);
        v.extend(phrase_with_bumps(5, 0.90, false, &[], SR));
        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let mut i = 0;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            c.push(&v[i..end]);
            i = end;
        }
        assert!(
            (9..=11).contains(&c.count()),
            "speed change: expected ~10, got {}",
            c.count()
        );
    }

    #[test]
    fn empty_and_silence_return_none() {
        assert!(count_daimoku(&[], SR, DaimokuCountConfig::default()).is_none());
        assert!(count_daimoku(&silence(5.0), SR, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn steady_tone_returns_none() {
        let n = SR as usize * 10;
        let v: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 440.0 * i as f32 / SR as f32).sin() * 0.5)
            .collect();
        assert!(count_daimoku(&v, SR, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn ten_daimoku_continuous() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default()).expect("count");
        assert!((9..=11).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn thirty_bumps_fast() {
        let v = phrase_with_bumps(30, 1.43, false, &[], SR);
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default()).expect("count");
        assert!((29..=31).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn slow_daimoku() {
        let v = phrase_with_bumps(5, 4.0, false, &[], SR);
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default()).expect("count");
        assert!((4..=6).contains(&r.count), "got {}", r.count);
    }

    #[test]
    fn streaming_counts_ten_continuous() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let mut i = 0;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            c.push(&v[i..end]);
            i = end;
        }
        assert!((9..=11).contains(&c.count()), "streaming got {}", c.count());
    }

    /// THE USER'S CASE for streaming: 8 + silence + 2 must give 10, not 11.
    #[test]
    fn streaming_eight_plus_two_after_breath() {
        let mut v = phrase_with_bumps(8, 1.05, true, &[], SR);
        v.extend(silence(1.0));
        v.extend(phrase_with_bumps(2, 1.50, false, &[], SR));
        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let mut i = 0;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            c.push(&v[i..end]);
            i = end;
        }
        assert!(
            (9..=10).contains(&c.count()),
            "streaming expected 10 (not 11), got {}",
            c.count()
        );
    }

    #[test]
    #[ignore]
    fn real_wav_batch() {
        let path = std::path::Path::new("daimuku-training-0.wav");
        if !path.exists() {
            return;
        }
        let mut reader = hound::WavReader::open(path).expect("open");
        let spec = reader.spec();
        let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();
        match count_daimoku(&samples, spec.sample_rate, DaimokuCountConfig::default()) {
            Some(r) => println!(
                "batch: count={} conf={:.2} phrases={} period={:.0}ms method={}",
                r.count, r.confidence, r.phrase_count, r.period_ms, r.method
            ),
            None => println!("batch: no periodicity detected"),
        }
    }
}