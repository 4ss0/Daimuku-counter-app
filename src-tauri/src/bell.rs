//! The bell rung from the Count screen, taken out of the microphone signal
//! before the counter hears it.
//!
//! The bells are synthetic (tools/bells/make.py): a few partials at known
//! frequencies, gone within 6 s. After a strike every partial of that bell
//! is cut by a narrow notch filter until the sound is over; the voice loses
//! only those narrow bands. The filter fades in and out over a few
//! milliseconds, so it never clicks.
//!
//! The user's own bell (a real one) is handled the same way, with two
//! differences: its partials are not known in advance, they are measured on
//! a recording in which it rings (`learn_partials`, from the guided
//! recording of the Voice page); and nobody tells the filter when it is
//! struck, it hears it (`BellFilter::set_partials`): the notches work while
//! the partials stand out of the sound around them.

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

/// Band-pass (0 dB at the centre), the complement of `Notch`.
#[derive(Clone)]
struct Band {
    b0: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Band {
    fn new(hz: f32, sr: u32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * hz / sr as f32;
        let alpha = w0.sin() / (2.0 * Q);
        let a0 = 1.0 + alpha;
        Self { b0: alpha / a0, a1: -2.0 * w0.cos() / a0, a2: (1.0 - alpha) / a0, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = -self.a1 * y + self.z2;
        self.z2 = -self.b0 * x - self.a2 * y;
        y
    }
}

// --- the user's own bell ----------------------------------------------------

/// At most this many partials are learned and cut.
pub const MAX_PARTIALS: usize = 8;
/// Partials are looked for between these frequencies (below, the voice's
/// own lines are as narrow and strong as a bell's).
const LEARN_MIN_HZ: f32 = 500.0;
const LEARN_MAX_HZ: f32 = 12_000.0;
/// A partial stands this far (dB) above the median level within
/// `LEARN_AROUND_HZ` of it, and above the level `LEARN_NEAR_HZ` to its
/// sides (a held note of the voice is wider than a bell's partial).
const LEARN_PROMINENCE_DB: f32 = 20.0;
const LEARN_AROUND_HZ: f32 = 150.0;
const LEARN_NEAR_HZ: (f32, f32) = (12.0, 40.0);
const LEARN_NARROW_DB: f32 = 15.0;
/// ... and is at most this far (dB) below the strongest line of the whole
/// recording (the voice's included): what lies deeper is not heard.
const LEARN_RANGE_DB: f32 = 50.0;
/// Lines that are harmonics (up to this one) of one pitch in this range,
/// each within this share of its frequency, are a voice.
const VOICE_PITCH_HZ: (f32, f32) = (80.0, 400.0);
const VOICE_MAX_HARMONIC: u32 = 12;
const VOICE_TOLERANCE: f32 = 0.005;
/// The bell is told by its partials above this frequency, where the voice
/// has no narrow lines; at least `DETECT_NEED` of them must ring together.
const DETECT_MIN_HZ: f32 = 1000.0;
const DETECT_NEED: usize = 2;
/// A partial rings while the band on it holds this much more power (9 dB)
/// than the bands `DETECT_SIDE` below and above it.
const DETECT_RATIO: f32 = 8.0;
const DETECT_SIDE: f32 = 0.09;
/// Levels are measured on blocks of 10 ms, smoothed over about 30 ms.
const DETECT_SMOOTH: f32 = 0.35;
/// The notches stay on this long after the bell was last heard: its tail
/// goes on under the voice, too soft to be told from it.
const DETECT_HANG_SECS: f32 = 3.0;

/// The partials (Hz) of a bell ringing in `samples`, strongest first, or
/// nothing when no bell is heard: narrow lines that stand far above the
/// sound around them, at least `DETECT_NEED` of them above `DETECT_MIN_HZ`.
/// The voice may be chanting over the bell.
pub fn learn_partials(samples: &[f32], sr: u32) -> Vec<f32> {
    use crate::features::Fft;
    if sr < 8_000 {
        return Vec::new();
    }
    // about 0.7-1 s windows: lines 1-1.5 Hz apart
    let nfft = ((sr as f32 * 0.68) as usize).next_power_of_two();
    if samples.len() < nfft * 2 {
        return Vec::new();
    }
    let fft = Fft::new(nfft);
    let window: Vec<f64> = (0..nfft)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (nfft - 1) as f64).cos())
        .collect();
    let bins = nfft / 2 + 1;
    let mut power = vec![0.0f64; bins];
    let (mut re, mut im) = (vec![0.0f64; nfft], vec![0.0f64; nfft]);
    let mut start = 0;
    while start + nfft <= samples.len() {
        for i in 0..nfft {
            re[i] = samples[start + i] as f64 * window[i];
            im[i] = 0.0;
        }
        fft.transform(&mut re, &mut im);
        for k in 0..bins {
            power[k] += re[k] * re[k] + im[k] * im[k];
        }
        start += nfft / 4;
    }
    let db: Vec<f32> = power.iter().map(|p| 10.0 * (p + 1e-30).log10() as f32).collect();
    let hz = sr as f32 / nfft as f32;
    let bin = |f: f32| (f / hz).round() as usize;
    let lo = bin(LEARN_MIN_HZ).max(1);
    let hi = bin(LEARN_MAX_HZ.min(0.45 * sr as f32)).min(bins - 2);
    let around = bin(LEARN_AROUND_HZ).max(4);
    let (near0, near1) = (bin(LEARN_NEAR_HZ.0).max(2), bin(LEARN_NEAR_HZ.1).max(4));
    let median = |v: &mut Vec<f32>| -> f32 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };
    // (prominence, frequency, level)
    let mut found: Vec<(f32, f32, f32)> = Vec::new();
    for k in lo..=hi {
        if db[k] < db[k - 1] || db[k] < db[k + 1] {
            continue;
        }
        let mut wide: Vec<f32> = db[k.saturating_sub(around)..(k + around + 1).min(bins)].to_vec();
        let prominence = db[k] - median(&mut wide);
        if prominence < LEARN_PROMINENCE_DB {
            continue;
        }
        let mut sides: Vec<f32> = db[k.saturating_sub(near1)..k.saturating_sub(near0)].to_vec();
        sides.extend_from_slice(&db[(k + near0).min(bins)..(k + near1).min(bins)]);
        if sides.is_empty() || db[k] - median(&mut sides) < LEARN_NARROW_DB {
            continue;
        }
        // centre of the line (parabola through the three bins)
        let (a, b, c) = (db[k - 1], db[k], db[k + 1]);
        let d = if (a - 2.0 * b + c).abs() > 1e-6 { 0.5 * (a - c) / (a - 2.0 * b + c) } else { 0.0 };
        found.push((prominence, (k as f32 + d.clamp(-0.5, 0.5)) * hz, db[k]));
    }
    let top = db.iter().cloned().fold(f32::MIN, f32::max);
    found.retain(|f| f.2 >= top - LEARN_RANGE_DB);
    found.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    // one notch covers frequency / Q: lines closer than that are one partial
    // (the partials of a bell often come in pairs a few Hz apart)
    let mut freqs: Vec<f32> = Vec::new();
    for (_, f, _) in found {
        if !freqs.iter().any(|&g| (g - f).abs() < 0.6 * g / Q) {
            freqs.push(f);
        }
    }
    // a voice holding one pitch draws narrow lines too, but they are
    // harmonics: three or more lines that are low multiples of one pitch
    // are the voice, not the bell (whose partials are not harmonic)
    let mut voice = vec![false; freqs.len()];
    for &f in &freqs {
        for n in 1..=VOICE_MAX_HARMONIC {
            let pitch = f / n as f32;
            if !(VOICE_PITCH_HZ.0..=VOICE_PITCH_HZ.1).contains(&pitch) {
                continue;
            }
            let fits: Vec<usize> = (0..freqs.len())
                .filter(|&i| {
                    let h = (freqs[i] / pitch).round();
                    h >= 1.0 && h <= VOICE_MAX_HARMONIC as f32 && (freqs[i] - h * pitch).abs() <= VOICE_TOLERANCE * freqs[i]
                })
                .collect();
            if fits.len() >= 3 {
                for i in fits {
                    voice[i] = true;
                }
            }
        }
    }
    let out: Vec<f32> = freqs.iter().zip(&voice).filter(|(_, v)| !**v).map(|(f, _)| *f).take(MAX_PARTIALS).collect();
    if out.iter().filter(|&&f| f >= DETECT_MIN_HZ).count() < DETECT_NEED {
        return Vec::new();
    }
    out
}

/// `samples` as the counter hears them when the bell of `partials` is the
/// user's: for takes and recordings analysed after the fact.
pub fn without_bell(samples: &[f32], sr: u32, partials: &[f32]) -> Vec<f32> {
    let mut f = BellFilter::new(sr);
    f.set_partials(partials);
    let mut x = samples.to_vec();
    if f.active() {
        for c in x.chunks_mut(1024) {
            f.process(c);
        }
    }
    x
}

/// The user's bell: its notches, and the probes that hear it ring.
struct Own {
    notches: Vec<Notch>,
    /// Per partial above `DETECT_MIN_HZ`: bands on it, below and above it.
    probes: Vec<[Band; 3]>,
    acc: Vec<[f32; 3]>,
    level: Vec<[f32; 3]>,
    block: usize,
    n: usize,
    /// Blocks left with the notches on.
    hang: u32,
    mix: f32,
}

pub struct BellFilter {
    sr: u32,
    banks: Vec<Bank>,
    own: Option<Own>,
}

impl BellFilter {
    pub fn new(sr: u32) -> Self {
        Self { sr, banks: Vec::new(), own: None }
    }

    /// The partials of the user's own bell (see `learn_partials`): from now
    /// on it is heard and cut by itself. Nothing happens with fewer than
    /// `DETECT_NEED` partials above `DETECT_MIN_HZ`.
    pub fn set_partials(&mut self, partials: &[f32]) {
        let nyquist = self.sr as f32 / 2.0;
        let ok: Vec<f32> = partials
            .iter()
            .cloned()
            .filter(|f| f.is_finite() && *f >= 100.0 && *f < 0.9 * nyquist / (1.0 + DETECT_SIDE))
            .take(MAX_PARTIALS)
            .collect();
        let high: Vec<f32> = ok.iter().cloned().filter(|&f| f >= DETECT_MIN_HZ).collect();
        if high.len() < DETECT_NEED {
            self.own = None;
            return;
        }
        let sr = self.sr;
        self.own = Some(Own {
            notches: ok.iter().map(|&f| Notch::new(f, sr)).collect(),
            probes: high
                .iter()
                .map(|&f| [Band::new(f, sr), Band::new(f * (1.0 - DETECT_SIDE), sr), Band::new(f * (1.0 + DETECT_SIDE), sr)])
                .collect(),
            acc: vec![[0.0; 3]; high.len()],
            level: vec![[0.0; 3]; high.len()],
            block: (sr as usize / 100).max(1),
            n: 0,
            hang: 0,
            mix: 0.0,
        });
    }

    /// The user's own bell is being cut right now.
    pub fn own_ringing(&self) -> bool {
        self.own.as_ref().is_some_and(|o| o.hang > 0)
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

    /// There is something to do: a bell of the app is ringing, or the user
    /// has a bell of their own to listen for.
    pub fn active(&self) -> bool {
        !self.banks.is_empty() || self.own.is_some()
    }

    /// Filters `x` in place.
    pub fn process(&mut self, x: &mut [f32]) {
        let step = 1.0 / (FADE_SECS * self.sr as f32).max(1.0);
        if let Some(o) = self.own.as_mut() {
            let hang_blocks = (DETECT_HANG_SECS * 100.0) as u32;
            for v in x.iter_mut() {
                // listen on the sound as it comes in
                for (p, a) in o.probes.iter_mut().zip(o.acc.iter_mut()) {
                    for j in 0..3 {
                        let y = p[j].run(*v);
                        a[j] += y * y;
                    }
                }
                o.n += 1;
                if o.n == o.block {
                    let mut ringing = 0;
                    for (a, l) in o.acc.iter_mut().zip(o.level.iter_mut()) {
                        for j in 0..3 {
                            l[j] += DETECT_SMOOTH * (a[j] / o.block as f32 - l[j]);
                            a[j] = 0.0;
                        }
                        if l[0] > DETECT_RATIO * 0.5 * (l[1] + l[2]) + 1e-9 {
                            ringing += 1;
                        }
                    }
                    o.n = 0;
                    o.hang = if ringing >= DETECT_NEED { hang_blocks } else { o.hang.saturating_sub(1) };
                }
                // the notches always run, so that they are settled when needed
                let mut y = *v;
                for n in o.notches.iter_mut() {
                    y = n.run(y);
                }
                o.mix = if o.hang > 0 { (o.mix + step).min(1.0) } else { (o.mix - step).max(0.0) };
                *v = o.mix * y + (1.0 - o.mix) * *v;
            }
        }
        if self.banks.is_empty() {
            return;
        }
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

    /// A struck bell: partials decaying over a second or two.
    fn bell(partials: &[(f32, f32)], sr: u32, secs: f32) -> Vec<f32> {
        (0..(secs * sr as f32) as usize)
            .map(|i| {
                let t = i as f32 / sr as f32;
                partials.iter().map(|&(hz, a)| a * (-t / 1.5).exp() * (2.0 * std::f32::consts::PI * hz * t).sin()).sum()
            })
            .collect()
    }

    #[test]
    fn the_partials_of_a_bell_are_learned() {
        let sr = 16_000;
        let x = bell(&[(681.0, 0.3), (1875.0, 0.2), (3480.0, 0.3), (5456.0, 0.1)], sr, 5.0);
        let mut p = learn_partials(&x, sr);
        p.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(p.len(), 4, "{p:?}");
        for (got, want) in p.iter().zip([681.0, 1875.0, 3480.0, 5456.0]) {
            assert!((got - want).abs() < 2.0, "{got} for {want}");
        }
        // a voice-like sound (harmonics of a wavering pitch) is not a bell
        let voice: Vec<f32> = (0..5 * sr as usize)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let phase = 2.0 * std::f32::consts::PI * (140.0 * t + 3.0 * (2.0 * std::f32::consts::PI * 0.7 * t).sin());
                (1..20).map(|h| (h as f32 * phase).sin() / h as f32).sum::<f32>() * 0.1
            })
            .collect();
        assert!(learn_partials(&voice, sr).is_empty());
        // nor is a voice holding one pitch: its lines are harmonics
        let held: Vec<f32> = (0..5 * sr as usize)
            .map(|i| {
                let t = i as f32 / sr as f32;
                [3.0f32, 5.0, 6.0].iter().map(|h| 0.2 * (2.0 * std::f32::consts::PI * 247.0 * h * t).sin()).sum()
            })
            .collect();
        assert!(learn_partials(&held, sr).is_empty(), "{:?}", learn_partials(&held, sr));
    }

    #[test]
    fn the_own_bell_is_heard_and_cut() {
        let sr = 48_000;
        let partials = [1875.0, 3480.0, 5456.0];
        let mut f = BellFilter::new(sr);
        f.set_partials(&partials);
        assert!(f.active());
        // a voice alone passes untouched
        let mut voice = tone(220.0, sr, 1.0);
        let before = voice.clone();
        f.process(&mut voice);
        assert!(!f.own_ringing());
        assert_eq!(voice, before);
        // the bell rings: heard within a tenth of a second, then cut
        let mut x = bell(&[(1875.0, 0.3), (3480.0, 0.3), (5456.0, 0.2)], sr, 1.0);
        let raw = rms(&x[sr as usize / 2..]);
        f.process(&mut x);
        assert!(f.own_ringing());
        assert!(rms(&x[sr as usize / 2..]) < 0.05 * raw, "left: {} of {raw}", rms(&x[sr as usize / 2..]));
        // and the filter lets go a few seconds after it
        let mut quiet = vec![0.0; 4 * sr as usize];
        f.process(&mut quiet);
        assert!(!f.own_ringing());
        // one partial is not a bell
        let mut g = BellFilter::new(sr);
        g.set_partials(&[1875.0]);
        assert!(!g.active());
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
