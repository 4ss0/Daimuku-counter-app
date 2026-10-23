//! The bell rung from the Count screen, taken out of the microphone signal
//! before the counter hears it.
//!
//! The bells are synthetic (tools/bells/make.py): a few partials at known
//! frequencies, gone within 6 s. After a strike every partial of that bell
//! is cut by a narrow notch filter until the sound is over; the voice loses
//! only those narrow bands. The filter fades in and out over a few
//! milliseconds, so it never clicks.

/// The bells and their partials (frequency, beat; Hz), as in
/// tools/bells/make.py: each partial is a pair of sines `beat` apart, and
/// each of the two gets its own notch.
const BELLS: [(&str, &[(f32, f32)]); 3] = [
    ("high", &[(1876.0, 1.6), (3484.0, 6.0), (5460.0, 4.0), (7782.0, 3.0)]),
    ("medium", &[(523.0, 0.9), (1417.0, 2.2), (2678.0, 3.1), (4236.0, 2.0)]),
    ("deep", &[(146.0, 0.6), (296.0, 0.9), (351.0, 1.3), (438.0, 1.1), (604.0, 1.7), (789.0, 2.3)]),
];

/// How long a strike is filtered: the sound (6 s) and the time it takes
/// from the speaker back to the microphone.
const RING_SECS: f32 = 6.5;
/// Notch width: bandwidth = frequency / Q (1876 Hz: ~94 Hz wide).
const Q: f32 = 20.0;
const FADE_SECS: f32 = 0.02;


/// RBJ notch biquad (transposed direct form II).
#[derive(Clone)]
struct Notch {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Notch {
    fn new(hz: f32, sr: u32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * hz / sr as f32;
        let alpha = w0.sin() / (2.0 * Q);
        let a0 = 1.0 + alpha;
        let c = -2.0 * w0.cos();
        Self { b0: 1.0 / a0, b1: c / a0, b2: 1.0 / a0, a1: c / a0, a2: (1.0 - alpha) / a0, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// The notches of one bell, on while it rings.
struct Bank {
    bell: usize,
    notches: Vec<Notch>,
    /// Samples left until the sound is over.
    left: u64,
    /// 0 = signal untouched, 1 = filtered.
    mix: f32,
}

pub struct BellFilter {
    sr: u32,
    banks: Vec<Bank>,
}

impl BellFilter {
    pub fn new(sr: u32) -> Self {
        Self { sr, banks: Vec::new() }
    }

    /// The bell `kind` was struck now. False for an unknown bell.
    pub fn strike(&mut self, kind: &str) -> bool {
        let Some(bell) = BELLS.iter().position(|(k, _)| *k == kind) else { return false };
        let left = (RING_SECS * self.sr as f32) as u64;
        if let Some(b) = self.banks.iter_mut().find(|b| b.bell == bell) {
            b.left = left;
        } else {
            let nyquist = self.sr as f32 / 2.0;
            let notches = BELLS[bell]
                .1
                .iter()
                .filter(|p| p.0 < 0.95 * nyquist)
                .flat_map(|&(f, beat)| [Notch::new(f - beat / 2.0, self.sr), Notch::new(f + beat / 2.0, self.sr)])
                .collect();
            self.banks.push(Bank { bell, notches, left, mix: 0.0 });
        }
        true
    }

    pub fn active(&self) -> bool {
        !self.banks.is_empty()
    }

    /// Filters `x` in place.
    pub fn process(&mut self, x: &mut [f32]) {
        if self.banks.is_empty() {
            return;
        }
        let step = 1.0 / (FADE_SECS * self.sr as f32).max(1.0);
        for b in self.banks.iter_mut() {
            for v in x.iter_mut() {
                let mut y = *v;
                for n in b.notches.iter_mut() {
                    y = n.run(y);
                }
                if b.left > 0 {
                    b.left -= 1;
                    b.mix = (b.mix + step).min(1.0);
                } else {
                    b.mix = (b.mix - step).max(0.0);
                }
                *v = b.mix * y + (1.0 - b.mix) * *v;
            }
        }
        self.banks.retain(|b| b.left > 0 || b.mix > 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(hz: f32, sr: u32, secs: f32) -> Vec<f32> {
        (0..(secs * sr as f32) as usize).map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / sr as f32).sin()).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn a_partial_is_removed_and_the_voice_kept() {
        let sr = 48_000;
        let mut f = BellFilter::new(sr);
        assert!(f.strike("high"));
        let mut bell = tone(1876.0, sr, 1.0);
        f.process(&mut bell);
        assert!(rms(&bell[sr as usize / 2..]) < 0.01, "partial left: {}", rms(&bell[sr as usize / 2..]));
        let mut f = BellFilter::new(sr);
        f.strike("high");
        let mut voice = tone(220.0, sr, 1.0);
        f.process(&mut voice);
        assert!((rms(&voice[sr as usize / 2..]) - 0.707).abs() < 0.01);
    }

    #[test]
    fn it_stops_after_the_bell() {
        let sr = 16_000;
        let mut f = BellFilter::new(sr);
        assert!(!f.strike("gong"));
        assert!(f.strike("deep"));
        let mut x = vec![0.0; (7.0 * sr as f32) as usize];
        f.process(&mut x);
        assert!(!f.active());
        // untouched afterwards
        let mut y = tone(146.0, sr, 0.5);
        let z = y.clone();
        f.process(&mut y);
        assert_eq!(y, z);
    }
}
