//! Digital signal processing primitives for Daimoku detection.

use serde::Serialize;

// -----------------------------------------------------------------------------
// Onset detection (sillable-level analysis)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct OnsetConfig {
    pub window_ms: u32,
    pub hop_ms: u32,
    /// Peak must exceed this factor times the local moving average of the OSF.
    pub threshold_alpha: f32,
    /// Minimum absolute OSF value to be considered a peak, as a fraction of
    /// the global maximum OSF. Rejects peaks in near-silent regions.
    pub threshold_delta_rel: f32,
    /// Frames used for the moving-average threshold (~500 ms of context).
    pub threshold_window_ms: u32,
    /// Frames used for smoothing the OSF before peak picking.
    pub smoothing_frames: usize,
    /// Minimum interval between accepted onsets. Sillables cannot be closer.
    pub min_ioi_ms: u32,
}

impl Default for OnsetConfig {
    fn default() -> Self {
        Self {
            window_ms: 20,
            hop_ms: 10,
            threshold_alpha: 1.8,
            threshold_delta_rel: 0.01,
            threshold_window_ms: 500,
            smoothing_frames: 3,
            min_ioi_ms: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OnsetInfo {
    pub count: usize,
    pub times_secs: Vec<f32>,
    pub ioi_ms: Vec<f32>,
    pub mean_ioi_ms: Option<f32>,
}

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

/// Onset Strength Function: positive first-order difference of the envelope.
pub fn onset_strength(env: &[f32]) -> Vec<f32> {
    if env.is_empty() {
        return Vec::new();
    }
    let mut osf = Vec::with_capacity(env.len());
    osf.push(0.0);
    for i in 1..env.len() {
        osf.push((env[i] - env[i - 1]).max(0.0));
    }
    osf
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

fn pick_peaks(osf: &[f32], threshold: &[f32], min_ioi_frames: usize) -> Vec<usize> {
    let mut peaks: Vec<usize> = Vec::new();
    if osf.len() < 3 {
        return peaks;
    }
    for i in 1..osf.len() - 1 {
        let above = osf[i] > threshold[i];
        let local_max = osf[i] > osf[i - 1] && osf[i] >= osf[i + 1];
        if !(above && local_max) {
            continue;
        }
        if let Some(&last) = peaks.last() {
            if i - last < min_ioi_frames {
                if osf[i] > osf[last] {
                    peaks.pop();
                    peaks.push(i);
                }
                continue;
            }
        }
        peaks.push(i);
    }
    peaks
}

/// Detect onsets in a mono audio signal.
pub fn detect_onsets(samples: &[f32], sample_rate: u32, config: OnsetConfig) -> OnsetInfo {
    let env = rms_envelope(samples, sample_rate, config.window_ms, config.hop_ms);
    if env.len() < 3 {
        return OnsetInfo {
            count: 0,
            times_secs: Vec::new(),
            ioi_ms: Vec::new(),
            mean_ioi_ms: None,
        };
    }

    let osf_raw = onset_strength(&env);
    let osf = if config.smoothing_frames > 1 {
        moving_average(&osf_raw, config.smoothing_frames)
    } else {
        osf_raw
    };

    let max_osf = osf.iter().cloned().fold(0.0_f32, f32::max);
    if max_osf <= f32::EPSILON {
        return OnsetInfo {
            count: 0,
            times_secs: Vec::new(),
            ioi_ms: Vec::new(),
            mean_ioi_ms: None,
        };
    }

    let delta = max_osf * config.threshold_delta_rel;
    let threshold_window_frames =
        (config.threshold_window_ms as usize / config.hop_ms.max(1) as usize).max(1);
    let local_avg = moving_average(&osf, threshold_window_frames);
    let threshold: Vec<f32> = local_avg
        .iter()
        .map(|&m| m * config.threshold_alpha + delta)
        .collect();

    let min_ioi_frames = (config.min_ioi_ms as usize / config.hop_ms.max(1) as usize).max(1);
    let peak_frames = pick_peaks(&osf, &threshold, min_ioi_frames);

    let frame_to_secs = config.hop_ms as f32 / 1000.0;
    let times_secs: Vec<f32> = peak_frames
        .iter()
        .map(|&f| f as f32 * frame_to_secs)
        .collect();
    let ioi_ms: Vec<f32> = times_secs
        .windows(2)
        .map(|w| (w[1] - w[0]) * 1000.0)
        .collect();
    let mean_ioi_ms = if ioi_ms.is_empty() {
        None
    } else {
        Some(ioi_ms.iter().sum::<f32>() / ioi_ms.len() as f32)
    };

    OnsetInfo {
        count: times_secs.len(),
        times_secs,
        ioi_ms,
        mean_ioi_ms,
    }
}

// -----------------------------------------------------------------------------
// Daimoku counting via autocorrelation
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct DaimokuCountConfig {
    pub window_ms: u32,
    pub hop_ms: u32,
    pub smoothing_frames: usize,
    /// Long moving average used to remove slow amplitude drift.
    pub detrend_window_ms: u32,
    /// Bounds for the per-Daimoku period.
    pub min_period_ms: u32,
    pub max_period_ms: u32,
    /// Minimum autocorrelation peak (normalized to lag-0) to accept a result.
    pub min_peak_confidence: f32,
    /// Envelope level, as a fraction of the 90th-percentile "typical active"
    /// level, below which a frame is considered silent. Used only to find
    /// pauses between phrases, not to detect syllable boundaries within one.
    pub pause_floor_rel: f32,
    /// A sustained silence must last at least this fraction of the detected
    /// per-Daimoku period to be treated as a breathing pause between
    /// phrases. Expressed relative to the period (not a fixed ms value) so
    /// it adapts to the user's own recitation speed: the natural dip
    /// between two consecutive Daimoku is always a *fraction* of one
    /// period, while a breath pause is comparable to, or longer than, a
    /// whole period.
    pub pause_gap_period_factor: f32,
}

impl Default for DaimokuCountConfig {
    fn default() -> Self {
        Self {
            window_ms: 30,
            hop_ms: 10,
            smoothing_frames: 5,
            detrend_window_ms: 5000,
            // A Daimoku spans roughly 0.8 to 4 seconds.
            min_period_ms: 800,
            max_period_ms: 4000,
            min_peak_confidence: 0.15,
            pause_floor_rel: 0.25,
            pause_gap_period_factor: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DaimokuCountResult {
    /// Estimated number of Daimoku in the signal.
    pub count: usize,
    /// Estimated period in milliseconds (one Daimoku).
    pub period_ms: f32,
    /// Normalized autocorrelation peak value at `period_ms`, in [0, 1].
    /// Higher means a more clearly periodic signal.
    pub confidence: f32,
    /// Total duration analyzed, in seconds (includes any pauses).
    pub duration_secs: f32,
    /// Number of distinct chanting phrases found, separated by pauses.
    /// 1 means no pause long enough to split the recording was found.
    pub segment_count: usize,
    /// Time actually spent chanting, excluding pauses between phrases.
    /// `count` is derived from this, not from `duration_secs`.
    pub active_duration_secs: f32,
}

/// Count Daimoku by finding the dominant period in the RMS envelope
/// via time-domain autocorrelation.
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
    let trend_frames =
        (config.detrend_window_ms as usize / config.hop_ms.max(1) as usize).max(1);
    let trend = moving_average(&smoothed, trend_frames);
    let mut signal: Vec<f32> = smoothed
        .iter()
        .zip(trend.iter())
        .map(|(s, t)| s - t)
        .collect();

    // Remove any residual mean and reject flat signals.
    let mean = signal.iter().sum::<f32>() / signal.len() as f32;
    for s in signal.iter_mut() {
        *s -= mean;
    }
    let energy: f32 = signal.iter().map(|x| x * x).sum();
    if energy <= f32::EPSILON {
        return None;
    }

    let hop_secs = config.hop_ms as f32 / 1000.0;
    let min_lag = ((config.min_period_ms as f32 / 1000.0) / hop_secs).round() as usize;
    let max_lag = ((config.max_period_ms as f32 / 1000.0) / hop_secs).round() as usize;
    let max_lag = max_lag.min(signal.len().saturating_sub(1));
    if min_lag >= max_lag || max_lag == 0 {
        return None;
    }

    // Normalized autocorrelation over the candidate lag range.
    let n = signal.len();
    let mut best_lag = 0usize;
    let mut best_score = 0.0_f32;
    for lag in min_lag..=max_lag {
        let mut num = 0.0_f32;
        let mut denom_a = 0.0_f32;
        let mut denom_b = 0.0_f32;
        for i in 0..(n - lag) {
            num += signal[i] * signal[i + lag];
            denom_a += signal[i] * signal[i];
            denom_b += signal[i + lag] * signal[i + lag];
        }
        let denom = (denom_a * denom_b).sqrt();
        if denom <= f32::EPSILON {
            continue;
        }
        let score = num / denom;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    if best_lag == 0 || best_score < config.min_peak_confidence {
        return None;
    }

    let period_secs = best_lag as f32 * hop_secs;
    let duration_secs = samples.len() as f32 / sample_rate as f32;

    // --- Segment the *original* smoothed envelope (not the detrended one,
    // which is centered on zero and useless for a silence floor) into
    // phrases separated by sustained pauses. This is what actually fixes
    // the overcount: a naive duration_secs / period_secs assumes the whole
    // clip is uninterrupted periodic chanting, so any breathing pause
    // between series of Daimoku gets silently "filled in" with fake
    // repetitions at the detected tempo. ---
    let mut sorted_env = smoothed.clone();
    sorted_env.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let typical_level = percentile(&sorted_env, 0.90);
    if typical_level <= f32::EPSILON {
        return None;
    }
    let floor = typical_level * config.pause_floor_rel;
    let min_gap_frames = ((period_secs * config.pause_gap_period_factor) / hop_secs)
        .round()
        .max(1.0) as usize;
    let segments = find_active_segments(&smoothed, floor, min_gap_frames);
    if segments.is_empty() {
        return None;
    }

    // Count each phrase independently and sum, instead of dividing the
    // whole clip by the period in one shot: rounding per phrase is more
    // accurate, since each phrase on its own is expected to hold a whole
    // number of Daimoku, and pauses between phrases no longer contribute
    // any fictitious fraction of a repetition.
    let mut count = 0usize;
    let mut active_frames = 0usize;
    for seg in &segments {
        let seg_frames = seg.end_frame - seg.start_frame;
        active_frames += seg_frames;
        let seg_secs = seg_frames as f32 * hop_secs;
        // A run shorter than half a period is residual noise (e.g. a mic
        // click) rather than a truncated Daimoku; drop it instead of
        // always rounding it up to 1.
        if seg_secs < period_secs * 0.5 {
            continue;
        }
        // A contiguous run of chanting starts at the attack of its first
        // Daimoku and ends at the release of its last one, so its length
        // measures roughly (N + 0.5) periods. Subtract half a period before
        // rounding to compensate for that intrinsic bias.
        let cycles = seg_secs / period_secs - 0.5;
        if cycles < 0.5 {
            continue;
        }
        count += cycles.round() as usize;
    }
    if count == 0 {
        return None;
    }

    Some(DaimokuCountResult {
        count,
        period_ms: period_secs * 1000.0,
        confidence: best_score,
        duration_secs,
        segment_count: segments.len(),
        active_duration_secs: active_frames as f32 * hop_secs,
    })
}

/// One contiguous run of "active" envelope frames (end exclusive).
#[derive(Debug, Clone, Copy)]
struct ActiveSegment {
    start_frame: usize,
    end_frame: usize,
}

/// Splits `env` into active runs, tolerating brief dips (e.g. between
/// syllables within a phrase) and only breaking on a sustained low-energy
/// stretch of at least `min_gap_frames` — a breath or pause between
/// phrases.
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

    fn bursts(count: usize, burst_secs: f32, silence_secs: f32, leading_secs: f32) -> Vec<f32> {
        let mut v = Vec::new();
        let leading = (leading_secs * RATE as f32) as usize;
        v.extend(std::iter::repeat(0.0).take(leading));
        for _ in 0..count {
            let n = (burst_secs * RATE as f32) as usize;
            for j in 0..n {
                let t = j as f32 / RATE as f32;
                v.push((2.0 * PI * 440.0 * t).sin() * 0.5);
            }
            let s = (silence_secs * RATE as f32) as usize;
            v.extend(std::iter::repeat(0.0).take(s));
        }
        v
    }

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
    fn onset_strength_is_zero_on_flat_signal() {
        let env = vec![0.5_f32; 10];
        let osf = onset_strength(&env);
        assert!(osf.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn onset_strength_only_positive_rises() {
        let env = vec![0.0, 0.5, 0.3, 0.4, 0.1];
        let osf = onset_strength(&env);
        assert_eq!(osf.len(), 5);
        assert!((osf[1] - 0.5).abs() < 1e-6);
        assert_eq!(osf[2], 0.0);
        assert!((osf[3] - 0.1).abs() < 1e-6);
        assert_eq!(osf[4], 0.0);
    }

    #[test]
    fn detects_ten_clean_onsets() {
        let samples = bursts(10, 1.0, 0.3, 0.2);
        let info = detect_onsets(&samples, RATE, OnsetConfig::default());
        assert_eq!(info.count, 10, "expected 10 onsets, got {}", info.count);
    }

    #[test]
    fn detects_five_onsets_with_wider_gaps() {
        let samples = bursts(5, 0.4, 1.0, 0.1);
        let info = detect_onsets(&samples, RATE, OnsetConfig::default());
        assert_eq!(info.count, 5);
    }

    #[test]
    fn ignores_silent_signal() {
        let samples = vec![0.0_f32; RATE as usize * 2];
        let info = detect_onsets(&samples, RATE, OnsetConfig::default());
        assert_eq!(info.count, 0);
    }

    #[test]
    fn ioi_reflects_actual_gaps() {
        let samples = bursts(4, 0.5, 0.5, 0.1);
        let info = detect_onsets(&samples, RATE, OnsetConfig::default());
        assert_eq!(info.count, 4);
        for ioi in &info.ioi_ms {
            assert!((ioi - 1000.0).abs() < 50.0, "IOI {ioi:.1} ms far from 1000 ms");
        }
    }

    // -------------------------------------------------------------------------
    // Daimoku counting via autocorrelation
    // -------------------------------------------------------------------------

    #[test]
    fn autocorr_counts_ten_periodic_daimoku() {
        // 14 s at 1.4 s period -> 10 Daimoku.
        let samples = modulated_tone(14.0, 1.4, RATE);
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (9..=11).contains(&result.count),
            "expected ~10, got {} (period {:.0} ms, confidence {:.2})",
            result.count, result.period_ms, result.confidence
        );
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn autocorr_counts_thirty_periodic_daimoku() {
        // 43 s at 1.43 s period -> 30 Daimoku.
        let samples = modulated_tone(43.0, 1.43, RATE);
        let result = count_daimoku(&samples, RATE, DaimokuCountConfig::default())
            .expect("should detect periodicity");
        assert!(
            (29..=31).contains(&result.count),
            "expected ~30, got {} (period {:.0} ms)",
            result.count, result.period_ms
        );
    }

    #[test]
    fn autocorr_rejects_silence() {
        let samples = vec![0.0_f32; RATE as usize * 5];
        assert!(count_daimoku(&samples, RATE, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn autocorr_rejects_steady_tone() {
        // Constant amplitude: no periodicity to find.
        let n = RATE as usize * 10;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 440.0 * i as f32 / RATE as f32).sin() * 0.5)
            .collect();
        assert!(count_daimoku(&samples, RATE, DaimokuCountConfig::default()).is_none());
    }

    #[test]
    fn autocorr_handles_double_periodicity() {
        // Signal at 1.4 s with a strong harmonic at 0.7 s:
        // the fundamental must win.
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
        // 20 s at 1.4 s -> ~14 Daimoku.
        assert!(
            (13..=15).contains(&result.count),
            "expected ~14, got {} (period {:.0} ms)",
            result.count, result.period_ms
        );
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
                println!(
                    "active duration: {:.2} s (excluded {:.2} s of pauses)",
                    r.active_duration_secs,
                    r.duration_secs - r.active_duration_secs
                );
            }
            None => println!("no periodicity detected"),
        }
    }

    /// Onset-level debug on the user's WAV. Kept for comparison.
    #[test]
    #[ignore]
    fn real_wav_onset_stats() {
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

        let info = detect_onsets(&samples, spec.sample_rate, OnsetConfig::default());
        println!("onsets detected: {}", info.count);
        if let Some(m) = info.mean_ioi_ms {
            println!("mean IOI: {m:.1} ms");
        }
    }
}