//! Acoustic model of "Nam-myoho-renge-kyo".
//!
//! The phrase is modelled as 6 syllables in a fixed order
//! (Nam, myo, ho, ren, ge, kyo), each made of `K` consecutive states
//! with a diagonal-Gaussian description of the sound (MFCC + deltas).
//! Every state may last any number of frames, which is what makes the
//! model independent of the recitation speed.
//!
//! Next to the phrase there is a "garbage" model (a small Gaussian
//! mixture of generic speech and of the user's own voice): a stretch of
//! audio only counts as a Daimoku if the phrase model explains it
//! clearly better than the garbage model does.

use crate::features::FEAT_DIM;
use serde::{Deserialize, Serialize};

pub const N_SYL: usize = 6;
/// States per syllable.
pub const K: usize = 3;
pub const N_CHAIN: usize = N_SYL * K;
/// Optional pause slots between two consecutive syllables.
pub const N_PAUSE: usize = N_SYL - 1;
pub const NODE_PAUSE0: usize = N_CHAIN;
pub const NODE_SIL: usize = N_CHAIN + N_PAUSE;
pub const NODE_GAR: usize = NODE_SIL + 1;
pub const N_NODES: usize = NODE_GAR + 1;

pub const SYLLABLES: [&str; N_SYL] = ["Nam", "myo", "ho", "ren", "ge", "kyo"];

/// Gaussians per chain state. The same syllable sounds different when
/// chanted fast (short, blended into its neighbours) and slowly (long
/// sustained vowel): two components let one state cover both.
pub const MIX: usize = 2;

/// One analysis frame ready for the decoder (mean-normalised features).
#[derive(Clone, Copy, Debug)]
pub struct Feat {
    pub x: [f32; FEAT_DIM],
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gmm {
    pub log_w: Vec<f32>,
    pub mu: Vec<f32>,
    pub var: Vec<f32>,
}

impl Gmm {
    pub fn n_comp(&self) -> usize {
        self.log_w.len()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    /// N_CHAIN x MIX x FEAT_DIM
    pub mu: Vec<f32>,
    pub var: Vec<f32>,
    /// N_CHAIN x MIX log mixture weights.
    #[serde(default)]
    pub log_w: Vec<f32>,
    pub garb: Gmm,
    /// Average raw cepstrum of the training material (prior for the
    /// online mean normalisation).
    pub cep_mean: Vec<f32>,
    /// Typical log-likelihood-ratio per frame (phrase vs garbage) of a
    /// correctly recognised Daimoku, measured on the training takes.
    pub ref_llr: f32,
    /// Shortest / longest Daimoku seen in training (frames of 10 ms).
    pub min_cycle_frames: u32,
    pub max_cycle_frames: u32,
}

const LN_2PI: f32 = 1.837_877_1;
/// Score of a syllable state on a quiet frame (garbage scores -2 there).
pub const QUIET_IN_SYLLABLE: f32 = -4.0;
/// Per-frame cost (vs. the garbage model) of voiced audio in a pause slot.
const PAUSE_VOICED_COST: f32 = 3.0;
const LL_FLOOR: f32 = -600.0;

/// Precomputed constants for fast frame scoring.
#[derive(Clone)]
pub struct Scorer {
    mix: usize,
    mu: Vec<f32>,
    inv_var: Vec<f32>,
    cst: Vec<f32>,
    log_w: Vec<f32>,
    g_logw: Vec<f32>,
    g_mu: Vec<f32>,
    g_inv: Vec<f32>,
    g_cst: Vec<f32>,
}

fn consts(var: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = var.len() / FEAT_DIM;
    let mut inv = Vec::with_capacity(var.len());
    let mut cst = Vec::with_capacity(n);
    for s in 0..n {
        let mut c = 0.0f32;
        for d in 0..FEAT_DIM {
            let v = var[s * FEAT_DIM + d].max(1e-4);
            inv.push(1.0 / v);
            c += LN_2PI + v.ln();
        }
        cst.push(-0.5 * c);
    }
    (inv, cst)
}

#[inline]
fn gauss(mu: &[f32], inv: &[f32], cst: f32, x: &[f32; FEAT_DIM]) -> f32 {
    let mut q = 0.0f32;
    for d in 0..FEAT_DIM {
        let z = x[d] - mu[d];
        q += z * z * inv[d];
    }
    (cst - 0.5 * q).max(LL_FLOOR)
}

impl Scorer {
    pub fn new(m: &Model) -> Self {
        let (inv, cst) = consts(&m.var);
        let (g_inv, g_cst) = consts(&m.garb.var);
        let mix = (m.var.len() / (N_CHAIN * FEAT_DIM)).max(1);
        let log_w = if m.log_w.len() == N_CHAIN * mix {
            m.log_w.clone()
        } else {
            vec![-(mix as f32).ln(); N_CHAIN * mix]
        };
        Self {
            mix,
            mu: m.mu.clone(),
            inv_var: inv,
            cst,
            log_w,
            g_logw: m.garb.log_w.clone(),
            g_mu: m.garb.mu.clone(),
            g_inv,
            g_cst,
        }
    }

    /// Log-likelihood of `x` under one component of a chain state.
    #[inline]
    pub fn comp_ll(&self, s: usize, c: usize, x: &[f32; FEAT_DIM]) -> f32 {
        let k = s * self.mix + c;
        self.log_w[k]
            + gauss(
                &self.mu[k * FEAT_DIM..(k + 1) * FEAT_DIM],
                &self.inv_var[k * FEAT_DIM..(k + 1) * FEAT_DIM],
                self.cst[k],
                x,
            )
    }

    pub fn mix(&self) -> usize {
        self.mix
    }

    #[inline]
    pub fn chain_ll(&self, s: usize, x: &[f32; FEAT_DIM]) -> f32 {
        let mut best = f32::MIN;
        let mut lls = [0.0f32; 8];
        let m = self.mix.min(8);
        for c in 0..m {
            let ll = self.comp_ll(s, c, x);
            lls[c] = ll;
            best = best.max(ll);
        }
        if m == 1 {
            return best;
        }
        let mut acc = 0.0f32;
        for c in 0..m {
            acc += (lls[c] - best).exp();
        }
        (best + acc.ln()).max(LL_FLOOR)
    }

    pub fn gar_ll(&self, x: &[f32; FEAT_DIM]) -> f32 {
        let m = self.g_logw.len();
        let mut best = f32::MIN;
        let mut lls = [0.0f32; 16];
        let m = m.min(16);
        for c in 0..m {
            let ll = self.g_logw[c]
                + gauss(
                    &self.g_mu[c * FEAT_DIM..(c + 1) * FEAT_DIM],
                    &self.g_inv[c * FEAT_DIM..(c + 1) * FEAT_DIM],
                    self.g_cst[c],
                    x,
                );
            lls[c] = ll;
            if ll > best {
                best = ll;
            }
        }
        let mut s = 0.0f32;
        for c in 0..m {
            s += (lls[c] - best).exp();
        }
        best + s.ln()
    }

    /// Emission scores of every decoder node for one frame (decoding).
    pub fn emissions(&self, f: &Feat, out: &mut [f32; N_NODES]) {
        if f.active {
            let g = self.gar_ll(&f.x);
            for s in 0..N_CHAIN {
                out[s] = self.chain_ll(s, &f.x);
            }
            out[NODE_GAR] = g;
            // Voiced frames may sit between two syllables (a held "n", a
            // breath) at a small cost per frame instead of forcing the
            // decoder to abandon the phrase; `engine::accept` limits how
            // much of a phrase may be such filler.
            let sil = g - PAUSE_VOICED_COST;
            for p in 0..N_PAUSE {
                out[NODE_PAUSE0 + p] = sil;
            }
            out[NODE_SIL] = sil;
        } else {
            // A quiet frame inside a syllable (the closure of "k", "g", a
            // dip of a compressed recording) costs less than leaving the
            // phrase for garbage would.
            for s in 0..N_CHAIN {
                out[s] = QUIET_IN_SYLLABLE;
            }
            out[NODE_GAR] = -2.0;
            for p in 0..N_PAUSE {
                out[NODE_PAUSE0 + p] = 0.0;
            }
            out[NODE_SIL] = 0.0;
        }
    }

    /// Emission scores used when *aligning* a take whose content is known
    /// (training): no garbage, pauses only on quiet frames.
    pub fn train_emissions(&self, f: &Feat, out: &mut [f32; N_NODES]) {
        if f.active {
            for s in 0..N_CHAIN {
                out[s] = self.chain_ll(s, &f.x);
            }
            out[NODE_GAR] = LL_FLOOR;
            for p in 0..N_PAUSE {
                out[NODE_PAUSE0 + p] = -1000.0;
            }
            out[NODE_SIL] = -1000.0;
        } else {
            for s in 0..N_CHAIN {
                out[s] = -8.0;
            }
            out[NODE_GAR] = LL_FLOOR;
            for p in 0..N_PAUSE {
                out[NODE_PAUSE0 + p] = 0.0;
            }
            out[NODE_SIL] = 0.0;
        }
    }
}

