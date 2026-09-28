//! Real-time Daimoku counter.
//!
//! Pipeline, all streaming:
//!   samples -> MFCC frames (10 ms) -> online mean normalisation ->
//!   Viterbi over the looped phrase graph -> fixed-lag state decisions ->
//!   cycle evaluation (when the decoder reaches "kyo", the whole phrase
//!   is traced back to its "Nam" and checked: every syllable present in
//!   order, enough acoustic evidence, plausible duration -> +1).
//!
//! The Viterbi search is exact; the only approximation is that a state is
//! *committed* `lag` frames (0.25 s) after it happened. Fixed-lag
//! decisions can be revised later (typical in very slow recitation: a
//! long vowel is briefly taken for the next syllable, or for noise), so
//! the per-frame decisions are only used to notice *that* "kyo" was
//! reached. What the phrase contained is always read from one consistent
//! traceback of the best path, never stitched together from decisions
//! taken at different times.

use crate::features::*;
use crate::hmm::*;
use crate::model::*;
use std::collections::VecDeque;

/// Frames of history kept for traceback. Must exceed the longest
/// accepted Daimoku (`Model::max_cycle_frames` is capped at 1500) plus
/// the look-ahead.
const RING: usize = 2048;

/// Every chain state lasts at least this many frames, so a syllable at
/// least `K * MIN_STATE_FRAMES` = 60 ms (fast chanting stays above 70 ms).
/// Without it the decoder could squeeze "Nam-myo-ho" into 90 ms and
/// "restart" the phrase inside a long vowel of a slow Daimoku.
const MIN_STATE_FRAMES: usize = 2;
/// A phrase may contain at most this share of voiced non-syllable frames
/// (a held "n", a breath between words), and no single such stretch
/// longer than `MAX_GAP_FRAMES`. Tuned so that reshuffled fragments of
/// real Daimoku are still rejected.
const MAX_GAP_SHARE: f32 = 0.15;
const MAX_GAP_FRAMES: u32 = 90;
/// Frames after leaving the phrase at which it is traced back and judged
/// (see `Engine::watch_exit`).
const EXIT_EVAL_FRAMES: u32 = 30;

/// Acceptance thresholds, derived from the trained model so they adapt to
/// the user's voice and microphone.
#[derive(Clone, Debug)]
pub struct Params {
    /// Frames of look-ahead before a state is committed.
    pub lag: usize,
    /// A cycle needs at least this mean log-likelihood ratio (phrase vs
    /// garbage per frame) to count at all.
    pub llr_lo: f32,
    /// Above this the evidence alone is enough; between `llr_lo` and
    /// `llr_hi` the cycle must also fit the rhythm of the previous ones.
    pub llr_hi: f32,
    /// Every syllable before "kyo" must last at least this many frames.
    pub min_syl: u32,
    /// A "kyo" shorter than this is never accepted.
    pub kyo_min: u32,
    /// Frames of "kyo" after which the phrase is judged (earlier if the
    /// "kyo" ends first). Enough to be sure it really is "kyo" and not
    /// the "yo" of a long "myo", yet only 80 ms of extra delay.
    pub kyo_eval: u32,
    pub cycle_min: u32,
    pub cycle_max: u32,
}

impl Params {
    pub fn from_model(m: &Model) -> Self {
        Self {
            lag: 25,
            llr_lo: (0.30 * m.ref_llr).max(0.6),
            llr_hi: (0.60 * m.ref_llr).max(1.2),
            min_syl: 5,
            kyo_min: 3,
            kyo_eval: 8,
            cycle_min: m.min_cycle_frames,
            cycle_max: m.max_cycle_frames,
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle evaluation
// ---------------------------------------------------------------------------

/// Statistics of one candidate phrase, read from a traceback.
#[derive(Clone, Debug, Default)]
struct Cycle {
    start: u64,
    syl_frames: [u32; N_SYL],
    syl_act: [u32; N_SYL],
    syl_llr: [f32; N_SYL],
    act: u32,
    good: u32,
    llr_sum: f32,
    /// Voiced frames spent *between* syllables (a held nasal, a breath).
    gap_act: u32,
    /// Longest such stretch.
    max_gap: u32,
}

/// One accepted Daimoku.
#[derive(Clone, Debug)]
pub struct CountEvent {
    /// Frame at which "kyo" was reached.
    pub frame: u64,
    pub llr: f32,
    /// Frames from "Nam" to "kyo".
    pub span: u32,
}

#[derive(Default)]
struct Counter {
    recent_spans: VecDeque<f32>,
    events: Vec<CountEvent>,
    /// Frame of the last accepted "kyo": a candidate starting before it
    /// is the same phrase seen again after a revision.
    last_count_frame: Option<u64>,
    /// Consecutive committed frames inside "kyo".
    kyo_run: u32,
    /// Node of the last committed "kyo" frame.
    last_kyo: Option<usize>,
    /// Consecutive frames in which the best path is outside the phrase.
    exit_run: u32,
    /// Candidates whose traceback did not reach a clean "Nam" start.
    aborted: u32,
    /// Candidates that reached "kyo" but failed the checks.
    rejected: u32,
}

fn median_of(v: &VecDeque<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    let mut s: Vec<f32> = v.iter().cloned().collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(s[s.len() / 2])
}

/// Decides whether a traced-back phrase is a real Daimoku.
///
/// The rules were tuned on real recordings (slow, medium and fast, with
/// and without personal training) against reversed audio, noise and
/// recordings chopped into 200 ms pieces and shuffled:
/// - fragments almost always contain a syllable squeezed to the decoder's
///   minimum length (3 frames) or one that the acoustics flatly deny;
/// - real phrases have every syllable at least ~7 frames long and at most
///   one syllable mildly below the garbage model.
fn accept(c: &Cycle, span: u32, recent: &VecDeque<f32>, p: &Params) -> (bool, f32) {
    let llr = if c.act > 0 { c.llr_sum / c.act as f32 } else { -1.0 };
    let mut worst = f32::MAX;
    let mut pos = 0;
    for i in 0..N_SYL - 1 {
        if c.syl_frames[i] < p.min_syl || c.syl_act[i] == 0 {
            return (false, llr);
        }
        let m = c.syl_llr[i] / c.syl_act[i] as f32;
        worst = worst.min(m);
        if m > 0.5 {
            pos += 1;
        }
    }
    if c.act < 20 || llr < p.llr_lo {
        return (false, llr);
    }
    if c.max_gap > MAX_GAP_FRAMES || c.gap_act as f32 > MAX_GAP_SHARE * (c.act + c.gap_act) as f32 {
        return (false, llr);
    }
    let lo_span = (p.cycle_min as f32 * 0.7) as u32;
    if span < lo_span || span > p.cycle_max {
        return (false, llr);
    }
    if llr >= p.llr_hi {
        // Strong overall evidence: only refuse if one syllable is
        // clearly something else.
        return (worst >= -3.0, llr);
    }

    // Middle band: the evidence alone is not conclusive.
    let good = c.good as f32 / c.act.max(1) as f32;
    if good < 0.70 || worst < -1.5 {
        return (false, llr);
    }
    match median_of(recent) {
        // Must fit the rhythm of the Daimoku just counted.
        Some(med) => {
            let r = span as f32 / med;
            ((0.6..=1.7).contains(&r), llr)
        }
        // Nothing to compare with yet (first phrase of a session, typical
        // of a new voice or a very slow recitation): most syllables must
        // be clearly positive.
        None => (pos >= N_SYL - 2, llr),
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Warming,
    Locked,
}

#[derive(Clone, Debug)]
pub struct EngineSnapshot {
    pub count: usize,
    pub state: EngineState,
    pub period_ms: Option<f32>,
}

pub struct Engine {
    scorer: Scorer,
    params: Params,
    graph: Graph,
    fe: FrontEnd,
    cmn: OnlineCmn,
    raw: Vec<RawFrame>,

    /// Number of decoder nodes (chain states are split into
    /// `MIN_STATE_FRAMES` copies to impose a minimum duration).
    nn: usize,
    /// Model node (0..N_NODES) represented by each decoder node.
    node_model: Vec<usize>,
    d: Vec<f32>,
    nd: Vec<f32>,
    t: u64,
    /// Back-pointers, `RING x nn`.
    bp: Vec<u8>,
    emit: Vec<[f32; N_NODES]>,
    act: Vec<bool>,
    next_commit: u64,

    ev: Counter,
    last_active: u64,
    active_frames: u64,
    finished: bool,

    /// Channel prior from training (average cepstrum).
    prior: Vec<f32>,
    /// Frames seen so far while the session is still "warming up"; see
    /// [`Engine::rewarm`]. `None` once done.
    warm: Option<Vec<RawFrame>>,
    warm_active: usize,
}

/// Voiced frames (1 s) after which the session's own voice/microphone
/// average is known well enough to re-decode the start with it.
const WARM_ACTIVE: usize = 100;
/// Give up warming if the start of the session is mostly silence.
const WARM_MAX_FRAMES: usize = 3000;
/// Weight of the training prior against the measured average.
const WARM_PRIOR_WEIGHT: f32 = 100.0;

impl Engine {
    pub fn new(model: &Model, sample_rate: u32) -> Self {
        let graph = Graph::decode_loop_min(MIN_STATE_FRAMES);
        let nn = graph.n_nodes();
        debug_assert!(nn <= 256, "back-pointers are stored as u8");
        let node_model = graph.emit_idx.clone();
        Self {
            scorer: Scorer::new(model),
            params: Params::from_model(model),
            graph,
            fe: FrontEnd::new(sample_rate),
            cmn: OnlineCmn::new(&model.cep_mean),
            prior: model.cep_mean.clone(),
            warm: Some(Vec::new()),
            warm_active: 0,
            raw: Vec::new(),
            nn,
            node_model,
            d: vec![NEG; nn],
            nd: vec![NEG; nn],
            t: 0,
            bp: vec![0u8; RING * nn],
            emit: vec![[0.0; N_NODES]; RING],
            act: vec![false; RING],
            next_commit: 0,
            ev: Counter::default(),
            last_active: 0,
            active_frames: 0,
            finished: false,
        }
    }

    pub fn params(&self) -> &Params {
        &self.params
    }

    pub fn push(&mut self, samples: &[f32]) {
        if self.finished {
            return;
        }
        let mut frames = std::mem::take(&mut self.raw);
        frames.clear();
        self.fe.push(samples, &mut frames);
        for f in &frames {
            self.step(f);
        }
        self.raw = frames;
    }

    /// Ends the stream: decides the frames still inside the look-ahead.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        let mut frames = Vec::new();
        self.fe.flush(&mut frames);
        for f in &frames {
            self.step(f);
        }
        self.finished = true;
        if self.t == 0 {
            return;
        }
        let end = self.t - 1;
        if self.next_commit > end {
            return;
        }
        let mut s = self.best_node();
        let n = (end - self.next_commit + 1) as usize;
        let mut states = vec![0usize; n];
        let mut u = end;
        loop {
            states[(u - self.next_commit) as usize] = s;
            if u == self.next_commit {
                break;
            }
            s = self.bp[(u as usize) % RING * self.nn + s] as usize;
            u -= 1;
        }
        let start = self.next_commit;
        for (i, &st) in states.iter().enumerate() {
            self.commit(start + i as u64, st);
        }
        self.next_commit = end + 1;
        // Recording stopped in the middle of the final "kyo" (often right
        // at its start: people press stop while still saying it).
        let run = self.ev.kyo_run;
        if run >= 1 && run < self.params.kyo_eval {
            if let Some(k) = self.ev.last_kyo {
                self.evaluate(end, k);
            }
        }
        self.ev.kyo_run = 0;
    }

    /// Frame `u` has been decided to be in node `s` (with the current
    /// best path running through it).
    fn commit(&mut self, u: u64, s: usize) {
        let m = self.node_model[s];
        let in_kyo = m < N_CHAIN && m / K == N_SYL - 1;
        if !in_kyo {
            // A short "kyo" that is already over: judge it now.
            let run = self.ev.kyo_run;
            self.ev.kyo_run = 0;
            if run >= self.params.kyo_min && run < self.params.kyo_eval && u > 0 {
                if let Some(k) = self.ev.last_kyo {
                    self.evaluate(u - 1, k);
                }
            }
            return;
        }
        self.ev.kyo_run += 1;
        self.ev.last_kyo = Some(s);
        if self.ev.kyo_run == self.params.kyo_eval {
            self.evaluate(u, s);
        }
    }

    /// Traces the phrase ending in (`u`, `s`) back to its "Nam" along the
    /// current best path and decides whether it counts.
    fn evaluate(&mut self, u: u64, s: usize) {
        let mut c = Cycle::default();
        let mut node = s;
        let mut t = u;
        let limit = (RING - self.params.lag - 2) as u64;
        let clean_start;
        let mut gap_run = 0u32;
        loop {
            let idx = (t as usize) % RING;
            let m = self.node_model[node];
            if (NODE_PAUSE0..NODE_SIL).contains(&m) && self.act[idx] {
                // sound that is not a syllable, between two syllables
                c.gap_act += 1;
                gap_run += 1;
                c.max_gap = c.max_gap.max(gap_run);
            } else {
                gap_run = 0;
            }
            if m < N_CHAIN {
                let syl = m / K;
                c.syl_frames[syl] += 1;
                if self.act[idx] {
                    let e = &self.emit[idx];
                    let llr = e[m] - e[NODE_GAR];
                    c.act += 1;
                    c.llr_sum += llr;
                    c.syl_act[syl] += 1;
                    c.syl_llr[syl] += llr;
                    if llr > 0.0 {
                        c.good += 1;
                    }
                }
            }
            c.start = t;
            if t == 0 || u - t >= limit {
                clean_start = t == 0 && m == 0;
                break;
            }
            let prev = self.bp[idx * self.nn + node] as usize;
            let pm = self.node_model[prev];
            let boundary = m == 0 && !(pm < N_CHAIN && pm / K == 0);
            if boundary {
                // Entered "Nam" from silence, noise or the previous "kyo".
                clean_start = true;
                break;
            }
            node = prev;
            t -= 1;
        }
        if !clean_start {
            self.ev.aborted += 1;
            return;
        }
        if let Some(last) = self.ev.last_count_frame {
            if c.start <= last {
                // Same phrase, already counted (seen again after the
                // decoder revised where its "kyo" is).
                return;
            }
        }
        let span = (u - c.start + 1) as u32;
        let (ok, llr) = accept(&c, span, &self.ev.recent_spans, &self.params);
        if ok {
            self.ev.recent_spans.push_back(span as f32);
            if self.ev.recent_spans.len() > 5 {
                self.ev.recent_spans.pop_front();
            }
            self.ev.last_count_frame = Some(u);
            self.ev.events.push(CountEvent { frame: u, llr, span });
        } else {
            self.ev.rejected += 1;
        }
    }

    /// Second route to a count, with more hindsight than the fixed lag.
    /// In slow recitation the long final vowel of "kyo" (or a held "n"
    /// before "ge") keeps the 0.25 s decisions undecided, and "kyo" may
    /// never be committed. Once the best path has left the phrase for
    /// `EXIT_EVAL_FRAMES`, trace it back: if it came out of "kyo", judge
    /// that phrase. Already-counted phrases are recognised and skipped.
    fn watch_exit(&mut self) {
        let best = self.best_node();
        let bm = self.node_model[best];
        if bm == NODE_SIL || bm == NODE_GAR {
            self.ev.exit_run += 1;
        } else {
            self.ev.exit_run = 0;
            return;
        }
        if self.ev.exit_run != EXIT_EVAL_FRAMES || self.t < 2 {
            return;
        }
        let mut node = best;
        let mut t = self.t;
        let limit = self.t.saturating_sub(EXIT_EVAL_FRAMES as u64 + 20);
        while t > limit {
            node = self.bp[(t as usize) % RING * self.nn + node] as usize;
            t -= 1;
            let m = self.node_model[node];
            if m < N_CHAIN {
                if m / K == N_SYL - 1 {
                    self.evaluate(t, node);
                }
                return;
            }
        }
    }

    fn best_node(&self) -> usize {
        let mut best = NEG;
        let mut arg = self.nn - 1;
        for s in 0..self.nn {
            if self.d[s] > best {
                best = self.d[s];
                arg = s;
            }
        }
        arg
    }

    fn step(&mut self, raw: &RawFrame) {
        if let Some(buf) = self.warm.as_mut() {
            buf.push(*raw);
            if raw.active {
                self.warm_active += 1;
            }
            if self.warm_active >= WARM_ACTIVE {
                self.rewarm();
                return;
            }
            if buf.len() > WARM_MAX_FRAMES {
                self.warm = None;
            }
        }
        self.step_inner(raw);
    }

    /// The online mean normalisation starts from the training average,
    /// which can be far from this session's microphone and room. After the
    /// first second of voice, measure the session's own average, blend it
    /// with the prior and decode everything so far again from scratch.
    /// Costs one replay of ~1-2 s of frames, once per session, and happens
    /// before the first Daimoku can have been committed.
    fn rewarm(&mut self) {
        let buf = match self.warm.take() {
            Some(b) => b,
            None => return,
        };
        let mut mean = vec![0.0f32; N_CEP];
        let mut n = 0.0f32;
        for f in buf.iter().filter(|f| f.active) {
            for k in 0..N_CEP {
                mean[k] += f.cep[k];
            }
            n += 1.0;
        }
        let mut prior = vec![0.0f32; N_CEP];
        for k in 0..N_CEP {
            let p = self.prior.get(k).copied().unwrap_or(0.0);
            prior[k] = (WARM_PRIOR_WEIGHT * p + mean[k]) / (WARM_PRIOR_WEIGHT + n);
        }
        self.cmn = OnlineCmn::new(&prior);
        self.d.iter_mut().for_each(|v| *v = NEG);
        self.t = 0;
        self.next_commit = 0;
        self.ev = Counter::default();
        self.last_active = 0;
        self.active_frames = 0;
        for f in &buf {
            self.step_inner(f);
        }
    }

    fn step_inner(&mut self, raw: &RawFrame) {
        self.cmn.update(&raw.cep, raw.active);
        let feat = Feat {
            x: self.cmn.normalise(raw),
            active: raw.active,
        };
        let idx = (self.t as usize) % RING;
        let mut e = [0.0f32; N_NODES];
        self.scorer.emissions(&feat, &mut e);
        self.emit[idx] = e;
        self.act[idx] = raw.active;
        if raw.active {
            self.last_active = self.t;
            self.active_frames += 1;
        }

        let nn = self.nn;
        if self.t == 0 {
            self.d.iter_mut().for_each(|v| *v = NEG);
            for &(s, c) in &self.graph.init {
                self.d[s] = c + e[self.node_model[s]];
            }
        } else {
            let base = idx * nn;
            for s in 0..nn {
                let mut best = NEG;
                let mut a = 0usize;
                for &(p, c) in &self.graph.preds[s] {
                    let v = self.d[p] + c;
                    if v > best {
                        best = v;
                        a = p;
                    }
                }
                self.nd[s] = if best > NEG / 2.0 {
                    best + e[self.node_model[s]]
                } else {
                    NEG
                };
                self.bp[base + s] = a as u8;
            }
            let m = self.nd.iter().cloned().fold(NEG, f32::max);
            if m > NEG / 2.0 {
                for v in self.nd.iter_mut() {
                    if *v > NEG / 2.0 {
                        *v -= m;
                    }
                }
            }
            std::mem::swap(&mut self.d, &mut self.nd);
        }

        self.watch_exit();

        let lag = self.params.lag as u64;
        if self.t >= lag {
            let mut s = self.best_node();
            for k in 0..lag {
                s = self.bp[((self.t - k) as usize) % RING * nn + s] as usize;
            }
            let u = self.t - lag;
            self.commit(u, s);
            self.next_commit = u + 1;
        }
        self.t += 1;
    }

    pub fn count(&self) -> usize {
        self.ev.events.len()
    }

    pub fn events(&self) -> &[CountEvent] {
        &self.ev.events
    }

    pub fn aborted(&self) -> u32 {
        self.ev.aborted
    }
    pub fn rejected(&self) -> u32 {
        self.ev.rejected
    }
    pub fn frames_seen(&self) -> u64 {
        self.t
    }
    pub fn active_frames(&self) -> u64 {
        self.active_frames
    }
    /// True while someone is (or was, within the last 0.3 s) making sound.
    pub fn is_speaking(&self) -> bool {
        self.active_frames > 0 && self.t.saturating_sub(self.last_active) < 30
    }

    /// Seconds of audio processed so far.
    pub fn elapsed_secs(&self) -> f32 {
        self.t as f32 * HOP_SECS
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Mean LLR of the accepted cycles (0 if none).
    pub fn mean_llr(&self) -> f32 {
        let e = &self.ev.events;
        if e.is_empty() {
            0.0
        } else {
            e.iter().map(|x| x.llr).sum::<f32>() / e.len() as f32
        }
    }

    /// Typical time between Daimoku in ms (median of the recent spacing).
    pub fn period_ms(&self) -> Option<f32> {
        let e = &self.ev.events;
        if e.len() < 2 {
            return e.first().map(|x| x.span as f32 * HOP_SECS * 1000.0);
        }
        let mut gaps: Vec<f32> = e
            .windows(2)
            .rev()
            .take(5)
            .map(|w| (w[1].frame - w[0].frame) as f32)
            .collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(gaps[gaps.len() / 2] * HOP_SECS * 1000.0)
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let silent_for = self.t.saturating_sub(self.last_active);
        let state = if self.t == 0 || silent_for > 100 {
            EngineState::Idle
        } else if let Some(last) = self.ev.events.last() {
            let period = self.period_ms().unwrap_or(1500.0) / (HOP_SECS * 1000.0);
            let since = self.t.saturating_sub(last.frame) as f32;
            if since < (period * 2.5).max(300.0) {
                EngineState::Locked
            } else {
                EngineState::Warming
            }
        } else {
            EngineState::Warming
        };
        EngineSnapshot {
            count: self.count(),
            state,
            period_ms: self.period_ms(),
        }
    }
}

/// Result of counting a complete recording.
pub struct BatchResult {
    pub count: usize,
    pub events: Vec<CountEvent>,
    pub mean_llr: f32,
    pub period_ms: Option<f32>,
    pub rejected: u32,
    pub aborted: u32,
    pub frames: u64,
}

pub fn count_samples(model: &Model, samples: &[f32], sample_rate: u32) -> BatchResult {
    let mut e = Engine::new(model, sample_rate);
    for chunk in samples.chunks(2048) {
        e.push(chunk);
    }
    e.finish();
    BatchResult {
        count: e.count(),
        events: e.events().to_vec(),
        mean_llr: e.mean_llr(),
        period_ms: e.period_ms(),
        rejected: e.rejected(),
        aborted: e.aborted(),
        frames: e.frames_seen(),
    }
}
