//! Digital signal processing primitives for Daimoku counting.
//!
//! Strategy: split the recording into phrases separated by breathing pauses,
//! then run autocorrelation *independently on each phrase* to find the local
//! period. Per-phrase periods are accurate even when the user changes speed
//! between phrases, which a global autocorrelation cannot handle.

use serde::Serialize;

/// Short-time RMS envelope. One value per analysis frame.
pub fn rms_envelope(samples: &[f32], sample_rate: u32, window_ms: u32, hop_ms: u32) -> Vec<f32> {
    let window = (window_ms as usize * sample_rate as usize) / 1000;
    let hop = (hop_ms as usize * sample_rate as usize) / 1000;
    if window == 0 || hop == 0 || samples.len() < window {
        return Vec::new();
    }

    let mut env = Vec::with_capacity((samples.len() - window) / hop + 1);
    let mut start = 0;
    while start + window <= samples.len() {
        let chunk = &samples[start..start + window];
        let sum_sq: f32 = chunk.iter().map(|x| x * x).sum();
        env.push((sum_sq / window as f32).sqrt());
        start += hop;
    }
    env
}

/// Simple centered moving average.
fn moving_average(data: &[f32], window: usize) -> Vec<f32> {
    let n = data.len();
    if window == 0 || n == 0 {
        return vec![0.0; n];
    }
    let half = window / 2;
    let mut out = vec![0.0_f32; n];
    for (i, slot) in out.iter_mut().enumerate() {
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let count = hi - lo;
        *slot = data[lo..hi].iter().sum::<f32>() / count as f32;
    }
    out
}

// -----------------------------------------------------------------------------
// Daimoku counting
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct DaimokuCountConfig {
    pub window_ms: u32,
    pub hop_ms: u32,
    pub smoothing_frames: usize,
    /// Long moving average used to remove slow amplitude drift within a phrase.
    pub detrend_window_ms: u32,
    /// Bounds for the per-Daimoku period.
    pub min_period_ms: u32,
    pub max_period_ms: u32,
    /// Minimum normalized autocorrelation peak to accept a phrase.
    pub min_peak_confidence: f32,
    /// Envelope level, as a fraction of the 90th-percentile "typical active"
    /// level, below which a frame is considered silent.
    pub pause_floor_rel: f32,
    /// Sustained silence (ms) long enough to mark a phrase boundary. Since
    /// Daimoku are continuous within a phrase, any silence of at least this
    /// length is a breath between phrases, not a syllable boundary.
    pub phrase_gap_ms: u32,
}

impl Default for DaimokuCountConfig {
    fn default() -> Self {
        Self {
            window_ms: 30,
            hop_ms: 10,
            smoothing_frames: 5,
            detrend_window_ms: 5000,
            min_period_ms: 800,
            max_period_ms: 4000,
            min_peak_confidence: 0.15,
            pause_floor_rel: 0.25,
            phrase_gap_ms: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DaimokuCountResult {
    pub count: usize,
    /// Duration-weighted mean period across all analyzed phrases.
    pub period_ms: f32,
    /// Duration-weighted mean autocorrelation confidence across phrases.
    pub confidence: f32,
    pub duration_secs: f32,
    pub segment_count: usize,
    pub active_duration_secs: f32,
}

/// Count Daimoku: split the recording into phrases by silence, then run
/// autocorrelation independently on each phrase and sum the per-phrase counts.
pub fn count_daimoku(
    samples: &[f32],
    sample_rate: u32,
    config: DaimokuCountConfig,
) -> Option<DaimokuCountResult> {
    let env = rms_envelope(samples, sample_rate, config.window_ms, config.hop_ms);
    if env.len() < 3 {
        return None;
    }

    let smoothed = moving_average(&env, config.smoothing_frames);

    // Silence floor from the 90th percentile of the smoothed envelope, so it
    // adapts to microphone gain and to the user's volume.
    let mut sorted_env = smoothed.clone();
    sorted_env.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let typical_level = percentile(&sorted_env, 0.90);
    if typical_level <= f32::EPSILON {
        return None;
    }
    let floor = typical_level * config.pause_floor_rel;

    let hop_secs = config.hop_ms as f32 / 1000.0;
    let hop_ms = config.hop_ms.max(1);
    let min_gap_frames = ((config.phrase_gap_ms + hop_ms - 1) / hop_ms) as usize;
    let segments = find_active_segments(&smoothed, floor, min_gap_frames);

    let min_period_secs = config.min_period_ms as f32 / 1000.0;
    let max_period_secs = config.max_period_ms as f32 / 1000.0;
    let min_lag = (min_period_secs / hop_secs).round() as usize;
    let max_lag_all = (max_period_secs / hop_secs).round() as usize;

    let mut total_count = 0usize;
    let mut total_active_frames = 0usize;
    let mut weighted_period = 0.0_f32;
    let mut weighted_confidence = 0.0_f32;
    let mut weight = 0.0_f32;
    let mut analyzed_segments = 0usize;

    for seg in &segments {
        let frames = seg.end_frame - seg.start_frame;
        let seg_secs = frames as f32 * hop_secs;
        // Two periods are the minimum needed for a meaningful autocorrelation.
        if seg_secs < min_period_secs * 2.0 {
            continue;
        }

        let seg_smoothed = &smoothed[seg.start_frame..seg.end_frame];

        // Detrend within the segment to remove the phrase's amplitude drift.
        let detrend_w = ((config.detrend_window_ms / hop_ms) as usize).max(1);
        let trend = moving_average(seg_smoothed, detrend_w.min(frames));
        let mut sig: Vec<f32> = seg_smoothed
            .iter()
            .zip(trend.iter())
            .map(|(s, t)| s - t)
            .collect();
        let mean = sig.iter().sum::<f32>() / sig.len() as f32;
        for s in sig.iter_mut() {
            *s -= mean;
        }
        let energy: f32 = sig.iter().map(|x| x * x).sum();
        if energy <= f32::EPSILON {
            continue;
        }

        let n = sig.len();
        let max_lag = max_lag_all.min(n.saturating_sub(1));
        if min_lag >= max_lag {
            continue;
        }

        // Normalized autocorrelation over the candidate lag range.
        let mut corr = vec![0.0_f32; max_lag + 1];
        for lag in min_lag..=max_lag {
            let mut num = 0.0_f32;
            let mut da = 0.0_f32;
            let mut db = 0.0_f32;
            for i in 0..(n - lag) {
                num += sig[i] * sig[i + lag];
                da += sig[i] * sig[i];
                db += sig[i + lag] * sig[i + lag];
            }
            let denom = (da * db).sqrt();
            if denom > f32::EPSILON {
                corr[lag] = num / denom;
            }
        }

        // Harmonic-aware selection: score each lag together with its harmonics
        // with decreasing weights, so a spurious peak at a multiple of the
        // fundamental cannot beat the fundamental itself.
        let mut best_lag = 0usize;
        let mut best_combined = 0.0_f32;
        for lag in min_lag..=max_lag {
            let mut combined = corr[lag];
            let mut w = 0.5_f32;
            let mut k = 2;
            while k * lag <= max_lag {
                combined += w * corr[k * lag];
                w *= 0.5;
                k += 1;
            }
            if combined > best_combined {
                best_combined = combined;
                best_lag = lag;
            }
        }

        let best_score = if best_lag > 0 { corr[best_lag] } else { 0.0 };
        if best_lag == 0 || best_score < config.min_peak_confidence {
            continue;
        }

        // Parabolic interpolation around the peak for sub-lag resolution:
        // the true period usually sits between two discrete lags.
        let precise_lag = if best_lag > min_lag && best_lag < max_lag {
            let y0 = corr[best_lag - 1];
            let y1 = corr[best_lag];
            let y2 = corr[best_lag + 1];
            let denom = 2.0 * y1 - y0 - y2;
            if denom.abs() > 1e-9 {
                let delta = ((y2 - y0) / (2.0 * denom)).clamp(-0.5, 0.5);
                (best_lag as f32 + delta).max(1.0)
            } else {
                best_lag as f32
            }
        } else {
            best_lag as f32
        };

        let period_secs = precise_lag * hop_secs;
        if period_secs < min_period_secs * 0.5 {
            continue;
        }

        let seg_count = (seg_secs / period_secs).round() as usize;
        if seg_count == 0 {
            continue;
        }

        total_count += seg_count;
        total_active_frames += frames;
        weighted_period += period_secs * seg_secs;
        weighted_confidence += best_score * seg_secs;
        weight += seg_secs;
        analyzed_segments += 1;
    }

    if total_count == 0 || weight <= 0.0 {
        return None;
    }

    Some(DaimokuCountResult {
        count: total_count,
        period_ms: (weighted_period / weight) * 1000.0,
        confidence: weighted_confidence / weight,
        duration_secs: samples.len() as f32 / sample_rate as f32,
        segment_count: analyzed_segments,
        active_duration_secs: total_active_frames as f32 * hop_secs,
    })
}

/// One contiguous run of "active" envelope frames (end exclusive).
#[derive(Debug, Clone, Copy)]
struct ActiveSegment {
    start_frame: usize,
    end_frame: usize,
}

/// Splits `env` into active runs, tolerating brief dips and only breaking
/// on a sustained low-energy stretch of at least `min_gap_frames`.
fn find_active_segments(env: &[f32], floor: f32, min_gap_frames: usize) -> Vec<ActiveSegment> {
    let mut segments = Vec::new();
    let mut seg_start: Option<usize> = None;
    let mut last_active = 0usize;
    let mut silent_run = 0usize;

    for (i, &v) in env.iter().enumerate() {
        if v > floor {
            if seg_start.is_none() {
                seg_start = Some(i);
            }
            last_active = i;
            silent_run = 0;
        } else {
            silent_run += 1;
            if silent_run >= min_gap_frames {
                if let Some(s) = seg_start.take() {
                    segments.push(ActiveSegment {
                        start_frame: s,
                        end_frame: last_active + 1,
                    });
                }
            }
        }
    }
    if let Some(s) = seg_start {
        segments.push(ActiveSegment {
            start_frame: s,
            end_frame: last_active + 1,
        });
    }
    segments
}

/// Value at percentile `p` (0.0..=1.0) of an already-sorted slice.
fn percentile(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (((sorted.len() - 1) as f32) * p).round() as usize;
    sorted[idx]
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const RATE: u32 = 48_000;

    /// Amplitude oscillates with `period_s`; the envelope has one peak per period.
    fn modulated_tone(duration_s: f32, period_s: f32, rate: u32) -> Vec<f32> {
        let n = (duration_s * rate as f32) as usize;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / rate as f32;
            let carrier = (2.0 * PI * 440.0 * t).sin();
            let envelope = 0.5 + 0.5 * (2.0 * PI * t / period_s).sin();
            v.push(carrier * envelope * 0.5);
        }
        v
    }

    #[test]
    fn envelope_empty_on_short_signal() {
        let env = rms_envelope(&[0.0; 100], RATE, 20, 10);
        assert!(env.is_empty());
    }

    #[test]
    fn envelope_tracks_energy() {
        let n = RATE as usize;
        let mut samples = vec![0.0_f32; 2 * n];
        for (i, s) in samples.iter_mut().take(n).enumerate() {
            *s = (2.0 * PI * 440.0 * i as f32 / RATE as f32).sin();
        }
        let env = rms_envelope(&samples, RATE, 20, 10);
        let mid = env.len() / 2;
        assert!(env[10] > 0.5);
        assert!(env[mid + 10] < 0.01);
    }

    #[test]
    fn counts_ten_periodic_daimoku() {
        let samples = modulated_tone(14.0, 1.4, RATE);
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (9..=11).contains(&result.count),
            "expected ~10, got {} (period {:.0} ms, confidence {:.2})",
            result.count,
            result.period_ms,
            result.confidence
        );
    }

    #[test]
    fn counts_thirty_periodic_daimoku() {
        let samples = modulated_tone(43.0, 1.43, RATE);
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (29..=31).contains(&result.count),
            "expected ~30, got {} (period {:.0} ms)",
            result.count,
            result.period_ms
        );
    }

    #[test]
    fn rejects_silence() {
        let samples = vec![0.0_f32; RATE as usize * 5];
        assert!(count_daimoku(&samples, RATE, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn rejects_steady_tone() {
        let n = RATE as usize * 10;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 440.0 * i as f32 / RATE as f32).sin() * 0.5)
            .collect();
        assert!(count_daimoku(&samples, RATE, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn handles_double_periodicity() {
        let n = (20.0 * RATE as f32) as usize;
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / RATE as f32;
            let carrier = (2.0 * PI * 440.0 * t).sin();
            let env_fund = 0.6 + 0.4 * (2.0 * PI * t / 1.4).sin();
            let env_harm = 0.15 * (2.0 * PI * t / 0.7).sin();
            samples.push(carrier * (env_fund + env_harm) * 0.5);
        }
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (13..=15).contains(&result.count),
            "expected ~14, got {} (period {:.0} ms)",
            result.count,
            result.period_ms
        );
    }

    #[test]
    fn counts_two_phrases_at_different_speeds() {
        // Mimics the user's failure mode: 5 Daimoku at 1.0 s, then a 400 ms
        // breath, then 5 Daimoku at 1.35 s. Total should be 10.
        let mut samples = modulated_tone(5.0, 1.0, RATE);
        samples.extend(std::iter::repeat(0.0).take((0.4 * RATE as f32) as usize));
        samples.extend(modulated_tone(6.75, 1.35, RATE));
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (9..=11).contains(&result.count),
            "expected ~10, got {} (period {:.0} ms, segments {})",
            result.count,
            result.period_ms,
            result.segment_count
        );
    }

    #[test]
    fn counts_two_long_phrases_separately() {
        let mut samples = modulated_tone(20.0, 1.4, RATE);
        samples.extend(std::iter::repeat(0.0).take((1.0 * RATE as f32) as usize));
        samples.extend(modulated_tone(20.0, 1.4, RATE));
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        // 20 / 1.4 = 14.28 per phrase -> 14 each -> 28 total.
        assert!(
            (27..=29).contains(&result.count),
            "expected ~28, got {} (segments {})",
            result.count,
            result.segment_count
        );
        assert_eq!(result.segment_count, 2);
    }

    /// Reads the user's reference WAV from the Desktop and prints the count.
    /// Run with:
    ///     cargo test -- --ignored --nocapture real_wav_daimoku_count
    #[test]
    #[ignore]
    fn real_wav_daimoku_count() {
        let dir = dirs::desktop_dir().expect("no desktop dir");
        let path = dir.join("daimuku-training-0.wav");
        if !path.exists() {
            eprintln!("skipping: {} not found", path.display());
            return;
        }

        let mut reader = hound::WavReader::open(&path).expect("open failed");
        let spec = reader.spec();
        let samples: Vec<f32> = reader
            .samples::<f32>()
            .map(|s| s.expect("read failed"))
            .collect();

        println!(
            "loaded {} samples at {} Hz = {:.2}s",
            samples.len(),
            spec.sample_rate,
            samples.len() as f32 / spec.sample_rate as f32
        );

        match count_daimoku(&samples, spec.sample_rate, DaimokuCountConfig::default()) {
            Some(r) => {
                println!("count: {}", r.count);
                println!("period: {:.0} ms", r.period_ms);
                println!("confidence: {:.3}", r.confidence);
                println!("duration: {:.2} s", r.duration_secs);
                println!("segments: {}", r.segment_count);
                println!("active duration: {:.2} s", r.active_duration_secs);
            }
            None => println!("no periodicity detected"),
        }
    }
}