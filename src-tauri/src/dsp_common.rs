//! Shared low-level DSP helpers used by both the training-time profile
//! extraction (`profile.rs`) and the counting engine (`dsp.rs`).
//!
//! Keeping a single copy avoids the two paths silently drifting apart
//! (e.g. one being tuned and the other forgotten).

use std::cmp::Ordering;

/// Short-time RMS envelope of `samples`, using a sliding window of
/// `window_ms` with a hop of `hop_ms`. Returns one value per hop.
pub fn rms_envelope(samples: &[f32], sr: u32, window_ms: u32, hop_ms: u32) -> Vec<f32> {
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

/// Centered moving average with window `window` (frames), edge-safe.
pub fn moving_average(data: &[f32], window: usize) -> Vec<f32> {
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

/// First-order "how much higher than `lag` frames ago" novelty curve.
/// Negative differences are clipped to zero since we only care about
/// onsets (energy rising), not decays.
pub fn compute_novelty(sig: &[f32], lag: usize) -> Vec<f32> {
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

pub fn median_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut v = data.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let m = v.len() / 2;
    if v.len() % 2 == 0 {
        (v[m - 1] + v[m]) * 0.5
    } else {
        v[m]
    }
}

pub fn percentile(data: &[f32], p: f32) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut s = data.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let idx = (((s.len() - 1) as f32) * p.clamp(0.0, 1.0)).round() as usize;
    s[idx]
}

/// Finds contiguous runs where `env` stays above `floor`, tolerating
/// gaps shorter than `min_gap` frames (so a single low dip between two
/// syllables of the *same* Daimoku doesn't split it into two
/// segments). Used both to find silence-safe cut points for the live
/// counter and to keep breaths/pauses from biasing period estimation
/// during training.
pub fn find_active_segments(env: &[f32], floor: f32, min_gap: usize) -> Vec<(usize, usize)> {
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

/// Zero-crossing rate of a raw audio segment, in crossings per sample
/// (already normalized to segment length, so segments of different
/// length are comparable). Used as a cheap, dependency-free proxy for
/// spectral content: sustained vocalization sits in a fairly
/// consistent ZCR band for a given speaker/phrase, while unrelated
/// noises (claps, taps, door slams, background speech at a very
/// different pitch/timbre) tend to fall well outside it. This is
/// *not* a substitute for real spectral features (MFCCs etc.), but it
/// needs no extra dependency and is enough to catch grossly different
/// sounds — see the content gate in `dsp.rs`.
pub fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut crossings = 0usize;
    for w in samples.windows(2) {
        if (w[0] >= 0.0) != (w[1] >= 0.0) {
            crossings += 1;
        }
    }
    crossings as f32 / (samples.len() - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even() {
        assert_eq!(median_f32(&[1.0, 3.0, 2.0]), 2.0);
        assert_eq!(median_f32(&[1.0, 2.0, 3.0, 4.0]), 2.5);
        assert_eq!(median_f32(&[]), 0.0);
    }

    #[test]
    fn active_segments_bridges_short_gaps() {
        // active, short dip (bridged), active, long gap, active
        let env = [1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0];
        let segs = find_active_segments(&env, 0.5, 3);
        assert_eq!(segs, vec![(0, 5), (9, 11)]);
    }

    #[test]
    fn zcr_of_square_wave_is_high() {
        let alt: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        assert!(zero_crossing_rate(&alt) > 0.9);
        let dc = vec![1.0f32; 100];
        assert_eq!(zero_crossing_rate(&dc), 0.0);
    }
}