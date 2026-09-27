//! Extraction of a per-user Daimoku rhythm template from onset intervals.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaimokuTemplate {
    pub period: usize,
    /// Normalized IOI ratios within one Daimoku cycle.
    /// Every value is relative to the mean IOI of the cycle, so the
    /// template is independent of the absolute recitation speed.
    pub normalized_ioi: Vec<f32>,
    /// Mean IOI observed during training, in milliseconds. Informational.
    pub mean_ioi_ms: f32,
    /// Mean relative error between individual cycles and the template.
    /// Small values mean consistent rhythm; large values mean the user
    /// did not articulate clearly or did not keep a steady pace.
    pub training_error: f32,
    pub training_daimoku_count: u32,
}

const MIN_PERIOD: usize = 4;
const MAX_PERIOD: usize = 12;
const MAX_TRAINING_ERROR: f32 = 0.20;
/// The last recitation is often truncated (Stop pressed before the final
/// syllable boundary), so we accept one fewer cycle than expected.
const CYCLE_TOLERANCE: usize = 1;

pub fn learn_template(
    ioi_ms: &[f32],
    expected_daimoku: u32,
) -> Result<DaimokuTemplate, String> {
    if expected_daimoku == 0 {
        return Err("expected_daimoku must be > 0".into());
    }
    if ioi_ms.len() < MIN_PERIOD {
        return Err(format!(
            "not enough onsets: {} IOIs detected, need at least {}",
            ioi_ms.len(),
            MIN_PERIOD
        ));
    }

    let n = expected_daimoku as usize;
    let onset_count = ioi_ms.len() + 1;
    let period = (onset_count as f32 / n as f32).round() as usize;

    if !(MIN_PERIOD..=MAX_PERIOD).contains(&period) {
        return Err(format!(
            "detected period {period} out of range [{MIN_PERIOD}, {MAX_PERIOD}]. \
             Onsets detected: {onset_count}, Daimoku recited: {n}. \
             Try reciting with clearer syllable separation."
        ));
    }

    let complete_cycles = ioi_ms.len() / period;
    let required = n.saturating_sub(CYCLE_TOLERANCE).max(MIN_PERIOD);
    if complete_cycles < required {
        return Err(format!(
            "only {complete_cycles} complete cycles of period {period} found, \
             but at least {required} expected for {n} Daimoku. \
             Onsets detected: {onset_count}. Recite with clearer syllable \
             separation and a steadier rhythm."
        ));
    }

    // Use as many cycles as we have available, capped at the expected count.
    let usable_cycles = complete_cycles.min(n);
    let start_cycle = (complete_cycles - usable_cycles) / 2;

    let mut positions: Vec<Vec<f32>> = vec![Vec::with_capacity(usable_cycles); period];
    for cycle in start_cycle..start_cycle + usable_cycles {
        for (pos, slot) in positions.iter_mut().enumerate() {
            slot.push(ioi_ms[cycle * period + pos]);
        }
    }

    let means: Vec<f32> = positions
        .iter()
        .map(|v| v.iter().sum::<f32>() / v.len() as f32)
        .collect();

    if means.iter().any(|&m| m <= 0.0) {
        return Err("some cycle positions have zero or negative mean IOI".into());
    }

    let global_mean = means.iter().sum::<f32>() / period as f32;
    let normalized_ioi: Vec<f32> = means.iter().map(|&m| m / global_mean).collect();

    let mut total_err = 0.0_f32;
    for cycle in start_cycle..start_cycle + usable_cycles {
        let mut cycle_err = 0.0_f32;
        for pos in 0..period {
            let observed = ioi_ms[cycle * period + pos];
            let expected = global_mean * normalized_ioi[pos];
            cycle_err += ((observed - expected) / expected).abs();
        }
        total_err += cycle_err / period as f32;
    }
    let training_error = total_err / usable_cycles as f32;

    if training_error > MAX_TRAINING_ERROR {
        return Err(format!(
            "training error {:.1}% exceeds {:.0}% threshold. \
             Recite more consistently (clear syllables, steady rhythm) and try again.",
            training_error * 100.0,
            MAX_TRAINING_ERROR * 100.0
        ));
    }

    Ok(DaimokuTemplate {
        period,
        normalized_ioi,
        mean_ioi_ms: global_mean,
        training_error,
        training_daimoku_count: usable_cycles as u32,
    })
}

/// Similarity between a candidate IOI cycle and the template.
/// Returns a score in `[0, 1]`; `1.0` is a perfect match.
/// The candidate must contain at least `template.period` values.
pub fn match_score(template: &DaimokuTemplate, cycle_ioi: &[f32]) -> f32 {
    let p = template.period;
    if cycle_ioi.len() < p {
        return 0.0;
    }
    let window = &cycle_ioi[..p];
    let window_mean = window.iter().sum::<f32>() / p as f32;
    if window_mean <= 0.0 {
        return 0.0;
    }
    let mut err = 0.0_f32;
    for i in 0..p {
        let observed = window[i] / window_mean;
        let expected = template.normalized_ioi[i];
        err += ((observed - expected) / expected).abs();
    }
    (1.0 - err / p as f32).max(0.0)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a stream of IOIs from `n` repetitions of `pattern_ms`.
    fn repeat_pattern(pattern_ms: &[f32], n: usize) -> Vec<f32> {
        let mut v = Vec::with_capacity(pattern_ms.len() * n);
        for _ in 0..n {
            v.extend_from_slice(pattern_ms);
        }
        v
    }

    #[test]
    fn learns_a_clean_periodic_pattern() {
        // Period 6, last IOI longer (Kyo -> next Nam).
        let pattern = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        let iois = repeat_pattern(&pattern, 10);

        let template = learn_template(&iois, 10).expect("learn failed");
        assert_eq!(template.period, 6);
        assert_eq!(template.normalized_ioi.len(), 6);
        assert!(template.training_error < 0.01);
        // Last position should be about twice the mean.
        assert!(template.normalized_ioi[5] > 1.5);
        assert!(template.normalized_ioi[0] < 1.0);
    }

    #[test]
    fn learns_a_fast_pattern() {
        // Same rhythm, half duration: the normalized template must be identical.
        let slow = [200.0, 200.0, 200.0, 200.0, 200.0, 400.0];
        let fast = [50.0, 50.0, 50.0, 50.0, 50.0, 100.0];

        let t_slow = learn_template(&repeat_pattern(&slow, 5), 5).unwrap();
        let t_fast = learn_template(&repeat_pattern(&fast, 5), 5).unwrap();

        for i in 0..6 {
            let diff = (t_slow.normalized_ioi[i] - t_fast.normalized_ioi[i]).abs();
            assert!(
                diff < 0.01,
                "position {i}: slow {} vs fast {}",
                t_slow.normalized_ioi[i],
                t_fast.normalized_ioi[i]
            );
        }
    }

    #[test]
    fn accepts_one_missing_cycle() {
        // 10 Daimoku expected but only 9 complete cycles available:
        // the last one was truncated by Stop.
        let pattern = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        let iois = repeat_pattern(&pattern, 9);
        let template = learn_template(&iois, 10).expect("should accept 9/10");
        assert_eq!(template.training_daimoku_count, 9);
    }

    #[test]
    fn rejects_extremely_short_signal() {
        // Only one cycle of six IOIs, claimed to be 10 Daimoku:
        // period estimate collapses to 1, which is out of range.
        let pattern = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        assert!(learn_template(&pattern, 10).is_err());
    }

    #[test]
    fn rejects_inconsistent_rhythm() {
        // Random IOIs, no periodicity.
        let iois: Vec<f32> = (0..60).map(|i| 100.0 + (i * 37 % 200) as f32).collect();
        let result = learn_template(&iois, 10);
        assert!(result.is_err(), "inconsistent rhythm should be rejected");
    }

    #[test]
    fn rejects_too_few_onsets() {
        let iois = vec![100.0, 100.0];
        assert!(learn_template(&iois, 10).is_err());
    }

    #[test]
    fn rejects_absurd_period() {
        // 1 onset per Daimoku expected -> period ~1, out of range.
        let iois = vec![100.0; 10];
        assert!(learn_template(&iois, 1000).is_err());
    }

    #[test]
    fn match_score_perfect_on_training_pattern() {
        let pattern = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        let template = learn_template(&repeat_pattern(&pattern, 5), 5).unwrap();
        let score = match_score(&template, &pattern);
        assert!(score > 0.99, "expected ~1.0, got {score}");
    }

    #[test]
    fn match_score_high_on_scaled_pattern() {
        // Same rhythm, different speed: still a good match.
        let training = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        let faster = [50.0, 50.0, 50.0, 50.0, 50.0, 100.0];
        let template = learn_template(&repeat_pattern(&training, 5), 5).unwrap();
        let score = match_score(&template, &faster);
        assert!(score > 0.99, "expected > 0.99, got {score}");
    }

    #[test]
    fn match_score_low_on_different_pattern() {
        let training = [100.0, 100.0, 100.0, 100.0, 100.0, 200.0];
        let different = [100.0, 200.0, 100.0, 100.0, 200.0, 100.0];
        let template = learn_template(&repeat_pattern(&training, 5), 5).unwrap();
        let score = match_score(&template, &different);
        assert!(score < 0.6, "expected < 0.6, got {score}");
    }
}