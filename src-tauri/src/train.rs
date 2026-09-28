//! Model training from labelled takes ("this recording contains exactly
//! N Daimoku").
//!
//! Algorithm: Viterbi re-estimation. Each take is aligned to a chain of
//! exactly N phrases with the current model, the frames assigned to each
//! state are used to re-estimate that state's Gaussian, and the process
//! is repeated a few times. Because the alignment is anchored on the
//! declared count, a take at any speed teaches the model what each
//! syllable sounds like *for that speed and for that speaker*.

use crate::features::*;
use crate::hmm::*;
use crate::model::*;

#[derive(Clone, Debug)]
pub struct TrainItem {
    pub n_cycles: usize,
    pub feats: Vec<Feat>,
    pub weight: f32,
    /// Mean raw cepstrum over the active frames (already subtracted).
    pub raw_mean: [f32; N_CEP],
}

/// Builds a training item from raw audio. `n_cycles` is 0 for material
/// that is not a recitation (normal speech used for the garbage model).
pub fn build_item(samples: &[f32], sr: u32, n_cycles: usize, weight: f32) -> Option<TrainItem> {
    let frames = extract_frames(samples, sr);
    let n_active = frames.iter().filter(|f| f.active).count();
    if n_active < 30 {
        return None;
    }
    let mut mean = [0.0f32; N_CEP];
    for f in frames.iter().filter(|f| f.active) {
        for k in 0..N_CEP {
            mean[k] += f.cep[k];
        }
    }
    for k in 0..N_CEP {
        mean[k] /= n_active as f32;
    }
    let feats = frames
        .iter()
        .map(|f| {
            let mut x = [0.0f32; FEAT_DIM];
            for k in 0..N_CEP {
                x[k] = f.cep[k] - mean[k];
                x[N_CEP + k] = f.delta[k];
            }
            Feat { x, active: f.active }
        })
        .collect();
    Some(TrainItem {
        n_cycles,
        feats,
        weight,
        raw_mean: mean,
    })
}

/// Result of aligning one take.
pub struct Alignment {
    /// Chain state (0..N_CHAIN) of each frame, if it belongs to the phrase.
    pub state: Vec<Option<usize>>,
    /// Cycle index of each frame that is in the phrase.
    pub cycle: Vec<usize>,
}

impl Alignment {
    /// Duration of each cycle (first to last phrase frame), in frames.
    pub fn cycle_spans(&self, n: usize) -> Vec<usize> {
        let mut first = vec![usize::MAX; n];
        let mut last = vec![0usize; n];
        for (t, s) in self.state.iter().enumerate() {
            if s.is_some() {
                let c = self.cycle[t];
                if c < n {
                    first[c] = first[c].min(t);
                    last[c] = last[c].max(t);
                }
            }
        }
        (0..n)
            .filter(|&c| first[c] != usize::MAX)
            .map(|c| last[c] - first[c] + 1)
            .collect()
    }
}

/// Aligns `item` to exactly `item.n_cycles` phrases using `model`.
pub fn align_item(item: &TrainItem, scorer: &Scorer) -> Option<Alignment> {
    let n = item.n_cycles;
    if n == 0 || item.feats.len() < N_CHAIN * n {
        return None;
    }
    let g = Graph::unrolled(n);
    let mut emit = vec![[0.0f32; N_NODES]; item.feats.len()];
    for (t, f) in item.feats.iter().enumerate() {
        scorer.train_emissions(f, &mut emit[t]);
    }
    let path = viterbi_offline(&g, &emit)?;
    let mut state = vec![None; path.len()];
    let mut cycle = vec![0usize; path.len()];
    for (t, &node) in path.iter().enumerate() {
        if let Some((c, s)) = unrolled_node_to_chain(node) {
            state[t] = Some(s);
            cycle[t] = c;
        }
    }
    Some(Alignment { state, cycle })
}

/// Equal-length split of the active region: only used to bootstrap the
/// very first model.
fn uniform_alignment(item: &TrainItem) -> Option<Alignment> {
    let n = item.n_cycles.max(1);
    let first = item.feats.iter().position(|f| f.active)?;
    let last = item.feats.iter().rposition(|f| f.active)?;
    if last <= first {
        return None;
    }
    let span = (last - first + 1) as f32;
    let mut state = vec![None; item.feats.len()];
    let mut cycle = vec![0usize; item.feats.len()];
    for t in first..=last {
        if !item.feats[t].active {
            continue;
        }
        let pos = (t - first) as f32 / span * n as f32;
        let c = (pos.floor() as usize).min(n - 1);
        let q = pos - c as f32;
        state[t] = Some(((q * N_CHAIN as f32) as usize).min(N_CHAIN - 1));
        cycle[t] = c;
    }
    Some(Alignment { state, cycle })
}

/// Frames a Daimoku is "worth" in training, whatever its real length.
const REF_CYCLE_FRAMES: f32 = 100.0;

/// Per-frame weight multiplier that makes every Daimoku count the same in
/// training regardless of its speed. Without it a single very slow take
/// (5 s per Daimoku = 5x the frames of a 1 s one) would drown out every
/// faster example and the model would stop recognising normal speed.
fn speed_norm(item: &TrainItem, al: &Alignment) -> f32 {
    let n = al
        .state
        .iter()
        .zip(&item.feats)
        .filter(|(s, f)| s.is_some() && f.active)
        .count();
    if n == 0 || item.n_cycles == 0 {
        return 1.0;
    }
    (item.n_cycles as f32 * REF_CYCLE_FRAMES / n as f32).clamp(0.1, 3.0)
}

/// Cycles longer than this (frames) seed the "slow" mixture component.
const SLOW_CYCLE_FRAMES: f32 = 200.0;

/// Re-estimates the chain states' Gaussian mixtures.
///
/// Every frame is assigned to one component of its state: on the first
/// pass by tempo (slow takes -> component 1, others -> component 0), then
/// to whichever component explains it best under `prev`. A component that
/// gets too little data is replaced by the state's pooled Gaussian, so a
/// model trained only on fast material degrades gracefully to one
/// Gaussian per state.
fn estimate(items: &[TrainItem], aligns: &[Alignment], prev: Option<&Scorer>) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let nk = N_CHAIN * MIX;
    let mut w = vec![0.0f64; nk];
    let mut sum = vec![0.0f64; nk * FEAT_DIM];
    let mut sum2 = vec![0.0f64; nk * FEAT_DIM];
    let mut gw = 0.0f64;
    let mut gsum = vec![0.0f64; FEAT_DIM];
    let mut gsum2 = vec![0.0f64; FEAT_DIM];

    for (item, al) in items.iter().zip(aligns) {
        let norm = speed_norm(item, al);
        let wt = (item.weight * norm) as f64;
        // frames per cycle of this take
        let n_act = al.state.iter().filter(|s| s.is_some()).count() as f32;
        let per_cycle = n_act / item.n_cycles.max(1) as f32;
        let tempo_comp = if MIX > 1 && per_cycle > SLOW_CYCLE_FRAMES { 1 } else { 0 };
        for (t, s) in al.state.iter().enumerate() {
            let f = &item.feats[t];
            if let (Some(s), true) = (s, f.active) {
                let c = match prev {
                    Some(sc) if sc.mix() == MIX => {
                        let mut best = f32::MIN;
                        let mut arg = 0;
                        for c in 0..MIX {
                            let ll = sc.comp_ll(*s, c, &f.x);
                            if ll > best {
                                best = ll;
                                arg = c;
                            }
                        }
                        arg
                    }
                    _ => tempo_comp,
                };
                let k = *s * MIX + c;
                w[k] += wt;
                gw += wt;
                for d in 0..FEAT_DIM {
                    let v = f.x[d] as f64;
                    sum[k * FEAT_DIM + d] += wt * v;
                    sum2[k * FEAT_DIM + d] += wt * v * v;
                    gsum[d] += wt * v;
                    gsum2[d] += wt * v * v;
                }
            }
        }
    }

    let mut gmean = vec![0.0f64; FEAT_DIM];
    let mut gvar = vec![1.0f64; FEAT_DIM];
    if gw > 0.0 {
        for d in 0..FEAT_DIM {
            gmean[d] = gsum[d] / gw;
            gvar[d] = (gsum2[d] / gw - gmean[d] * gmean[d]).max(1e-3);
        }
    }

    const FLOOR: f64 = 0.20;
    const TAU: f64 = 4.0;
    /// Minimum (weighted) frames for a component to stand on its own.
    const MIN_COMP_W: f64 = 12.0;
    let mut mu = vec![0.0f32; nk * FEAT_DIM];
    let mut var = vec![0.0f32; nk * FEAT_DIM];
    let mut log_w = vec![0.0f32; nk];
    for s in 0..N_CHAIN {
        // pooled statistics of the state
        let ks = s * MIX;
        let pw: f64 = (0..MIX).map(|c| w[ks + c]).sum();
        let mut pm = vec![0.0f64; FEAT_DIM];
        let mut pv = vec![0.0f64; FEAT_DIM];
        for d in 0..FEAT_DIM {
            if pw >= 3.0 {
                let s1: f64 = (0..MIX).map(|c| sum[(ks + c) * FEAT_DIM + d]).sum();
                let s2: f64 = (0..MIX).map(|c| sum2[(ks + c) * FEAT_DIM + d]).sum();
                let m = s1 / pw;
                let v = (s2 / pw - m * m).max(0.0);
                pm[d] = m;
                pv[d] = ((pw * v + TAU * gvar[d]) / (pw + TAU)).max(FLOOR * gvar[d]);
            } else {
                pm[d] = gmean[d];
                pv[d] = 2.0 * gvar[d];
            }
        }
        let strong: Vec<bool> = (0..MIX).map(|c| w[ks + c] >= MIN_COMP_W).collect();
        let n_strong = strong.iter().filter(|&&b| b).count();
        for c in 0..MIX {
            let k = ks + c;
            if n_strong < 2 || !strong[c] {
                // not enough data to split: behave as a single Gaussian
                for d in 0..FEAT_DIM {
                    mu[k * FEAT_DIM + d] = pm[d] as f32;
                    var[k * FEAT_DIM + d] = pv[d] as f32;
                }
                log_w[k] = -(MIX as f32).ln();
                continue;
            }
            for d in 0..FEAT_DIM {
                let i = k * FEAT_DIM + d;
                let m = sum[i] / w[k];
                let v = (sum2[i] / w[k] - m * m).max(0.0);
                let v = (w[k] * v + TAU * gvar[d]) / (w[k] + TAU);
                mu[i] = m as f32;
                var[i] = v.max(FLOOR * gvar[d]) as f32;
            }
            // keep both components in play: weights floored at 20 %
            log_w[k] = ((w[k] / pw).max(0.2) as f32).ln();
        }
        if n_strong >= 2 {
            let z: f32 = (0..MIX).map(|c| log_w[ks + c].exp()).sum();
            for c in 0..MIX {
                log_w[ks + c] -= z.ln();
            }
        }
    }
    (mu, var, log_w)
}

/// Fits a diagonal GMM to weighted frames (k-means start, a few EM steps).
fn fit_gmm(data: &[([f32; FEAT_DIM], f32)], m: usize) -> Gmm {
    let n = data.len();
    let m = m.min(n.max(1));
    // deterministic init: evenly spaced frames
    let mut centers: Vec<[f32; FEAT_DIM]> = (0..m).map(|c| data[(c * n) / m + n / (2 * m)].0).collect();
    let mut assign = vec![0usize; n];

    for _ in 0..6 {
        for (i, (x, _)) in data.iter().enumerate() {
            let mut best = f32::MAX;
            for (c, ctr) in centers.iter().enumerate() {
                let mut dd = 0.0;
                for d in 0..FEAT_DIM {
                    let z = x[d] - ctr[d];
                    dd += z * z;
                }
                if dd < best {
                    best = dd;
                    assign[i] = c;
                }
            }
        }
        let mut acc = vec![[0.0f64; FEAT_DIM]; m];
        let mut cnt = vec![0.0f64; m];
        for (i, (x, w)) in data.iter().enumerate() {
            cnt[assign[i]] += *w as f64;
            for d in 0..FEAT_DIM {
                acc[assign[i]][d] += (*w * x[d]) as f64;
            }
        }
        for c in 0..m {
            if cnt[c] > 0.0 {
                for d in 0..FEAT_DIM {
                    centers[c][d] = (acc[c][d] / cnt[c]) as f32;
                }
            }
        }
    }

    // global variance for floors
    let tw: f64 = data.iter().map(|(_, w)| *w as f64).sum::<f64>().max(1e-9);
    let mut gm = vec![0.0f64; FEAT_DIM];
    for (x, w) in data {
        for d in 0..FEAT_DIM {
            gm[d] += (*w * x[d]) as f64;
        }
    }
    for d in 0..FEAT_DIM {
        gm[d] /= tw;
    }
    let mut gv = vec![0.0f64; FEAT_DIM];
    for (x, w) in data {
        for d in 0..FEAT_DIM {
            let z = x[d] as f64 - gm[d];
            gv[d] += *w as f64 * z * z;
        }
    }
    for d in 0..FEAT_DIM {
        gv[d] = (gv[d] / tw).max(1e-3);
    }

    let mut mu: Vec<f32> = centers.iter().flat_map(|c| c.iter().cloned()).collect();
    let mut var: Vec<f32> = vec![0.0; m * FEAT_DIM];
    let mut logw = vec![(1.0 / m as f32).ln(); m];
    for c in 0..m {
        for d in 0..FEAT_DIM {
            var[c * FEAT_DIM + d] = gv[d] as f32;
        }
    }

    // EM
    let mut resp = vec![0.0f32; m];
    for _ in 0..8 {
        let g = Gmm {
            log_w: logw.clone(),
            mu: mu.clone(),
            var: var.clone(),
        };
        let mut nk = vec![0.0f64; m];
        let mut sx = vec![0.0f64; m * FEAT_DIM];
        let mut sx2 = vec![0.0f64; m * FEAT_DIM];
        for (x, w) in data {
            // responsibilities
            let mut best = f32::MIN;
            for c in 0..m {
                let mut q = 0.0f32;
                let mut cst = 0.0f32;
                for d in 0..FEAT_DIM {
                    let v = g.var[c * FEAT_DIM + d].max(1e-4);
                    let z = x[d] - g.mu[c * FEAT_DIM + d];
                    q += z * z / v;
                    cst += v.ln();
                }
                resp[c] = g.log_w[c] - 0.5 * (q + cst);
                best = best.max(resp[c]);
            }
            let mut z = 0.0f32;
            for c in 0..m {
                resp[c] = (resp[c] - best).exp();
                z += resp[c];
            }
            for c in 0..m {
                let r = (resp[c] / z * *w) as f64;
                nk[c] += r;
                for d in 0..FEAT_DIM {
                    let v = x[d] as f64;
                    sx[c * FEAT_DIM + d] += r * v;
                    sx2[c * FEAT_DIM + d] += r * v * v;
                }
            }
        }
        for c in 0..m {
            if nk[c] < 5.0 {
                continue;
            }
            logw[c] = ((nk[c] / tw) as f32).max(1e-4).ln();
            for d in 0..FEAT_DIM {
                let i = c * FEAT_DIM + d;
                let mean = sx[i] / nk[c];
                let v = (sx2[i] / nk[c] - mean * mean).max(0.25 * gv[d]);
                mu[i] = mean as f32;
                var[i] = v as f32;
            }
        }
    }
    Gmm { log_w: logw, mu, var }
}

fn median(v: &mut Vec<f32>) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = v.len() / 2;
    if v.len() % 2 == 0 {
        (v[m - 1] + v[m]) * 0.5
    } else {
        v[m]
    }
}

/// Everything the trainer reports about the final alignment of each take.
pub struct TakeStats {
    /// Median cycle duration of the take (frames).
    pub cycle_frames: f32,
    /// Mean per-frame log-likelihood ratio (phrase vs garbage).
    pub llr: f32,
}

pub struct Trained {
    pub model: Model,
    pub take_stats: Vec<Option<TakeStats>>,
}

/// Trains (or re-trains) the model on `items`. `speech` are extra frames
/// of ordinary talking used only for the garbage model. If `init` is
/// given, alignment starts from it, otherwise from an equal split.
pub fn train_model(items: &[TrainItem], speech: &[TrainItem], init: Option<&Model>, iters: usize) -> Option<Trained> {
    if items.is_empty() {
        return None;
    }

    let mut aligns: Vec<Alignment> = Vec::new();
    match init {
        Some(m) => {
            let sc = Scorer::new(m);
            for it in items {
                aligns.push(align_item(it, &sc).or_else(|| uniform_alignment(it))?);
            }
        }
        None => {
            for it in items {
                aligns.push(uniform_alignment(it)?);
            }
        }
    }

    let mut model = Model {
        mu: Vec::new(),
        var: Vec::new(),
        log_w: Vec::new(),
        garb: Gmm {
            log_w: vec![0.0],
            mu: vec![0.0; FEAT_DIM],
            var: vec![1.0; FEAT_DIM],
        },
        cep_mean: Vec::new(),
        ref_llr: 2.0,
        min_cycle_frames: 40,
        max_cycle_frames: 800,
    };

    let mut prev: Option<Scorer> = None;
    for _ in 0..iters.max(1) {
        let (mu, var, lw) = estimate(items, &aligns, prev.as_ref());
        model.mu = mu;
        model.var = var;
        model.log_w = lw;
        let sc = Scorer::new(&model);
        let mut next = Vec::new();
        for (it, old) in items.iter().zip(aligns.iter()) {
            match align_item(it, &sc) {
                Some(a) => next.push(a),
                None => next.push(Alignment {
                    state: old.state.clone(),
                    cycle: old.cycle.clone(),
                }),
            }
        }
        aligns = next;
        prev = Some(sc);
    }
    let (mu, var, lw) = estimate(items, &aligns, prev.as_ref());
    model.mu = mu;
    model.var = var;
    model.log_w = lw;

    // Garbage model: ordinary speech (if provided) + the phrase frames.
    let mut pool: Vec<([f32; FEAT_DIM], f32)> = Vec::new();
    let speech_w = 1.0f32;
    let chant_w = if speech.is_empty() { 1.0 } else { 0.35 };
    for s in speech {
        for f in s.feats.iter().filter(|f| f.active) {
            pool.push((f.x, speech_w * s.weight));
        }
    }
    for (it, al) in items.iter().zip(&aligns) {
        let norm = speed_norm(it, al);
        for (t, st) in al.state.iter().enumerate() {
            if st.is_some() && it.feats[t].active {
                pool.push((it.feats[t].x, chant_w * it.weight * norm));
            }
        }
    }
    if pool.is_empty() {
        return None;
    }
    // Subsample so training stays fast on long takes.
    let stride = (pool.len() / 6000).max(1);
    let pool: Vec<_> = pool.into_iter().step_by(stride).collect();
    let mut garb = fit_gmm(&pool, 6);
    // A touch broader than the data, so real chant clearly beats it.
    for v in garb.var.iter_mut() {
        *v *= 1.15;
    }
    model.garb = garb;

    // Mean cepstrum prior.
    let mut cm = vec![0.0f32; N_CEP];
    let mut tw = 0.0f32;
    for it in items {
        for k in 0..N_CEP {
            cm[k] += it.weight * it.raw_mean[k];
        }
        tw += it.weight;
    }
    for k in 0..N_CEP {
        cm[k] /= tw.max(1e-6);
    }
    model.cep_mean = cm;

    // Reference statistics from the final alignment.
    let sc = Scorer::new(&model);
    let mut all_llr = Vec::new();
    let mut spans_all = Vec::new();
    let mut take_stats = Vec::new();
    for (it, al) in items.iter().zip(&aligns) {
        // per-cycle mean LLR
        let mut sums = vec![0.0f32; it.n_cycles];
        let mut cnts = vec![0u32; it.n_cycles];
        for (t, st) in al.state.iter().enumerate() {
            if let (Some(s), true) = (st, it.feats[t].active) {
                let c = al.cycle[t].min(it.n_cycles - 1);
                let x = &it.feats[t].x;
                sums[c] += sc.chain_ll(*s, x) - sc.gar_ll(x);
                cnts[c] += 1;
            }
        }
        let mut cyc_llr: Vec<f32> = (0..it.n_cycles)
            .filter(|&c| cnts[c] > 0)
            .map(|c| sums[c] / cnts[c] as f32)
            .collect();
        let spans: Vec<f32> = al.cycle_spans(it.n_cycles).iter().map(|&s| s as f32).collect();
        let mut sp = spans.clone();
        take_stats.push(Some(TakeStats {
            cycle_frames: median(&mut sp),
            llr: median(&mut cyc_llr.clone()),
        }));
        all_llr.extend(cyc_llr.drain(..));
        spans_all.extend(spans);
    }
    model.ref_llr = median(&mut all_llr).max(0.5);
    if !spans_all.is_empty() {
        let lo = spans_all.iter().cloned().fold(f32::MAX, f32::min);
        let hi = spans_all.iter().cloned().fold(0.0, f32::max);
        model.min_cycle_frames = ((lo * 0.5) as u32).max(30);
        model.max_cycle_frames = ((hi * 2.5) as u32).clamp(300, 1500);
    }

    Some(Trained { model, take_stats })
}
