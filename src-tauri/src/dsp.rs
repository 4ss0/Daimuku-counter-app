//! Daimoku counter: novelty-based, monotonic streaming, batch post-processed.

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

    // Sanity check.
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

    // Phrase count (informational).
    let hop_secs = cfg.hop_ms as f32 / 1000.0;
    let sm_frames = ((cfg.smooth_secs / hop_secs).round() as usize).max(1);
    let smoothed = moving_average(&env, sm_frames);
    let silence_floor = cfg.silence_rel * p95;
    let min_gap_frames = ((cfg.min_pause_secs / hop_secs).round() as usize).max(2);
    let phrases = find_active_segments(&smoothed, silence_floor, min_gap_frames);
    let phrase_count = phrases.len();

    // Run the streaming counter with 1 s of silence warm-up.
    let mut sc = StreamingCounter::new(sample_rate);
    let warmup = vec![0.0f32; sample_rate as usize];
    for chunk in warmup.chunks(4096) {
        sc.push(chunk);
    }
    for chunk in samples.chunks(4096) {
        sc.push(chunk);
    }

    // Post-process: robust period + NMS sorted by novelty strength.
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

/// Post-process the accepted peaks: robust period from top-60% of
/// intervals, then NMS sorted by novelty strength.
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

    // Sort indices by value descending.
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

/// Robust period: sort intervals, take the median of the top 60%.
/// Sub-peak intervals are always the shortest, so they fall in the
/// bottom 40% and are discarded.
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
// Helper functions
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

fn percentile(data: &[f32], p: f32) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut s = data.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let idx = (((s.len() - 1) as f32) * p).round() as usize;
    s[idx]
}

// ===========================================================================
// Streaming counter
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

    // Accepted peaks for post-processing.
    accepted: Vec<(u64, f32)>,

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

            state: StreamingState::Warming,
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
    pub fn accepted_peaks(&self) -> &[(u64, f32)] {
        &self.accepted
    }

    pub fn period_secs(&self) -> Option<f32> {
        self.period_median_frames().map(|f| (f * 0.01) as f32)
    }

    /// Robust period: median of the top 60% of recent intervals.
    /// Sub-peak intervals are the shortest and land in the bottom 40%.
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

        // Strict local max over ±local_win, plateau-tolerant.
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

        // Min distance gate. 0.55 * T once we have an estimate, else 0.5 s.
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

        // Accept.
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

    /// Sub-peak 0.65 s after each attack.
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

    fn run_streaming(v: &[f32]) -> StreamingCounter {
        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let warm = vec![0.0f32; SR as usize];
        let mut i = 0;
        while i < warm.len() {
            let end = (i + chunk).min(warm.len());
            c.push(&warm[i..end]);
            i = end;
        }
        i = 0;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            c.push(&v[i..end]);
            i = end;
        }
        c
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
    fn ten_daimoku_with_sub_peaks() {
        let v = phrase_with_sub_peaks(10, 1.30, SR);
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default())
            .expect("must count 10 not 20");
        assert!(
            (9..=11).contains(&r.count),
            "sub-peaks: expected ~10, got {}",
            r.count
        );
    }

    #[test]
    fn ten_daimoku_strong_first_attack() {
        let n_bumps = 10;
        let period_s = 1.05_f32;
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
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default())
            .expect("must count");
        assert!(
            (9..=11).contains(&r.count),
            "expected ~10, got {}",
            r.count
        );
    }

    #[test]
    fn eight_plus_two_phrases() {
        let mut v = phrase_with_bumps(8, 1.05, true, &[], SR);
        v.extend(silence(1.0));
        v.extend(phrase_with_bumps(2, 1.50, false, &[], SR));
        let r = count_daimoku(&v, SR, DaimokuCountConfig::default())
            .expect("must count 8+2");
        assert!(
            (9..=11).contains(&r.count),
            "8+2: expected ~10, got {}",
            r.count
        );
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

    // ---- Streaming ------------------------------------------------------

    #[test]
    fn streaming_counts_ten_continuous() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let c = run_streaming(&v);
        assert!(
            (9..=11).contains(&c.count()),
            "streaming got {}",
            c.count()
        );
    }

    #[test]
    fn streaming_is_monotonic() {
        let v = phrase_with_bumps(10, 1.05, true, &[], SR);
        let mut c = StreamingCounter::new(SR);
        let chunk = (SR as usize) / 20;
        let mut i = 0;
        let mut prev = 0usize;
        while i < v.len() {
            let end = (i + chunk).min(v.len());
            let now = c.push(&v[i..end]);
            assert!(now >= prev, "count decreased: {} -> {}", prev, now);
            prev = now;
            i = end;
        }
    }

    #[test]
    fn streaming_eight_plus_two_after_breath() {
        let mut v = phrase_with_bumps(8, 1.05, true, &[], SR);
        v.extend(silence(1.0));
        v.extend(phrase_with_bumps(2, 1.50, false, &[], SR));
        let c = run_streaming(&v);
        assert!(
            (8..=11).contains(&c.count()),
            "streaming 8+2 got {}",
            c.count()
        );
    }

    #[test]
    fn streaming_handles_amplitude_drop() {
        let loud = phrase_with_bumps(5, 1.05, true, &[], SR);
        let quiet: Vec<f32> = phrase_with_bumps(5, 1.05, false, &[], SR)
            .iter()
            .map(|x| x * 0.35)
            .collect();
        let mut v = loud;
        v.extend(quiet);
        let c = run_streaming(&v);
        assert!(
            (8..=11).contains(&c.count()),
            "amplitude drop: streaming got {}",
            c.count()
        );
    }

    #[test]
    fn streaming_handles_speed_change() {
        let mut v = phrase_with_bumps(5, 1.50, true, &[], SR);
        v.extend(phrase_with_bumps(5, 0.90, false, &[], SR));
        let c = run_streaming(&v);
        assert!(
            (9..=11).contains(&c.count()),
            "speed change: streaming got {}",
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