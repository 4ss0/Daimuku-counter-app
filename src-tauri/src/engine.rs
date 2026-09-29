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
/// Inside a run of Daimoku (the phrase starts right where the previous
/// counted one ended) the next phrase is almost certainly another Daimoku,
/// so the evidence needed is lower. Still required: every syllable present
/// in order with a plausible length, and a duration that fits the rhythm.
const RUN_JOIN_FRAMES: u64 = 100;
/// A breath between two Daimoku does not end the run.
const RUN_BREATH_FRAMES: u64 = 160;
const RUN_LLR_FACTOR: f32 = 0.10;
const RUN_LLR_MIN: f32 = 0.35;
const RUN_GOOD: f32 = 0.50;
const RUN_WORST: f32 = -3.5;
const RUN_GAP_SHARE: f32 = 0.25;
const RUN_RHYTHM: std::ops::RangeInclusive<f32> = 0.7..=1.45;
/// Up to this many "almost" phrases just before an accepted one are
/// counted too, when they chain into it (typically the first Daimoku of a
/// session or after a breath, heard before the voice/rhythm was known).
const MAX_PENDING: usize = 3;
/// Gap filling: between two counted Daimoku of the same run, voiced time
/// worth `k` phrases (at the rhythm of the run) means `k - 1` were missed.
/// Silences at least `FILL_SILENCE_FRAMES` long (breaths) are not counted
/// as voiced time; one longer than `FILL_MAX_SILENCE` ends the run.
const FILL_SILENCE_FRAMES: u32 = 25;
const FILL_MAX_SILENCE: u32 = 150;
const FILL_MAX_MISSED: u32 = 4;
const FILL_TOLERANCE: f32 = 0.25;
/// Slow Daimoku ending on a held "kyo" the model does not recognise: they
/// may be judged up to "ge" if at least this long (2.5 s) ...
const SLOW_MIN_SPAN: u32 = 250;
/// ... and with this share of the normal minimum evidence.
const SLOW_GE_LLR: f32 = 0.8;
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
    pub ref_llr: f32,
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
            ref_llr: m.ref_llr,
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
    /// Not heard as such but inferred from the rhythm (a Daimoku missed
    /// between two counted ones, or the opening one).
    pub inferred: bool,
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
    /// Plausible phrases not counted yet (see `Verdict::Pending`), in
    /// order, each starting where the previous one ended.
    pending: Vec<PendingCycle>,
    /// Frames between consecutive counted Daimoku of a run.
    recent_gaps: VecDeque<f32>,
    /// Before anything is counted: the last phrase that reached "kyo" but
    /// was refused (often the first Daimoku, heard before the voice and
    /// the rhythm were known).
    first_weak: Option<u64>,
    /// First voiced frame of the session.
    first_voice: Option<u64>,
    /// Last committed frame inside the phrase, and its node.
    last_chain: Option<(u64, usize)>,
}

#[derive(Clone, Copy, Debug)]
struct PendingCycle {
    start: u64,
    end: u64,
    span: u32,
    llr: f32,
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
/// How a phrase that failed `accept` could still count.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Verdict {
    Accept,
    /// Plausible Daimoku with weak evidence: counted only if an accepted
    /// phrase follows right after it.
    Pending,
    Reject,
}

fn run_llr(p: &Params) -> f32 {
    (RUN_LLR_FACTOR * p.ref_llr).max(RUN_LLR_MIN)
}

/// Relaxed test, used inside a run and for pending phrases: all syllables
/// there with a plausible length, some positive evidence, little noise.
fn plausible(c: &Cycle, span: u32, p: &Params) -> bool {
    let llr = if c.act > 0 { c.llr_sum / c.act as f32 } else { -1.0 };
    let mut pos = 0;
    for i in 0..N_SYL - 1 {
        if c.syl_frames[i] < p.min_syl || c.syl_act[i] == 0 {
            return false;
        }
        let m = c.syl_llr[i] / c.syl_act[i] as f32;
        if m < RUN_WORST {
            return false;
        }
        if m > 0.5 {
            pos += 1;
        }
    }
    let good = c.good as f32 / c.act.max(1) as f32;
    let lo_span = (p.cycle_min as f32 * 0.7) as u32;
    c.act >= 20
        && llr >= run_llr(p)
        && good >= RUN_GOOD
        && pos >= 3
        && c.max_gap <= MAX_GAP_FRAMES
        && c.gap_act as f32 <= RUN_GAP_SHARE * (c.act + c.gap_act) as f32
        && span >= lo_span
        && span <= p.cycle_max
}

fn judge(c: &Cycle, span: u32, recent: &VecDeque<f32>, in_run: bool, p: &Params) -> (Verdict, f32) {
    let (ok, llr) = accept(c, span, recent, p);
    if ok {
        return (Verdict::Accept, llr);
    }
    if !plausible(c, span, p) {
        return (Verdict::Reject, llr);
    }
    if in_run {
        if let Some(med) = median_of(recent) {
            if RUN_RHYTHM.contains(&(span as f32 / med)) {
                return (Verdict::Accept, llr);
            }
        }
    }
    (Verdict::Pending, llr)
}

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
            // or a clear phrase at a new tempo (e.g. the three slow
            // Daimoku that close a fast recitation)
            let fits = (0.6..=1.7).contains(&r);
            (fits || (pos >= N_SYL - 2 && worst >= -1.0), llr)
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
        // committed path leaves the phrase right after "ge": maybe a slow
        // Daimoku whose held "kyo" the model does not recognise
        if m == NODE_GAR || m == NODE_SIL {
            if let Some((pu, ps)) = self.ev.last_chain.take() {
                if pu + 1 == u && self.node_model[ps] / K == N_SYL - 2 {
                    self.evaluate_ex(pu, ps, true);
                }
            }
        } else if m < N_CHAIN {
            self.ev.last_chain = Some((u, s));
        } else {
            self.ev.last_chain = None;
        }
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
        self.evaluate_ex(u, s, false);
    }

    /// `ge_end`: the phrase was traced back from "ge" (see `watch_exit`).
    fn evaluate_ex(&mut self, u: u64, s: usize, ge_end: bool) {
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
        if ge_end {
            // slow Daimoku whose long final "kyo" did not match the model:
            // everything up to "ge" must be clearly there, at a slow pace
            let llr = if c.act > 0 { c.llr_sum / c.act as f32 } else { -1.0 };
            if span >= SLOW_MIN_SPAN && plausible(&c, span, &self.params) && llr >= SLOW_GE_LLR * self.params.llr_lo {
                self.ev.pending.clear();
                self.record(u, llr, span);
            }
            return;
        }
        let in_run = self
            .ev
            .last_count_frame
            .is_some_and(|last| c.start <= last + RUN_BREATH_FRAMES);
        let (verdict, llr) = judge(&c, span, &self.ev.recent_spans, in_run, &self.params);
        match verdict {
            Verdict::Accept => {
                // earlier "almost" phrases that chain into this one count too
                let mut confirmed: Vec<PendingCycle> = Vec::new();
                let mut next_start = c.start;
                while let Some(pc) = self.ev.pending.pop() {
                    let joins = pc.end + RUN_JOIN_FRAMES >= next_start && pc.end < next_start + 5;
                    let r = pc.span as f32 / span as f32;
                    if joins && (0.6..=1.6).contains(&r) {
                        next_start = pc.start;
                        confirmed.push(pc);
                    } else {
                        break;
                    }
                }
                self.ev.pending.clear();
                if self.ev.events.is_empty() {
                    let first_start = confirmed.last().map(|p| p.start).unwrap_or(c.start);
                    let first_span = confirmed.last().map(|p| p.span).unwrap_or(span);
                    if let Some(w) = self.first_opening(first_start, first_span) {
                        self.record_event(w, 0.0, 0, true);
                    }
                }
                for pc in confirmed.into_iter().rev() {
                    self.record(pc.end, pc.llr, pc.span);
                }
                self.record(u, llr, span);
            }
            Verdict::Pending => {
                self.ev.rejected += 1;
                let pc = PendingCycle { start: c.start, end: u, span, llr };
                match self.ev.pending.last() {
                    // same phrase judged again (other route or revision)
                    Some(last) if last.start == pc.start => {
                        *self.ev.pending.last_mut().unwrap() = pc;
                    }
                    Some(last) if last.end + RUN_JOIN_FRAMES >= pc.start && last.end < pc.start + 5 => {
                        self.ev.pending.push(pc);
                    }
                    _ => {
                        self.ev.pending.clear();
                        self.ev.pending.push(pc);
                    }
                }
                if self.ev.pending.len() > MAX_PENDING {
                    self.ev.pending.remove(0);
                }
            }
            Verdict::Reject => {
                self.ev.rejected += 1;
                self.ev.pending.clear();
                if self.ev.events.is_empty() {
                    self.ev.first_weak = Some(u);
                }
            }
        }
    }

    /// The very first Daimoku of a session is the hardest (no rhythm yet,
    /// and the voice average is still being learned). If a refused phrase
    /// reached "kyo" right before the first counted one, and the voice
    /// before that first one lasted about one Daimoku, count it too.
    fn first_opening(&self, start: u64, span: u32) -> Option<u64> {
        let w = self.ev.first_weak?;
        let first_voice = self.ev.first_voice?;
        let horizon = (RING - self.params.lag - 2) as u64;
        if w >= start || w + RUN_JOIN_FRAMES < start || start <= first_voice {
            return None;
        }
        // voiced time just before the first counted phrase, back to the
        // last real silence (words said earlier do not matter)
        let mut voiced = 0u32;
        let mut quiet = 0u32;
        let mut t = start;
        while t > first_voice && start - t < horizon {
            t -= 1;
            if self.act[(t as usize) % RING] {
                voiced += 1 + if quiet < FILL_SILENCE_FRAMES { quiet } else { 0 };
                quiet = 0;
            } else {
                quiet += 1;
                if quiet >= FILL_SILENCE_FRAMES && voiced > 0 && t <= w.saturating_sub(span as u64 / 2) {
                    break;
                }
            }
        }
        let r = voiced as f32 / span as f32;
        (0.6..=1.5).contains(&r).then_some(w)
    }

    /// Daimoku that were chanted but not recognised between two counted
    /// ones (see `FILL_*`).
    fn missed_between(&self, prev: u64, u: u64) -> u32 {
        let Some(period) = median_of(&self.ev.recent_gaps) else { return 0 };
        if self.ev.recent_gaps.len() < 3 || u <= prev || u - prev >= (RING - self.params.lag - 2) as u64 {
            return 0;
        }
        let mut voiced = 0u32;
        let mut quiet = 0u32;
        for t in prev + 1..=u {
            if self.act[(t as usize) % RING] {
                if quiet < FILL_SILENCE_FRAMES {
                    voiced += quiet;
                }
                quiet = 0;
                voiced += 1;
            } else {
                quiet += 1;
                if quiet > FILL_MAX_SILENCE {
                    return 0;
                }
            }
        }
        // continuous chanting is voiced most of the time
        if (voiced as f32) < 0.7 * (u - prev) as f32 {
            return 0;
        }
        let k = voiced as f32 / period;
        let n = k.round();
        if n < 2.0 || (k - n).abs() > FILL_TOLERANCE * n.sqrt() {
            return 0;
        }
        let missed = (n as u32) - 1;
        if missed > FILL_MAX_MISSED {
            // too long to be a few unrecognised Daimoku
            return 0;
        }
        missed
    }

    fn record(&mut self, u: u64, llr: f32, span: u32) {
        self.record_event(u, llr, span, false);
    }

    fn record_event(&mut self, u: u64, llr: f32, span: u32, inferred: bool) {
        // only at the run's own tempo (not e.g. the slow closing Daimoku)
        let fits = |x: u32| median_of(&self.ev.recent_spans).is_some_and(|m| (0.7..=1.45).contains(&(x as f32 / m)));
        // both this phrase and the previous one at the run's tempo
        let prev_heard = self.ev.events.last().is_some_and(|e| !e.inferred && fits(e.span));
        let same_tempo = fits(span) && prev_heard;
        if let Some(prev) = self.ev.events.last().map(|e| e.frame) {
            let missed = if same_tempo { self.missed_between(prev, u) } else { 0 };
            for i in 1..=missed {
                let f = prev + (u - prev) * i as u64 / (missed as u64 + 1);
                self.ev.events.push(CountEvent { frame: f, llr: 0.0, span: 0, inferred: true });
            }
            let gap = (u - prev) as f32;
            if missed == 0 && median_of(&self.ev.recent_spans).is_some_and(|m| gap < 1.5 * m) {
                self.ev.recent_gaps.push_back(gap);
                if self.ev.recent_gaps.len() > 7 {
                    self.ev.recent_gaps.pop_front();
                }
            }
        }
        if !inferred {
            self.ev.recent_spans.push_back(span as f32);
            if self.ev.recent_spans.len() > 5 {
                self.ev.recent_spans.pop_front();
            }
        }
        self.ev.last_count_frame = Some(u);
        self.ev.events.push(CountEvent { frame: u, llr, span, inferred });
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
                } else if m / K == N_SYL - 2 {
                    // left the phrase right after "ge" while still voiced:
                    // possibly the held "kyo" of a slow Daimoku
                    let held = (t + 1..self.t).filter(|&x| self.act[(x as usize) % RING]).count() as u64;
                    if held * 3 >= (self.t - t) * 2 {
                        self.evaluate_ex(t, node, true);
                    }
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
            if self.ev.first_voice.is_none() {
                self.ev.first_voice = Some(self.t);
            }
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
        let heard: Vec<f32> = self.ev.events.iter().filter(|x| !x.inferred).map(|x| x.llr).collect();
        if heard.is_empty() {
            0.0
        } else {
            heard.iter().sum::<f32>() / heard.len() as f32
        }
    }

    /// Typical time between Daimoku in ms (median of the recent spacing).
    pub fn period_ms(&self) -> Option<f32> {
        let e = &self.ev.events;
        if e.len() < 2 {
            return e.iter().find(|x| !x.inferred).map(|x| x.span as f32 * HOP_SECS * 1000.0);
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
