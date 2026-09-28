//! Audio front-end: streaming MFCC extraction, no external dependencies.
//!
//! Turns raw microphone samples (any sample rate >= 16 kHz) into one
//! feature frame every 10 ms:
//!   - 12 cepstral coefficients (c1..c12): the "timbre" of the sound,
//!     independent of loudness (gain only affects c0, which we drop);
//!   - 12 delta coefficients: how that timbre is moving;
//!   - the frame energy and an "active" flag (is somebody speaking?).
//!
//! The same code path is used for training, batch validation and live
//! counting, so what the model learns is exactly what it later sees.

use std::collections::VecDeque;
use std::f64::consts::PI;

pub const N_MEL: usize = 40;
pub const N_CEP: usize = 12;
pub const FEAT_DIM: usize = 2 * N_CEP;

/// Frame hop in seconds (10 ms).
pub const HOP_SECS: f32 = 0.010;

const WIN_SECS: f32 = 0.025;
const PREEMPH: f32 = 0.97;
const TARGET_SR: u32 = 16_000;

/// Frames whose mean power is below this are never "active" (about
/// -54 dBFS RMS): microphone hiss, digital silence.
const ABS_ENERGY_FLOOR: f32 = -12.4; // ln(4e-6)
/// A frame is active only if it is within this many nats (ln units of
/// power, 6.5 nats ~ 28 dB) of the recent loudest frame.
const GATE_NATS: f32 = 6.5;
/// The "recent loudest frame" tracker forgets slowly (1.5 nats/s), so a
/// single loud bump cannot deafen the counter for long.
const PEAK_DECAY_PER_FRAME: f32 = 0.015;

/// One analysis frame as produced by [`FrontEnd`].
#[derive(Clone, Copy, Debug)]
pub struct RawFrame {
    pub cep: [f32; N_CEP],
    pub delta: [f32; N_CEP],
    /// ln(mean power) of the frame.
    pub energy: f32,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// FFT (iterative radix-2)
// ---------------------------------------------------------------------------

struct Fft {
    n: usize,
    cos: Vec<f64>,
    sin: Vec<f64>,
    rev: Vec<usize>,
}

impl Fft {
    fn new(n: usize) -> Self {
        assert!(n.is_power_of_two() && n >= 4);
        let bits = n.trailing_zeros();
        let rev = (0..n)
            .map(|i| i.reverse_bits() >> (usize::BITS - bits))
            .collect();
        let cos = (0..n / 2).map(|k| (2.0 * PI * k as f64 / n as f64).cos()).collect();
        let sin = (0..n / 2).map(|k| (2.0 * PI * k as f64 / n as f64).sin()).collect();
        Self { n, cos, sin, rev }
    }

    fn transform(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            let mut start = 0;
            while start < n {
                for k in 0..half {
                    let wr = self.cos[k * step];
                    let wi = -self.sin[k * step];
                    let a = start + k;
                    let b = a + half;
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
                start += len;
            }
            len *= 2;
        }
    }
}

// ---------------------------------------------------------------------------
// Mel filterbank + DCT
// ---------------------------------------------------------------------------

fn hz_to_mel(f: f64) -> f64 {
    2595.0 * (1.0 + f / 700.0).log10()
}
fn mel_to_hz(m: f64) -> f64 {
    700.0 * (10f64.powf(m / 2595.0) - 1.0)
}

struct MelBank {
    /// (first FFT bin, weights) per mel band.
    bands: Vec<(usize, Vec<f32>)>,
}

impl MelBank {
    fn new(sr: f64, nfft: usize) -> Self {
        let fmin = 100.0;
        let fmax = 7000.0f64.min(sr * 0.47);
        let (mlo, mhi) = (hz_to_mel(fmin), hz_to_mel(fmax));
        let pts: Vec<f64> = (0..N_MEL + 2)
            .map(|i| mel_to_hz(mlo + (mhi - mlo) * i as f64 / (N_MEL + 1) as f64))
            .collect();
        let bin_hz = sr / nfft as f64;
        let mut bands = Vec::with_capacity(N_MEL);
        for m in 0..N_MEL {
            let (l, c, r) = (pts[m], pts[m + 1], pts[m + 2]);
            let first = (l / bin_hz).floor() as usize;
            let last = ((r / bin_hz).ceil() as usize).min(nfft / 2);
            let mut w = Vec::new();
            for k in first..=last {
                let f = k as f64 * bin_hz;
                let v = if f > l && f <= c {
                    (f - l) / (c - l)
                } else if f > c && f < r {
                    (r - f) / (r - c)
                } else {
                    0.0
                };
                w.push(v as f32);
            }
            bands.push((first, w));
        }
        Self { bands }
    }
}

fn dct_matrix() -> Vec<f32> {
    // Orthonormal DCT-II rows 1..=N_CEP (row 0 = c0 is dropped).
    let mut m = vec![0.0f32; N_CEP * N_MEL];
    let scale = (2.0 / N_MEL as f64).sqrt();
    for k in 0..N_CEP {
        for n in 0..N_MEL {
            m[k * N_MEL + n] =
                (scale * (PI * (k + 1) as f64 * (n as f64 + 0.5) / N_MEL as f64).cos()) as f32;
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Streaming front-end
// ---------------------------------------------------------------------------

pub struct FrontEnd {
    /// Integer decimation factor applied first (box filter), so the FFT
    /// always works near 16-22 kHz whatever the device sample rate is.
    decim: usize,
    dec_acc: f32,
    dec_n: usize,

    win: usize,
    hop: usize,
    nfft: usize,
    fft: Fft,
    window: Vec<f64>,
    mel: MelBank,
    dct: Vec<f32>,

    buf: Vec<f32>,
    re: Vec<f64>,
    im: Vec<f64>,

    /// Frames waiting for their +-2 neighbours so deltas can be computed.
    hist: VecDeque<(f32, [f32; N_CEP], bool)>,
    started: bool,

    peak: f32,
}

impl FrontEnd {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate.max(8_000);
        let decim = ((sr / TARGET_SR) as usize).max(1);
        let eff = sr as f64 / decim as f64;
        let win = ((eff * WIN_SECS as f64).round() as usize).max(32);
        let hop = ((eff * HOP_SECS as f64).round() as usize).max(8);
        let nfft = win.next_power_of_two();
        Self {
            decim,
            dec_acc: 0.0,
            dec_n: 0,
            win,
            hop,
            nfft,
            fft: Fft::new(nfft),
            window: (0..win)
                .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f64 / (win - 1) as f64).cos())
                .collect(),
            mel: MelBank::new(eff, nfft),
            dct: dct_matrix(),
            buf: Vec::new(),
            re: vec![0.0; nfft],
            im: vec![0.0; nfft],
            hist: VecDeque::new(),
            started: false,
            peak: f32::MIN,
        }
    }

    /// Feeds samples, appending finished frames to `out`.
    pub fn push(&mut self, samples: &[f32], out: &mut Vec<RawFrame>) {
        if self.decim == 1 {
            self.buf.extend_from_slice(samples);
        } else {
            for &s in samples {
                self.dec_acc += s;
                self.dec_n += 1;
                if self.dec_n == self.decim {
                    self.buf.push(self.dec_acc / self.decim as f32);
                    self.dec_acc = 0.0;
                    self.dec_n = 0;
                }
            }
        }
        let mut pos = 0;
        while self.buf.len() - pos >= self.win {
            let (energy, cep) = self.analyse(pos);
            self.push_frame(energy, cep, out);
            pos += self.hop;
        }
        if pos > 0 {
            self.buf.drain(..pos);
        }
    }

    /// Emits the last frames still waiting for lookahead.
    pub fn flush(&mut self, out: &mut Vec<RawFrame>) {
        if !self.started {
            return;
        }
        if let Some(&last) = self.hist.back() {
            for _ in 0..2 {
                self.hist.push_back(last);
                self.emit_center(out);
            }
        }
    }

    fn analyse(&mut self, pos: usize) -> (f32, [f32; N_CEP]) {
        let frame = &self.buf[pos..pos + self.win];
        let mut pw = 0.0f64;
        for &x in frame {
            pw += (x as f64) * (x as f64);
        }
        let energy = ((pw / self.win as f64) + 1e-10).ln() as f32;

        for i in 0..self.nfft {
            self.re[i] = 0.0;
            self.im[i] = 0.0;
        }
        let mut prev = frame[0];
        for i in 0..self.win {
            let x = frame[i];
            let pre = if i == 0 { x } else { x - PREEMPH * prev };
            prev = x;
            self.re[i] = pre as f64 * self.window[i];
        }
        self.fft.transform(&mut self.re, &mut self.im);

        let mut logmel = [0.0f32; N_MEL];
        for (m, (first, w)) in self.mel.bands.iter().enumerate() {
            let mut acc = 0.0f64;
            for (j, &wt) in w.iter().enumerate() {
                let k = first + j;
                if k <= self.nfft / 2 {
                    acc += wt as f64 * (self.re[k] * self.re[k] + self.im[k] * self.im[k]);
                }
            }
            logmel[m] = (acc + 1e-10).ln() as f32;
        }
        let mut cep = [0.0f32; N_CEP];
        for k in 0..N_CEP {
            let row = &self.dct[k * N_MEL..(k + 1) * N_MEL];
            let mut s = 0.0f32;
            for n in 0..N_MEL {
                s += row[n] * logmel[n];
            }
            cep[k] = s;
        }
        (energy, cep)
    }

    fn push_frame(&mut self, energy: f32, cep: [f32; N_CEP], out: &mut Vec<RawFrame>) {
        // Activity gate.
        self.peak = if self.peak == f32::MIN {
            energy
        } else {
            energy.max(self.peak - PEAK_DECAY_PER_FRAME)
        };
        let active = energy > ABS_ENERGY_FLOOR && energy > self.peak - GATE_NATS;

        let item = (energy, cep, active);
        if !self.started {
            self.started = true;
            self.hist.push_back(item);
            self.hist.push_back(item);
        }
        self.hist.push_back(item);
        while self.hist.len() >= 5 {
            self.emit_center(out);
        }
    }

    /// Emits the frame in the middle of the 5-frame history window.
    fn emit_center(&mut self, out: &mut Vec<RawFrame>) {
        if self.hist.len() < 3 {
            return;
        }
        // Make sure we have 5 frames to look at (pad by repetition).
        while self.hist.len() < 5 {
            let last = *self.hist.back().unwrap();
            self.hist.push_back(last);
        }
        let h = &self.hist;
        let mut delta = [0.0f32; N_CEP];
        for k in 0..N_CEP {
            delta[k] = (h[3].1[k] - h[1].1[k] + 2.0 * (h[4].1[k] - h[0].1[k])) / 10.0;
        }
        out.push(RawFrame {
            cep: h[2].1,
            delta,
            energy: h[2].0,
            active: h[2].2,
        });
        self.hist.pop_front();
    }
}

/// Convenience: whole buffer -> frames.
pub fn extract_frames(samples: &[f32], sample_rate: u32) -> Vec<RawFrame> {
    let mut fe = FrontEnd::new(sample_rate);
    let mut out = Vec::new();
    for chunk in samples.chunks(4096) {
        fe.push(chunk, &mut out);
    }
    fe.flush(&mut out);
    out
}

// ---------------------------------------------------------------------------
// Online cepstral mean normalisation
// ---------------------------------------------------------------------------

/// Running estimate of the average cepstrum of the speaker/microphone
/// channel, updated on active frames only. Starts from a prior (the mean
/// seen at training time) so the very first repetitions are already
/// normalised sensibly.
#[derive(Clone, Debug)]
pub struct OnlineCmn {
    mean: [f32; N_CEP],
    wsum: f32,
}

const CMN_PRIOR_WEIGHT: f32 = 120.0;
const CMN_MAX_WINDOW: f32 = 600.0;

impl OnlineCmn {
    pub fn new(prior: &[f32]) -> Self {
        let mut mean = [0.0f32; N_CEP];
        for (i, v) in prior.iter().take(N_CEP).enumerate() {
            mean[i] = *v;
        }
        Self {
            mean,
            wsum: CMN_PRIOR_WEIGHT,
        }
    }

    pub fn update(&mut self, cep: &[f32; N_CEP], active: bool) {
        if !active {
            return;
        }
        self.wsum = (self.wsum + 1.0).min(CMN_MAX_WINDOW);
        let a = 1.0 / self.wsum;
        for k in 0..N_CEP {
            self.mean[k] += a * (cep[k] - self.mean[k]);
        }
    }

    pub fn normalise(&self, f: &RawFrame) -> [f32; FEAT_DIM] {
        let mut x = [0.0f32; FEAT_DIM];
        for k in 0..N_CEP {
            x[k] = f.cep[k] - self.mean[k];
            x[N_CEP + k] = f.delta[k];
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(sr: u32, secs: f32, hz: f32, amp: f32) -> Vec<f32> {
        (0..(sr as f32 * secs) as usize)
            .map(|i| amp * (2.0 * std::f32::consts::PI * hz * i as f32 / sr as f32).sin())
            .collect()
    }

    #[test]
    fn frame_rate_is_100_per_second() {
        for sr in [16_000u32, 44_100, 48_000] {
            let fr = extract_frames(&tone(sr, 2.0, 220.0, 0.3), sr);
            assert!((195..=205).contains(&fr.len()), "sr {sr}: {} frames", fr.len());
        }
    }

    #[test]
    fn silence_is_inactive_and_tone_is_active() {
        let sil = extract_frames(&vec![0.0; 16_000], 16_000);
        assert!(sil.iter().all(|f| !f.active));
        let t = extract_frames(&tone(16_000, 1.0, 300.0, 0.3), 16_000);
        assert!(t.iter().filter(|f| f.active).count() > 90);
    }

    #[test]
    fn cepstrum_is_gain_invariant() {
        let a = extract_frames(&tone(16_000, 1.0, 300.0, 0.4), 16_000);
        let b = extract_frames(&tone(16_000, 1.0, 300.0, 0.04), 16_000);
        let (fa, fb) = (a[50], b[50]);
        for k in 0..N_CEP {
            assert!((fa.cep[k] - fb.cep[k]).abs() < 0.05, "c{} differs", k + 1);
        }
    }

    #[test]
    fn different_sample_rates_give_similar_cepstra() {
        let a = extract_frames(&tone(16_000, 1.0, 500.0, 0.3), 16_000);
        let b = extract_frames(&tone(48_000, 1.0, 500.0, 0.3), 48_000);
        let d: f32 = (0..N_CEP).map(|k| (a[50].cep[k] - b[50].cep[k]).abs()).sum::<f32>() / N_CEP as f32;
        assert!(d < 1.0, "mean cepstral difference {d}");
    }
}
