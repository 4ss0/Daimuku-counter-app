//! Real-time Daimoku counter: four independent controllers that share a
//! map of the session.
//!
//! - **B, base controller**: streaming Viterbi over the looped phrase
//!   model; a Daimoku is counted as soon as its "kyo" is heard (~0.3 s).
//!   It is strict on its own, but never waits for anything: no state can
//!   keep it from counting the next clear Daimoku.
//! - **F, fast checker**: every 2 s the last 60 s are decoded again with
//!   full hindsight; phrases that belong to a run of at least
//!   `F_MIN_RUN` consecutive, regular Daimoku are confirmed.
//! - **S, slow checker**: the same with the slow model, for Daimoku of
//!   2-9 s (the slow ones that open and close a practice).
//! - **N, noise checker**: inside a steady run, a stretch of voice that
//!   nobody could read (beads rubbed, a bell) is measured against the run's
//!   tempo; the missing Daimoku are counted when the sound there keeps the
//!   rhythm and the shape of the Daimoku already counted around it.
//!
//! Every controller marks what it found on a map of the session at 0.1 s
//! resolution. A Daimoku found by one controller and not by the others is
//! added to the count ("recovered"), never twice.

use crate::features::*;
use crate::hmm::*;
use crate::model::*;
use crate::train::TrainItem;
use std::collections::VecDeque;

/// Frames of history (82 s): the checkers look 60 s back.
const RING: usize = 8192;

/// Every chain state lasts at least this many frames, so a syllable at
/// least `K * MIN_STATE_FRAMES` = 60 ms.
const MIN_STATE_FRAMES: usize = 2;
/// Slow model: a syllable lasts at least 3 x 5 frames = 0.15 s.
const SLOW_STATE_FRAMES: usize = 5;

// --- base controller -------------------------------------------------------

/// A phrase may contain at most this share of voiced non-syllable frames
/// (a held "n", a breath between words), and no single such stretch
/// longer than `MAX_GAP_FRAMES`.
const MAX_GAP_SHARE: f32 = 0.15;
const MAX_GAP_FRAMES: u32 = 90;
/// Inside a run (the phrase starts within this many frames of the last
/// Daimoku counted by any controller; a breath does not end it) a phrase
/// with middling evidence counts if it keeps the run's rhythm. Outside a
/// run only clear phrases count.
const RUN_BREATH_FRAMES: u64 = 160;
/// "kyo" must really be there (a phrase that stops at "renge" is never a
/// Daimoku): voiced frames of it, and its evidence.
const KYO_MIN_ACT: u32 = 5;
const KYO_MIN_LLR: f32 = -1.0;
/// A phrase with middling evidence and nothing counted just before it
/// counts only if perfectly formed: every syllable positive, this share
/// of positive frames, almost no filler, a long enough "kyo".
const ISO_GOOD: f32 = 0.8;
const ISO_GAP_SHARE: f32 = 0.05;
const ISO_KYO: u32 = 8;
/// Frames after leaving the phrase at which it is traced back and judged.
const EXIT_EVAL_FRAMES: u32 = 30;

// --- checkers ---------------------------------------------------------------

/// How often the checkers run, how far back they look, and how old a
/// phrase must be (its end) before they judge it.
const CHECK_EVERY: u64 = 200;
const WINDOW: u64 = 6000;
const SETTLE: u64 = 80;
/// A phrase starting this close to the window's start may be cut: it is
/// left to the previous checks (unless the window starts the session).
const EDGE: u64 = 150;

/// F: run of consecutive phrases (next one starting within
/// `F_CHAIN_GAP` frames of the previous end, durations within
/// `F_RATIO`), each one sane.
const F_MIN_RUN: usize = 4;
const F_CHAIN_GAP: i64 = 40;
const F_RATIO: (f32, f32) = (0.7, 1.43);
const F_MIN_DUR: u64 = 50;
const F_MAX_DUR: u64 = 240;
const F_LLR: f32 = 0.7;
const F_MINSYL: f32 = -6.0;
const F_NPOS: usize = 3;
const F_GAP_SHARE: f32 = 0.25;
const F_MIN_SYL: u32 = 5;
const F_KYO_MIN: u32 = 6;
/// ... or a shorter run (at least 2) of phrases that are each strong.
const F_STRONG_LLR: f32 = 2.0;
const F_STRONG_NPOS: usize = 5;
const F_STRONG_MINSYL: f32 = 0.5;
const F_STRONG_GAP_SHARE: f32 = 0.1;
const F_STRONG_KYO: u32 = 8;
/// ... or a long run that keeps a steady tempo (chanting is as regular as
/// a metronome, noise and talk are not): at least `F_REG_RUN` phrases
/// whose lengths vary by at most `F_REG_CV`, each passing looser checks.
const F_REG_RUN: usize = 5;
const F_REG_CV: f32 = 0.12;
const F_REG_LLR: f32 = 0.3;
const F_REG_NPOS: usize = 2;
const F_REG_MINSYL: f32 = -4.0;
const F_REG_GAP_SHARE: f32 = 0.2;
const F_REG_MIN_SYL: u32 = 4;
const F_REG_KYO_MIN: u32 = 6;

/// S: a slow phrase of `S_MIN_DUR..=S_MAX_DUR` frames with every syllable
/// recognisable, in a group (another slow one within `S_GROUP` frames).
const S_MIN_DUR: u64 = 200;
const S_MAX_DUR: u64 = 900;
const S_NPOS: usize = 4;
const S_MINSYL: f32 = -1.5;
const S_LLR: f32 = 0.5;
const S_GAP_SHARE: f32 = 0.10;
const S_KYO_MIN: u32 = 15;
const S_GROUP: u64 = 1500;
const S_GROUP_RATIO: (f32, f32) = (0.6, 1.6);

/// N: walking from the counted Daimoku through voice nobody could read.
/// Each step places the next phrase (start within `N_OFFSET` of a period,
/// length within `N_DUR`) where the sound envelope best matches the
/// Daimoku of the session; the walk stops at the first step that does not
/// match (`N_CORR`), that is not mostly voiced, or after a pause longer
/// than `N_MAX_BREATH`.
const N_MIN_ANCHORS_FAST: usize = 2;
const N_MIN_ANCHORS_SLOW: usize = 2;
const N_CORR: f32 = 0.6;
/// A weaker match is enough where the checkers' decoding found a phrase
/// ("kyo" reached) over most of the segment, or where loud broadband
/// noise covers the voice (beads, rustle: the voice keeps sounding).
const N_CORR_WEAK: f32 = 0.35;
const N_PATH_OVERLAP: f32 = 0.6;
const N_PATH_NPOS: usize = 3;
const N_PATH_LLR: f32 = 0.3;
const N_NOISE_NATS: f32 = 3.0;
const N_NOISY: f32 = 0.25;
const N_NOISE_VOICED: f32 = 0.45;
/// At most this many weak steps in a row.
const N_MAX_WEAK: u32 = 9;
/// Frames each syllable needs on the decoding for a phrase at the edge of
/// a run to count as complete.
const N_EDGE_SYL: u32 = 3;
const N_OFFSET: f32 = 0.15;
const N_DUR: (f32, f32) = (0.85, 1.15);
const N_ACTIVE: f32 = 0.8;
const N_VOICED: f32 = 0.5;
const N_MAX_BREATH: u64 = 80;
/// A frame is a pause when its low-band level is this far (nats, ~17 dB)
/// below that of the Daimoku (noise can keep the voice detector on);
/// a pause of at least `N_BREATH_MIN` frames right after a phrase is
/// stepped over.
const N_QUIET_NATS: f32 = 4.0;
const N_BREATH_MIN: u64 = 12;
/// Points of the envelope template.
const N_POINTS: usize = 60;
/// Phrases of at least this many frames walk with the slow template.
const N_SLOW_SPAN: u32 = 200;

/// Two phrases are the same Daimoku if they share at least this much of
/// the shorter one.
const SAME_SHARE: f32 = 0.4;
/// Map resolution: frames per slot (0.1 s).
const MAP_RES: u64 = 10;

/// Acceptance thresholds of the base controller, derived from the trained
/// model so they adapt to the user's voice and microphone.
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
    /// "kyo" ends first).
    pub kyo_eval: u32,
    pub cycle_min: u32,
    pub cycle_max: u32,
    pub ref_llr: f32,
}

impl Params {
    pub fn from_model(m: &Model) -> Self {
        Self {
            lag: 25,
            llr_lo: (0.30 * m.ref_llr).clamp(0.6, 1.5),
            llr_hi: (0.60 * m.ref_llr).clamp(1.2, 3.0),
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
// Base controller: phrase evaluation
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
    gap_act: u32,
    max_gap: u32,
    /// The decoder went straight from this phrase's "kyo" into "Nam".
    next_nam: bool,
}

fn has_kyo(c: &Cycle) -> bool {
    let k = N_SYL - 1;
    if c.next_nam {
        // fast chanting: "kyo" runs straight into the next "Nam" and is
        // often clipped; the next phrase starting is proof enough
        return c.syl_act[k] >= 3;
    }
    c.syl_act[k] >= KYO_MIN_ACT && c.syl_llr[k] / c.syl_act[k].max(1) as f32 >= KYO_MIN_LLR
}

fn median_of(v: &VecDeque<f32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    let mut s: Vec<f32> = v.iter().cloned().collect();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(s[s.len() / 2])
}



/// Strong evidence counts on its own (an isolated phrase only with every
/// syllable clearly there); middling evidence only inside a run, fitting
/// its rhythm.
fn accept(c: &Cycle, span: u32, recent: &VecDeque<f32>, p: &Params, in_run: bool) -> (bool, f32) {
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
    let all = (0..N_SYL).all(|i| c.syl_act[i] > 0 && c.syl_llr[i] / c.syl_act[i] as f32 > 0.5);
    if llr >= p.llr_hi {
        if !in_run {
            return (all && worst >= 0.5, llr);
        }
        return (worst >= -3.0, llr);
    }
    if !in_run {
        // middling evidence alone: only a perfectly formed phrase (a single
        // Daimoku said on its own)
        let good = c.good as f32 / c.act.max(1) as f32;
        let tidy = c.gap_act as f32 <= ISO_GAP_SHARE * (c.act + c.gap_act) as f32;
        return (all && worst >= 0.5 && good >= ISO_GOOD && tidy && c.syl_act[N_SYL - 1] >= ISO_KYO, llr);
    }
    let good = c.good as f32 / c.act.max(1) as f32;
    if good < 0.70 || worst < -1.5 {
        return (false, llr);
    }
    match median_of(recent) {
        Some(med) => {
            let r = span as f32 / med;
            let fits = (0.6..=1.7).contains(&r);
            (fits || (pos >= N_SYL - 2 && worst >= -1.0), llr)
        }
        None => (pos >= N_SYL - 2, llr),
    }
}

fn judge(c: &Cycle, span: u32, recent: &VecDeque<f32>, in_run: bool, p: &Params) -> (bool, f32) {
    if !has_kyo(c) {
        let llr = if c.act > 0 { c.llr_sum / c.act as f32 } else { -1.0 };
        return (false, llr);
    }
    accept(c, span, recent, p, in_run)
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Which controller found a Daimoku.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Base,
    Fast,
    Slow,
    Noise,
}

/// One counted Daimoku.
#[derive(Clone, Debug)]
pub struct CountEvent {
    /// Frame of its end ("kyo" reached for the base controller, end of
    /// the phrase for the checkers): the events are handed out in this
    /// order.
    pub frame: u64,
    /// First frame ("Nam").
    pub start: u64,
    /// One past the last frame of the phrase (for the base controller,
    /// which counts as soon as "kyo" is heard, the expected end of "kyo").
    pub end: u64,
    pub llr: f32,
    /// Frames from "Nam" to `frame`.
    pub span: u32,
    /// Found by a checker, not by the base controller.
    pub inferred: bool,
    pub source: Source,
    /// Frame at which it was added to the count.
    pub added_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Warming,
    Locked,
}

#[derive(Clone, Debug)]
pub struct EngineSnapshot {
    pub count: usize,
    /// Of `count`, the Daimoku added by the checkers (F, S, N).
    pub recovered: usize,
    pub state: EngineState,
    pub period_ms: Option<f32>,
}

// ---------------------------------------------------------------------------
// The map
// ---------------------------------------------------------------------------

/// The session at 0.1 s resolution: which counted Daimoku covers each
/// tenth of a second.
#[derive(Default)]
struct TimeMap {
    /// Event id + 1 per slot, 0 = free.
    slots: Vec<u32>,
}

impl TimeMap {
    fn slot(f: u64) -> usize {
        (f / MAP_RES) as usize
    }

    /// Ids of the Daimoku marked anywhere in `t0..t1`.
    fn owners(&self, t0: u64, t1: u64) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        if self.slots.is_empty() {
            return out;
        }
        let (a, b) = (Self::slot(t0), Self::slot(t1.max(t0 + 1) - 1));
        for s in a..=b.min(self.slots.len() - 1) {
            let id = self.slots[s];
            if id != 0 && !out.contains(&(id - 1)) {
                out.push(id - 1);
            }
        }
        out
    }

    fn mark(&mut self, t0: u64, t1: u64, id: u32) {
        let (a, b) = (Self::slot(t0), Self::slot(t1.max(t0 + 1) - 1));
        if self.slots.len() <= b {
            self.slots.resize(b + 1, 0);
        }
        for s in a..=b {
            if self.slots[s] == 0 {
                self.slots[s] = id + 1;
            }
        }
    }
}

/// One phrase found by a controller (each keeps its own list).
#[derive(Clone, Copy, Debug)]
struct Detection {
    t0: u64,
    t1: u64,
}

fn same_daimoku(a: (u64, u64), b: (u64, u64)) -> bool {
    let lo = a.0.max(b.0);
    let hi = a.1.min(b.1);
    if hi <= lo {
        return false;
    }
    let shorter = (a.1 - a.0).min(b.1 - b.0).max(1);
    (hi - lo) as f32 >= SAME_SHARE * shorter as f32
}

// ---------------------------------------------------------------------------
// Window decoding (checkers)
// ---------------------------------------------------------------------------

/// A complete phrase ("Nam" ... "kyo") on the best path of a window.
#[derive(Clone, Debug)]
struct WPhrase {
    t0: u64,
    /// One past the last frame.
    t1: u64,
    sf: [u32; N_SYL],
    sa: [u32; N_SYL],
    sl: [f32; N_SYL],
    gap: u32,
}

impl WPhrase {
    fn dur(&self) -> u64 {
        self.t1 - self.t0
    }
    fn llr(&self) -> f32 {
        let a: u32 = self.sa.iter().sum();
        self.sl.iter().sum::<f32>() / a.max(1) as f32
    }
    fn syl(&self, i: usize) -> f32 {
        if self.sa[i] == 0 {
            -10.0
        } else {
            self.sl[i] / self.sa[i] as f32
        }
    }
    fn minsyl(&self) -> f32 {
        (0..N_SYL).map(|i| self.syl(i)).fold(f32::MAX, f32::min)
    }
    fn npos(&self) -> usize {
        (0..N_SYL).filter(|&i| self.syl(i) > 0.5).count()
    }
    fn gap_share(&self) -> f32 {
        self.gap as f32 / self.dur().max(1) as f32
    }
}

struct WinDecoder {
    graph: Graph,
    nn: usize,
    node_model: Vec<usize>,
    bp: Vec<u8>,
    d: Vec<f32>,
    nd: Vec<f32>,
}

impl WinDecoder {
    fn new(min_dur: usize) -> Self {
        let graph = Graph::decode_loop_min(min_dur);
        let nn = graph.n_nodes();
        let node_model = graph.emit_idx.clone();
        Self { graph, nn, node_model, bp: Vec::new(), d: vec![NEG; nn], nd: vec![NEG; nn] }
    }

    /// Best path over frames `a..=b` (model node per frame).
    fn decode(&mut self, emit: &[[f32; N_NODES]], a: u64, b: u64) -> Vec<usize> {
        let n = (b - a + 1) as usize;
        let nn = self.nn;
        if self.bp.len() < n * nn {
            self.bp.resize(n * nn, 0);
        }
        self.d.iter_mut().for_each(|v| *v = NEG);
        let e0 = &emit[(a as usize) % RING];
        for &(s, c) in &self.graph.init {
            self.d[s] = c + e0[self.node_model[s]];
        }
        for i in 1..n {
            let e = &emit[((a + i as u64) as usize) % RING];
            let base = i * nn;
            for s in 0..nn {
                let mut best = NEG;
                let mut arg = 0usize;
                for &(p, c) in &self.graph.preds[s] {
                    let v = self.d[p] + c;
                    if v > best {
                        best = v;
                        arg = p;
                    }
                }
                self.nd[s] = if best > NEG / 2.0 { best + e[self.node_model[s]] } else { NEG };
                self.bp[base + s] = arg as u8;
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
        let mut s = 0usize;
        let mut best = NEG;
        for q in 0..nn {
            if self.d[q] > best {
                best = self.d[q];
                s = q;
            }
        }
        let mut path = vec![0usize; n];
        for i in (0..n).rev() {
            path[i] = self.node_model[s];
            if i > 0 {
                s = self.bp[i * nn + s] as usize;
            }
        }
        path
    }
}

/// Complete phrases on a decoded path starting at frame `a`.
fn phrases_on_path(path: &[usize], a: u64, emit: &[[f32; N_NODES]], act: &[bool]) -> Vec<WPhrase> {
    let n = path.len();
    let is_nam = |m: usize| m < N_CHAIN && m / K == 0;
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if is_nam(path[i]) && (i == 0 || !is_nam(path[i - 1])) {
            let mut p = WPhrase { t0: a + i as u64, t1: a + i as u64, sf: [0; N_SYL], sa: [0; N_SYL], sl: [0.0; N_SYL], gap: 0 };
            let mut last = 0usize;
            let mut j = i;
            while j < n {
                let m = path[j];
                let t = a + j as u64;
                let idx = (t as usize) % RING;
                if m < N_CHAIN {
                    let s = m / K;
                    if s < last && s == 0 {
                        break;
                    }
                    last = s;
                    p.sf[s] += 1;
                    if act[idx] {
                        p.sa[s] += 1;
                        p.sl[s] += emit[idx][m] - emit[idx][NODE_GAR];
                    }
                } else if (NODE_PAUSE0..NODE_SIL).contains(&m) {
                    if act[idx] {
                        p.gap += 1;
                    }
                } else {
                    break;
                }
                j += 1;
            }
            p.t1 = a + j as u64;
            // quiet frames at either end (a breath taken as part of "kyo")
            while p.t1 > p.t0 + 1 && !act[((p.t1 - 1) as usize) % RING] {
                p.t1 -= 1;
            }
            while p.t0 + 1 < p.t1 && !act[(p.t0 as usize) % RING] {
                p.t0 += 1;
            }
            if p.sf[N_SYL - 1] > 0 {
                out.push(p);
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// One acoustic model with its per-frame scores over the history.
struct Lane {
    scorer: Scorer,
    emit: Vec<[f32; N_NODES]>,
}

impl Lane {
    fn new(m: &Model) -> Self {
        Self { scorer: Scorer::new(m), emit: vec![[0.0; N_NODES]; RING] }
    }
}

/// Base controller state.
#[derive(Default)]
struct Base {
    /// Frame of the last "kyo" counted by this controller: a candidate
    /// starting before it is the same phrase seen again after a revision.
    last_count_frame: Option<u64>,
    kyo_run: u32,
    last_kyo: Option<usize>,
    exit_run: u32,
    next_nam: bool,
    /// The phrase judged a few frames into "kyo" was refused: judge it again
    /// once "kyo" is over (an isolated phrase is only clear with all of it).
    retry: bool,
    rejected: u32,
    aborted: u32,
}

pub struct Engine {
    params: Params,
    fe: FrontEnd,
    cmn: OnlineCmn,
    prior: Vec<f32>,
    raw: Vec<RawFrame>,
    /// Frames seen while the session is still "warming up" (see `rewarm`).
    warm: Option<Vec<RawFrame>>,
    warm_active: usize,

    main: Lane,
    slow: Option<Lane>,
    act: Vec<bool>,
    hf: Vec<f32>,
    voiced: Vec<f32>,
    lf: Vec<f32>,

    // base controller decoder
    graph: Graph,
    nn: usize,
    node_model: Vec<usize>,
    d: Vec<f32>,
    nd: Vec<f32>,
    bp: Vec<u8>,
    next_commit: u64,
    b: Base,

    // checkers
    win_fast: WinDecoder,
    win_slow: WinDecoder,
    /// Phrases on the checkers' last decoding (fast and slow models).
    fast_phrases: Vec<WPhrase>,
    slow_phrases: Vec<WPhrase>,
    det: [Vec<Detection>; 4],

    // shared result
    map: TimeMap,
    events: Vec<CountEvent>,

    t: u64,
    last_active: u64,
    active_frames: u64,
    finished: bool,
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
            params: Params::from_model(model),
            fe: FrontEnd::new(sample_rate),
            cmn: OnlineCmn::new(&model.cep_mean),
            prior: model.cep_mean.clone(),
            raw: Vec::new(),
            warm: Some(Vec::new()),
            warm_active: 0,
            main: Lane::new(model),
            slow: None,
            act: vec![false; RING],
            hf: vec![0.0; RING],
            voiced: vec![0.0; RING],
            lf: vec![0.0; RING],
            graph,
            nn,
            node_model,
            d: vec![NEG; nn],
            nd: vec![NEG; nn],
            bp: vec![0u8; RING * nn],
            next_commit: 0,
            b: Base::default(),
            win_fast: WinDecoder::new(MIN_STATE_FRAMES),
            win_slow: WinDecoder::new(SLOW_STATE_FRAMES),
            fast_phrases: Vec::new(),
            slow_phrases: Vec::new(),
            det: Default::default(),
            map: TimeMap::default(),
            events: Vec::new(),
            t: 0,
            last_active: 0,
            active_frames: 0,
            finished: false,
        }
    }

    /// Adds the slow checker with its model.
    pub fn with_slow(mut self, slow_model: &Model, _sample_rate: u32) -> Self {
        self.slow = Some(Lane::new(slow_model));
        self
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

    /// Ends the stream: decides the frames still inside the look-ahead and
    /// runs the checkers one last time.
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
        if self.next_commit <= end {
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
            // stopped in the middle of the final "kyo"
            let run = self.b.kyo_run;
            if (run >= 1 && run < self.params.kyo_eval) || (run > self.params.kyo_eval && self.b.retry) {
                self.b.retry = false;
                if let Some(k) = self.b.last_kyo {
                    self.evaluate(end, k);
                }
            }
            self.b.kyo_run = 0;
        }
        self.run_checkers(true);
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
    fn rewarm(&mut self) {
        let Some(buf) = self.warm.take() else { return };
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
        self.b = Base::default();
        self.det = Default::default();
        self.map = TimeMap::default();
        self.events.clear();
        self.last_active = 0;
        self.active_frames = 0;
        for f in &buf {
            self.step_inner(f);
        }
    }

    fn step_inner(&mut self, raw: &RawFrame) {
        self.cmn.update(&raw.cep, raw.active);
        let feat = Feat { x: self.cmn.normalise(raw), active: raw.active };
        let idx = (self.t as usize) % RING;
        let mut e = [0.0f32; N_NODES];
        self.main.scorer.emissions(&feat, &mut e);
        self.main.emit[idx] = e;
        if let Some(s) = self.slow.as_mut() {
            let mut se = [0.0f32; N_NODES];
            s.scorer.emissions(&feat, &mut se);
            s.emit[idx] = se;
        }
        self.act[idx] = raw.active;
        self.hf[idx] = raw.hf;
        self.voiced[idx] = raw.voiced;
        self.lf[idx] = raw.lf;
        if raw.active {
            self.last_active = self.t;
            self.active_frames += 1;
        }

        // base controller: one Viterbi step
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
                self.nd[s] = if best > NEG / 2.0 { best + e[self.node_model[s]] } else { NEG };
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
        if self.t > 0 && self.t % CHECK_EVERY == 0 {
            self.run_checkers(false);
        }
        self.t += 1;
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

    // --- base controller ----------------------------------------------------

    /// Frame `u` has been decided to be in node `s`.
    fn commit(&mut self, u: u64, s: usize) {
        let m = self.node_model[s];
        let in_kyo = m < N_CHAIN && m / K == N_SYL - 1;
        if !in_kyo {
            let run = self.b.kyo_run;
            self.b.kyo_run = 0;
            if run > self.params.kyo_eval && self.b.retry && u > 0 {
                self.b.retry = false;
                if let Some(k) = self.b.last_kyo {
                    self.b.next_nam = m < N_CHAIN && m / K == 0;
                    self.evaluate(u - 1, k);
                    self.b.next_nam = false;
                }
            }
            if run >= self.params.kyo_min && run < self.params.kyo_eval && u > 0 {
                if let Some(k) = self.b.last_kyo {
                    self.b.next_nam = m < N_CHAIN && m / K == 0;
                    self.evaluate(u - 1, k);
                    self.b.next_nam = false;
                }
            }
            return;
        }
        self.b.kyo_run += 1;
        self.b.last_kyo = Some(s);
        if self.b.kyo_run == self.params.kyo_eval {
            let before = self.events.len();
            self.evaluate(u, s);
            self.b.retry = self.events.len() == before;
        }
    }

    /// In slow recitation the long final vowel can keep the fixed-lag
    /// decisions undecided: once the best path has left the phrase for
    /// `EXIT_EVAL_FRAMES`, trace it back and judge the phrase it came from.
    fn watch_exit(&mut self) {
        let best = self.best_node();
        let bm = self.node_model[best];
        if bm == NODE_SIL || bm == NODE_GAR {
            self.b.exit_run += 1;
        } else {
            self.b.exit_run = 0;
            return;
        }
        if self.b.exit_run != EXIT_EVAL_FRAMES || self.t < 2 {
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

    /// Traces the phrase ending in (`u`, `s`) back to its "Nam" along the
    /// current best path and decides whether it counts.
    fn evaluate(&mut self, u: u64, s: usize) {
        let mut c = Cycle { next_nam: self.b.next_nam, ..Default::default() };
        let mut node = s;
        let mut t = u;
        let limit = (self.params.cycle_max as u64 + 200).min((RING - self.params.lag - 2) as u64);
        let clean_start;
        let mut gap_run = 0u32;
        loop {
            let idx = (t as usize) % RING;
            let m = self.node_model[node];
            if (NODE_PAUSE0..NODE_SIL).contains(&m) && self.act[idx] {
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
                    let e = &self.main.emit[idx];
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
            if m == 0 && !(pm < N_CHAIN && pm / K == 0) {
                clean_start = true;
                break;
            }
            node = prev;
            t -= 1;
        }
        if !clean_start {
            self.b.aborted += 1;
            return;
        }
        if self.b.last_count_frame.is_some_and(|last| c.start <= last) {
            return;
        }
        let span = (u - c.start + 1) as u32;
        // the run and its rhythm: the Daimoku counted by any controller
        let mut prev: Vec<&CountEvent> = self.events.iter().filter(|e| e.frame < c.start + 30).collect();
        prev.sort_by_key(|e| e.frame);
        let in_run = prev.last().is_some_and(|e| c.start <= e.frame + RUN_BREATH_FRAMES);
        let recent: VecDeque<f32> = prev.iter().rev().take(5).map(|e| e.span as f32).collect();
        let (ok, llr) = judge(&c, span, &recent, in_run, &self.params);
        if !ok {
            self.b.rejected += 1;
            return;
        }
        self.b.last_count_frame = Some(u);
        // "kyo" lasts about a sixth of the phrase; it was heard `kyo_run`
        // frames in
        let kyo_rest = (span / 5).saturating_sub(c.syl_frames[N_SYL - 1]) as u64;
        self.add_at(Source::Base, c.start, u + 1 + kyo_rest, u, llr);
    }

    // --- shared map ---------------------------------------------------------

    /// A controller found the Daimoku `t0..t1`: noted in its own list and,
    /// unless another controller already counted it, added to the count.
    fn add(&mut self, src: Source, t0: u64, t1: u64, llr: f32) -> bool {
        self.add_at(src, t0, t1, t1.saturating_sub(1), llr)
    }

    /// Like `add`, with the frame at which the Daimoku was heard.
    fn add_at(&mut self, src: Source, t0: u64, t1: u64, heard: u64, llr: f32) -> bool {
        let k = src as usize;
        if !self.det[k].iter().rev().take(64).any(|d| same_daimoku((d.t0, d.t1), (t0, t1))) {
            self.det[k].push(Detection { t0, t1 });
        }
        let taken = self.map.owners(t0, t1).iter().any(|&id| {
            let e = &self.events[id as usize];
            same_daimoku((e.start, e.end), (t0, t1))
        });
        if taken {
            return false;
        }
        // ids are indices into `events` (kept in insertion order)
        let id = self.events.len() as u32;
        self.map.mark(t0, t1, id);
        self.events.push(CountEvent {
            frame: heard,
            start: t0,
            end: t1,
            llr,
            span: (t1 - t0) as u32,
            inferred: src != Source::Base,
            source: src,
            added_at: self.t,
        });
        true
    }

    // --- checkers -----------------------------------------------------------

    fn run_checkers(&mut self, at_end: bool) {
        if self.t < 2 {
            return;
        }
        let b = self.t - 1;
        let a = b.saturating_sub(WINDOW - 1);
        let limit = if at_end { b + 1 } else { b.saturating_sub(SETTLE) };
        self.check_fast(a, b, limit);
        self.check_slow(a, b, limit);
        self.check_noise(a, b, limit);
    }

    /// Smoothed low-band envelope over `a..=b` (median of 5 frames).
    fn envelope(&self, a: u64, b: u64) -> Vec<f32> {
        let n = (b - a + 1) as usize;
        let raw: Vec<f32> = (0..n).map(|i| self.lf[((a + i as u64) as usize) % RING]).collect();
        (0..n)
            .map(|i| {
                let lo = i.saturating_sub(2);
                let hi = (i + 3).min(n);
                let mut w: Vec<f32> = raw[lo..hi].to_vec();
                w.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
                w[w.len() / 2]
            })
            .collect()
    }

    fn check_noise(&mut self, a: u64, b: u64, limit: u64) {
        // (start, end, span, source, confirmed): a Daimoku is confirmed when
        // a checker found it (F, S) or when one did after the base
        // controller had counted it; the walk only starts from those
        let confirmed = |e: &CountEvent| match e.source {
            Source::Fast | Source::Slow => true,
            Source::Noise => false,
            Source::Base => self.det[Source::Fast as usize]
                .iter()
                .chain(self.det[Source::Slow as usize].iter())
                .rev()
                .take(400)
                .any(|d| same_daimoku((d.t0, d.t1), (e.start, e.end))),
        };
        let mut evs: Vec<(u64, u64, u32, Source, bool)> = self
            .events
            .iter()
            .filter(|e| e.frame >= a && e.start >= a)
            .map(|e| (e.start, e.end, e.span, e.source, confirmed(e)))
            .collect();
        if evs.len() < 2 {
            return;
        }
        evs.sort_by_key(|e| e.0);
        let env = self.envelope(a, b);
        let at = |f: u64| env[(f.clamp(a, b) - a) as usize];
        // envelope of a segment, resampled and normalised
        let shape = |t0: u64, t1: u64| -> [f32; N_POINTS] {
            let mut v = [0.0f32; N_POINTS];
            let len = (t1 - t0) as f32;
            for (k, x) in v.iter_mut().enumerate() {
                let pos = t0 as f32 + len * k as f32 / (N_POINTS - 1) as f32;
                let f0 = pos.floor() as u64;
                let fr = pos - f0 as f32;
                *x = at(f0) * (1.0 - fr) + at(f0 + 1) * fr;
            }
            let m = v.iter().sum::<f32>() / N_POINTS as f32;
            let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / N_POINTS as f32).sqrt().max(1e-3);
            for x in v.iter_mut() {
                *x = (*x - m) / sd;
            }
            v
        };
        let mut added: Vec<(u64, u64, f32)> = Vec::new();
        for slow in [false, true] {
            let class: Vec<&(u64, u64, u32, Source, bool)> = evs
                .iter()
                .filter(|e| e.4 && (e.2 >= N_SLOW_SPAN) == slow)
                .collect();
            let need = if slow { N_MIN_ANCHORS_SLOW } else { N_MIN_ANCHORS_FAST };
            if class.len() < need {
                continue;
            }
            let mut spans: Vec<u32> = class.iter().map(|e| e.2).collect();
            spans.sort_unstable();
            let p = spans[spans.len() / 2] as f32;
            let typical: Vec<&&(u64, u64, u32, Source, bool)> = class.iter().filter(|e| (0.75..=1.33).contains(&(e.2 as f32 / p))).collect();
            if typical.len() < need {
                continue;
            }
            let mut tmpl = [0.0f32; N_POINTS];
            for e in &typical {
                let s = shape(e.0, e.1);
                for k in 0..N_POINTS {
                    tmpl[k] += s[k];
                }
            }
            let m = tmpl.iter().sum::<f32>() / N_POINTS as f32;
            let sd = (tmpl.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / N_POINTS as f32).sqrt().max(1e-3);
            for x in tmpl.iter_mut() {
                *x = (*x - m) / sd;
            }
            let corr = |t0: u64, t1: u64| -> f32 {
                let s = shape(t0, t1);
                s.iter().zip(&tmpl).map(|(x, y)| x * y).sum::<f32>() / N_POINTS as f32
            };
            let step = ((p * 0.02) as u64).max(1);
            let off = (p * N_OFFSET) as u64;
            let (dlo, dhi) = ((p * N_DUR.0) as u64, (p * N_DUR.1) as u64);
            let min_len = (0.75 * p) as u64;
            // counted (or just added) Daimoku, for the walk's limits
            let mut taken: Vec<(u64, u64)> = evs.iter().map(|e| (e.0, e.1)).collect();
            let shares = |t0: u64, t1: u64| -> (f32, f32) {
                let n = (t1 - t0).max(1) as f32;
                let act = (t0..t1).filter(|&f| self.act[(f as usize) % RING]).count() as f32;
                let vo = (t0..t1).filter(|&f| self.voiced[(f as usize) % RING] > 0.5).count() as f32;
                (act / n, vo / n)
            };
            // noise level of the voice in the confirmed Daimoku
            let mut base_hf: Vec<f32> = typical
                .iter()
                .flat_map(|e| (e.0..e.1).filter(|&f| self.act[(f as usize) % RING]).map(|f| self.hf[(f as usize) % RING]))
                .collect();
            base_hf.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            let hf_base = base_hf.get(base_hf.len() / 2).cloned().unwrap_or(0.0);
            let mut lfs: Vec<f32> = typical.iter().flat_map(|e| (e.0..e.1).map(|f| self.lf[(f as usize) % RING])).collect();
            lfs.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            let lf_ref = lfs.get(lfs.len() / 2).cloned().unwrap_or(0.0);
            let quiet = |f: u64| {
                let ix = (f as usize) % RING;
                !self.act[ix] || self.lf[ix] < lf_ref - N_QUIET_NATS
            };
            let phrases = if slow { &self.slow_phrases } else { &self.fast_phrases };
            let weak_ok = |t0: u64, t1: u64| -> bool {
                let len = (t1 - t0) as f32;
                let on_path = phrases.iter().any(|q| {
                    let ov = (q.t1.min(t1) as f32 - q.t0.max(t0) as f32).max(0.0);
                    ov >= N_PATH_OVERLAP * len && q.npos() >= N_PATH_NPOS && q.llr() >= N_PATH_LLR
                });
                if on_path {
                    return true;
                }
                let (mut noisy, mut nv) = (0u32, 0u32);
                for f in t0..t1 {
                    let ix = (f as usize) % RING;
                    if self.hf[ix] > hf_base + N_NOISE_NATS {
                        noisy += 1;
                        nv += (self.voiced[ix] > 0.7) as u32;
                    }
                }
                noisy as f32 >= N_NOISY * len && nv as f32 >= N_NOISE_VOICED * noisy as f32
            };
            let anchors: Vec<(u64, u64)> = evs
                .iter()
                .filter(|e| (e.4 || e.3 == Source::Noise) && (0.6..=1.6).contains(&(e.2 as f32 / p)))
                .map(|e| (e.0, e.1))
                .collect();
            // a phrase at the edge of a run (followed or preceded by a pause, by
            // the end of the recording or by other sounds) must be complete
            // on the checkers' decoding: every syllable, and a real "kyo" (a
            // Daimoku stopped at "renge" never counts)
            let complete = |t0: u64, t1: u64| -> bool {
                let len = (t1 - t0) as f32;
                phrases.iter().any(|q| {
                    let ov = (q.t1.min(t1) as f32 - q.t0.max(t0) as f32).max(0.0);
                    ov >= N_PATH_OVERLAP * len
                        && (0..N_SYL).all(|k| q.sf[k] >= N_EDGE_SYL)
                        && q.sa[N_SYL - 1] >= KYO_MIN_ACT
                        && q.syl(N_SYL - 1) >= KYO_MIN_LLR
                })
            };
            for &(s0, s1) in &anchors {
                for forward in [true, false] {
                    let mut t = if forward { s1 } else { s0 };
                    let mut weak_run = 0u32;
                    let walk_from = added.len();
                    let known: Vec<(u64, u64)> = taken.clone();
                    loop {
                        // step over a breath, right here or just ahead
                        let ahead = (0.35 * p) as u64;
                        let mut q = t;
                        let mut run = 0u64;
                        let mut jump = None;
                        for k in 0..(ahead + N_MAX_BREATH) {
                            let f = if forward { t + k } else { t.saturating_sub(k) };
                            if f <= a || f >= limit {
                                break;
                            }
                            if quiet(f) {
                                run += 1;
                                if run > N_MAX_BREATH {
                                    break;
                                }
                            } else {
                                if run >= N_BREATH_MIN || (run > 0 && run == k) {
                                    jump = Some(f);
                                    break;
                                }
                                run = 0;
                                if k >= ahead {
                                    break;
                                }
                            }
                            q = f;
                        }
                        let _ = q;
                        if run > N_MAX_BREATH {
                            break;
                        }
                        // after a pause the next phrase must really look like
                        // a Daimoku: pauses are also where talk begins
                        let after_pause = jump.is_some() && run >= N_BREATH_MIN;
                        if let Some(f) = jump {
                            t = f;
                        }
                        // room up to the next counted Daimoku
                        let room = if forward {
                            let next = taken.iter().filter(|x| x.0 >= t).map(|x| x.0).min().unwrap_or(u64::MAX);
                            next.min(limit).saturating_sub(t)
                        } else {
                            let prev = taken.iter().filter(|x| x.1 <= t).map(|x| x.1).max().unwrap_or(0).max(a);
                            t.saturating_sub(prev)
                        };
                        if room < min_len {
                            break;
                        }
                        let mut best = (f32::MIN, 0u64, 0u64);
                        let mut o = 0u64;
                        while o <= 2 * off {
                            let mut d = dlo;
                            while d <= dhi {
                                let (t0, t1) = if forward {
                                    let t0 = (t + o).saturating_sub(off);
                                    (t0, t0 + d)
                                } else {
                                    let t1 = (t + o).saturating_sub(off);
                                    (t1.saturating_sub(d), t1)
                                };
                                let inside = if forward { t1 <= t + room + off / 2 && t1 <= limit } else { t0 + off / 2 >= t.saturating_sub(room) && t0 >= a };
                                if inside && t1 > t0 + 10 && t0 >= a && t1 <= b {
                                    let c = corr(t0, t1);
                                    if c > best.0 {
                                        best = (c, t0, t1);
                                    }
                                }
                                d += step;
                            }
                            o += step;
                        }
                        if best.0 < N_CORR {
                            if after_pause || best.0 < N_CORR_WEAK || weak_run >= N_MAX_WEAK || !weak_ok(best.1, best.2) {
                                break;
                            }
                            weak_run += 1;
                        } else {
                            weak_run = 0;
                        }
                        let (act_sh, vo_sh) = shares(best.1, best.2);
                        if act_sh < N_ACTIVE || vo_sh < N_VOICED {
                            break;
                        }
                        if taken.iter().any(|&x| same_daimoku(x, (best.1, best.2))) {
                            break;
                        }
                        taken.push((best.1, best.2));
                        added.push((best.1, best.2, best.0));
                        t = if forward { best.2 } else { best.1 };
                    }
                    // did the walk end against a counted Daimoku (a gap
                    // filled) or at the edge of the run?
                    if added.len() > walk_from {
                        let reach = (0.5 * p) as u64;
                        let closed = known.iter().any(|x| {
                            if forward {
                                x.0 + reach / 2 >= t && x.0 <= t + reach
                            } else {
                                x.1 <= t + reach / 2 && x.1 + reach >= t
                            }
                        });
                        let (e0, e1, _) = *added.last().unwrap();
                        if forward && !closed && !complete(e0, e1) {
                            added.pop();
                            if let Some(pos) = taken.iter().rposition(|&x| x == (e0, e1)) {
                                taken.remove(pos);
                            }
                        }
                    }
                }
            }
        }
        for (t0, t1, c) in added {
            self.add(Source::Noise, t0, t1, c);
        }
    }

    fn check_fast(&mut self, a: u64, b: u64, limit: u64) {
        let accepted = self.fast_candidates(a, b, limit);
        for (t0, t1, llr) in accepted {
            self.add(Source::Fast, t0, t1, llr);
        }
    }

    /// Phrases the fast checker confirms on its decoding of `a..=b`.
    fn fast_candidates(&mut self, a: u64, b: u64, limit: u64) -> Vec<(u64, u64, f32)> {
        let emit = &self.main.emit;
        let path = self.win_fast.decode(emit, a, b);
        let phs = phrases_on_path(&path, a, emit, &self.act);
        self.fast_phrases = phs.clone();
        let sane: Vec<bool> = phs
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let next_nam = phs.get(i + 1).is_some_and(|q| q.t0 <= p.t1 + 2);
                let kyo_ok = if next_nam { p.sa[N_SYL - 1] >= 3 } else { p.sa[N_SYL - 1] >= F_KYO_MIN };
                let syl_ok = (0..N_SYL - 1).all(|s| p.sf[s] >= F_MIN_SYL);
                (F_MIN_DUR..=F_MAX_DUR).contains(&p.dur())
                    && kyo_ok
                    && syl_ok
                    && p.llr() >= F_LLR
                    && p.minsyl() >= F_MINSYL
                    && p.npos() >= F_NPOS
                    && p.gap_share() <= F_GAP_SHARE
            })
            .collect();
        let linked = |p: &WPhrase, q: &WPhrase| {
            let gap = q.t0 as i64 - p.t1 as i64;
            let r = q.dur() as f32 / p.dur() as f32;
            gap <= F_CHAIN_GAP && r >= F_RATIO.0 && r <= F_RATIO.1
        };
        let mut accepted: Vec<(u64, u64, f32)> = Vec::new();
        let mut i = 0;
        while i < phs.len() {
            if !sane[i] {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < phs.len() && sane[j + 1] && linked(&phs[j], &phs[j + 1]) {
                j += 1;
            }
            let strong = |p: &WPhrase| {
                p.llr() >= F_STRONG_LLR
                    && p.npos() >= F_STRONG_NPOS
                    && p.minsyl() >= F_STRONG_MINSYL
                    && p.gap_share() <= F_STRONG_GAP_SHARE
                    && p.sa[N_SYL - 1] >= F_STRONG_KYO
            };
            let all_strong = j > i && phs[i..=j].iter().all(strong);
            if j + 1 - i >= F_MIN_RUN || all_strong {
                for p in &phs[i..=j] {
                    if p.t1 <= limit && (a == 0 || p.t0 >= a + EDGE) {
                        accepted.push((p.t0, p.t1, p.llr()));
                    }
                }
            }
            i = j + 1;
        }
        // regular runs with looser per-phrase checks
        let loose: Vec<bool> = phs
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let next_nam = phs.get(i + 1).is_some_and(|q| q.t0 <= p.t1 + 2);
                let kyo_ok = if next_nam { p.sa[N_SYL - 1] >= 3 } else { p.sa[N_SYL - 1] >= F_REG_KYO_MIN };
                (F_MIN_DUR..=F_MAX_DUR).contains(&p.dur())
                    && kyo_ok
                    && (0..N_SYL - 1).all(|s| p.sf[s] >= F_REG_MIN_SYL)
                    && p.llr() >= F_REG_LLR
                    && p.minsyl() >= F_REG_MINSYL
                    && p.npos() >= F_REG_NPOS
                    && p.gap_share() <= F_REG_GAP_SHARE
            })
            .collect();
        let mut i = 0;
        while i < phs.len() {
            if !loose[i] {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < phs.len() && loose[j + 1] && linked(&phs[j], &phs[j + 1]) {
                j += 1;
            }
            if j + 1 - i >= F_REG_RUN {
                let d: Vec<f32> = phs[i..=j].iter().map(|p| p.dur() as f32).collect();
                let m = d.iter().sum::<f32>() / d.len() as f32;
                let cv = (d.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / d.len() as f32).sqrt() / m;
                if cv <= F_REG_CV {
                    for p in &phs[i..=j] {
                        if p.t1 <= limit && (a == 0 || p.t0 >= a + EDGE) {
                            accepted.push((p.t0, p.t1, p.llr()));
                        }
                    }
                }
            }
            i = j + 1;
        }
        accepted
    }

    fn check_slow(&mut self, a: u64, b: u64, limit: u64) {
        let Some(lane) = self.slow.as_ref() else { return };
        let path = self.win_slow.decode(&lane.emit, a, b);
        let phs = phrases_on_path(&path, a, &lane.emit, &self.act);
        self.slow_phrases = phs.clone();
        let good: Vec<&WPhrase> = phs
            .iter()
            .filter(|p| {
                (S_MIN_DUR..=S_MAX_DUR).contains(&p.dur())
                    && p.npos() >= S_NPOS
                    && p.minsyl() >= S_MINSYL
                    && p.llr() >= S_LLR
                    && p.gap_share() <= S_GAP_SHARE
                    && p.sa[N_SYL - 1] >= S_KYO_MIN
            })
            .collect();
        let mut accepted: Vec<(u64, u64, f32)> = Vec::new();
        for p in &good {
            if p.t1 > limit || (a > 0 && p.t0 < a + EDGE) {
                continue;
            }
            let similar = |t0: u64, t1: u64| {
                let r = (t1 - t0) as f32 / p.dur() as f32;
                let near = t0.abs_diff(p.t0) <= S_GROUP && t0 != p.t0;
                near && r >= S_GROUP_RATIO.0 && r <= S_GROUP_RATIO.1
            };
            let partner = good.iter().any(|q| similar(q.t0, q.t1))
                || self.events.iter().any(|e| e.span as u64 >= S_MIN_DUR && similar(e.start, e.end));
            if partner {
                accepted.push((p.t0, p.t1, p.llr()));
            }
        }
        for (t0, t1, llr) in accepted {
            self.add(Source::Slow, t0, t1, llr);
        }
    }

    // --- read-out -----------------------------------------------------------

    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// Counted Daimoku in time order.
    pub fn events(&self) -> Vec<CountEvent> {
        let mut v = self.events.clone();
        v.sort_by_key(|e| e.frame);
        v
    }

    /// Daimoku added by the checkers.
    pub fn recovered(&self) -> usize {
        self.events.iter().filter(|e| e.inferred).count()
    }

    /// Daimoku found by each controller (B, F, S, N), counted or not.
    pub fn detections(&self) -> [usize; 4] {
        [self.det[0].len(), self.det[1].len(), self.det[2].len(), self.det[3].len()]
    }

    pub fn aborted(&self) -> u32 {
        self.b.aborted
    }
    pub fn rejected(&self) -> u32 {
        self.b.rejected
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
    pub fn elapsed_secs(&self) -> f32 {
        self.t as f32 * HOP_SECS
    }
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Mean LLR of the Daimoku counted by the base controller (0 if none).
    pub fn mean_llr(&self) -> f32 {
        let heard: Vec<f32> = self.events.iter().filter(|x| !x.inferred).map(|x| x.llr).collect();
        if heard.is_empty() {
            0.0
        } else {
            heard.iter().sum::<f32>() / heard.len() as f32
        }
    }

    /// Typical time between Daimoku in ms (median of the recent spacing).
    pub fn period_ms(&self) -> Option<f32> {
        let mut fr: Vec<u64> = self.events.iter().map(|e| e.frame).collect();
        fr.sort_unstable();
        if fr.len() < 2 {
            return self.events.first().map(|x| x.span as f32 * HOP_SECS * 1000.0);
        }
        let mut gaps: Vec<f32> = fr.windows(2).rev().take(5).map(|w| (w[1] - w[0]) as f32).collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(gaps[gaps.len() / 2] * HOP_SECS * 1000.0)
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let silent_for = self.t.saturating_sub(self.last_active);
        let last = self.events.iter().map(|e| e.frame).max();
        let state = if self.t == 0 || silent_for > 100 {
            EngineState::Idle
        } else if let Some(last) = last {
            let period = self.period_ms().unwrap_or(1500.0) / (HOP_SECS * 1000.0);
            let since = self.t.saturating_sub(last) as f32;
            if since < (period * 2.5).max(300.0) {
                EngineState::Locked
            } else {
                EngineState::Warming
            }
        } else {
            EngineState::Warming
        };
        EngineSnapshot { count: self.count(), recovered: self.recovered(), state, period_ms: self.period_ms() }
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
    count_samples_with(model, None, samples, sample_rate)
}

/// Like [`count_samples`], with the slow checker when a slow model is given.
pub fn count_samples_with(model: &Model, slow: Option<&Model>, samples: &[f32], sample_rate: u32) -> BatchResult {
    let mut e = Engine::new(model, sample_rate);
    if let Some(sm) = slow {
        e = e.with_slow(sm, sample_rate);
    }
    for chunk in samples.chunks(2048) {
        e.push(chunk);
    }
    e.finish();
    BatchResult {
        count: e.count(),
        events: e.events(),
        mean_llr: e.mean_llr(),
        period_ms: e.period_ms(),
        rejected: e.rejected(),
        aborted: e.aborted(),
        frames: e.frames_seen(),
    }
}

/// Model of the slow checker: the slow base clips (at least `SLOW_SECS` per
/// Daimoku) and the user's slow takes; its garbage model also learns the
/// faster chant, so that a long vowel alone does not look like a phrase.
pub fn build_slow_model(base: &[TrainItem], user: &[(TrainItem, f32)]) -> Option<Model> {
    const SLOW_SECS: f32 = 2.5;
    let per_cycle = |it: &TrainItem| it.feats.len() as f32 * HOP_SECS / it.n_cycles.max(1) as f32;
    let mut items: Vec<TrainItem> = base.iter().filter(|b| per_cycle(b) >= SLOW_SECS).cloned().collect();
    items.extend(user.iter().filter(|u| u.1 >= SLOW_SECS).map(|u| u.0.clone()));
    if items.is_empty() {
        items.push(base.iter().max_by_key(|b| b.feats.len() / b.n_cycles.max(1))?.clone());
    }
    let speech: Vec<TrainItem> = base.iter().filter(|b| per_cycle(b) < SLOW_SECS).cloned().collect();
    crate::train::train_model(&items, &speech, None, 8).map(|t| t.model)
}
