//! Compatibility adapter: exposes the old `count_daimoku_with_profile`
//! shape (used by the rest of the app) on top of the new phrase-based
//! `engine` module. Also re-exports `StreamingState` under its old name.

use crate::engine::{count_samples, EngineState};
use crate::profile::PersonalProfile;
use serde::Serialize;

pub type StreamingState = EngineState;

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

pub fn count_daimoku_with_profile(
    samples: &[f32],
    sample_rate: u32,
    profile: Option<&PersonalProfile>,
) -> Option<DaimokuCountResult> {
    let sr = sample_rate as usize;
    if sr == 0 || samples.is_empty() {
        return None;
    }

    let owned;
    let model = match profile {
        Some(p) => &p.model,
        None => {
            owned = crate::profile::base_only_model();
            &owned
        }
    };

    let r = count_samples(model, samples, sample_rate);
    let duration_secs = samples.len() as f32 / sample_rate as f32;

    let (phrase_count, active_duration_secs) = active_segment_stats(samples, sample_rate);

    let confidence = if r.count > 0 {
        (r.mean_llr / model.ref_llr.max(0.1)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    Some(DaimokuCountResult {
        count: r.count,
        confidence,
        phrase_count,
        mean_period_ms: r.period_ms.unwrap_or(0.0),
        method: "nam-myoho-renge-kyo".to_string(),
        period_ms: r.period_ms.unwrap_or(0.0),
        segment_count: phrase_count,
        duration_secs,
        active_duration_secs,
    })
}

// ---------------------------------------------------------------------------
// Auxiliary, display-only stats: how many separate bursts of chanting
// (vs. pauses) the recording contains, purely informational.
// ---------------------------------------------------------------------------

fn active_segment_stats(samples: &[f32], sample_rate: u32) -> (usize, f32) {
    let env = rms_envelope(samples, sample_rate, 30, 10);
    if env.is_empty() {
        return (0, 0.0);
    }
    let smoothed = moving_average(&env, 20);
    let peak = smoothed.iter().cloned().fold(f32::MIN, f32::max);
    if peak <= 1e-6 {
        return (0, 0.0);
    }
    let floor = 0.10 * peak;
    let min_gap = 30usize; // 300ms
    let segs = find_active_segments(&smoothed, floor, min_gap);
    let active_secs: f32 = segs.iter().map(|&(s, e)| (e - s) as f32 * 0.01).sum();
    (segs.len(), active_secs)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileState;

    #[test]
    fn adapter_counts_a_base_clip() {
        let ps = ProfileState::new();
        let snap = ps.snapshot();
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[3].wav).unwrap();
        let r = count_daimoku_with_profile(&samples, sr, Some(&snap)).expect("result");
        assert_eq!(r.count, 7);
        assert!(r.confidence > 0.5);
        assert!(r.period_ms > 500.0 && r.period_ms < 2000.0);
    }

    #[test]
    fn adapter_falls_back_without_profile() {
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        let r = count_daimoku_with_profile(&samples, sr, None).expect("result");
        assert_eq!(r.count, 1);
    }
}
