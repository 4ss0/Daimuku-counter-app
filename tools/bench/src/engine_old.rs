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
/// Syllable-proportion check (see `Engine::evaluate`): only once this
/// many Daimoku of the session have been measured.
const SHAPE_MIN_PHRASES: u32 = 3;
/// Largest accepted difference (sum over syllables of the share of the
/// phrase) from the session's usual proportions.
const SHAPE_MAX: f32 = 0.45;
/// Gap filling: between two counted Daimoku of the same run, voiced time
/// worth `k` phrases (at the rhythm of the run) means `k - 1` were missed.
/// Silences at least `FILL_SILENCE_FRAMES` long (breaths) are not counted
/// as voiced time; one longer than `FILL_MAX_SILENCE` ends the run.
const FILL_SILENCE_FRAMES: u32 = 25;
const FILL_MAX_SILENCE: u32 = 150;
const FILL_MAX_MISSED: u32 = 4;
const FILL_TOLERANCE: f32 = 0.25;
/// Segment review (second pass, see `Engine::review_segment`): a stretch of
/// voice closed by a pause of at least this many quiet frames is analysed
/// again with full hindsight.
const REVIEW_QUIET: u32 = 40;
/// Longest stretch reviewed (must fit in the history ring).
const REVIEW_MAX_FRAMES: u64 = 1800;
/// At most this many Daimoku are added to one stretch.
const REVIEW_MAX_ADD: usize = 3;
/// Cost per voiced frame spent outside a phrase inside a reviewed stretch.
const REVIEW_GAR_PEN: f32 = 0.5;
/// Weight of the rhythm prior (every phrase near the session's usual
/// length) against the acoustic score.
const REVIEW_PRIOR_W: f32 = 4.0;
const REVIEW_PRIOR_SIGMA: f32 = 0.25;
/// The hypothesis with more Daimoku must win by at least this much.
const REVIEW_MARGIN: f32 = 6.0;
/// Every phrase of the winning hypothesis needs this much evidence (share
/// of the model's typical score per frame).
const REVIEW_LLR_FACTOR: f32 = 0.10;
/// "kyo" must really be there: at least this many voiced frames of it
/// (it is judged 8 frames in), with evidence for "kyo" not clearly below
/// that for anything else. A phrase that stops at "renge" is never a
/// Daimoku.
const KYO_MIN_ACT: u32 = 5;
const KYO_MIN_LLR: f32 = -1.0;

/// Long check (see `Engine::verify_window`): how often it runs, how far
/// back it looks (it must fit in the history ring, ~20 s), how old a
/// Daimoku must be before it is judged final, and which gaps are suspect.
const VERIFY_EVERY: u64 = 300;
const VERIFY_WINDOW: u64 = 1950;
const VERIFY_SETTLE: u64 = 80;
const VERIFY_MIN_EVENTS: usize = 6;
const VERIFY_GAP: f32 = 1.8;
/// Slow controller (see `Engine::with_slow`): only its phrases at least
/// this long (2.5 s) are used.
/// Noise bridge (part of the long check): a gap inside a steady run that
/// is covered by loud broadband noise (beads rubbed, rustle) while the voice
/// keeps sounding is filled at the run's tempo. Anchors: this many regular
/// intervals before the gap and after it...
const BRIDGE_ANCHORS_BEFORE: usize = 2;
const BRIDGE_ANCHORS_AFTER: usize = 1;
/// ... each within this share of the tempo;
const BRIDGE_ANCHOR_TOL: f32 = 0.2;
/// the gap must be a whole number of periods (this far at most) and at most
/// this many periods long;
const BRIDGE_FIT: f32 = 0.35;
const BRIDGE_MAX_PERIODS: u32 = 9;
/// voiced all along (share of active frames, share of frames with a pitch,
/// also among the noisy frames alone: noise with no voice under it is not
/// bridged),
const BRIDGE_ACTIVE: f32 = 0.9;
const BRIDGE_VOICED_LEVEL: f32 = 0.7;
const BRIDGE_VOICED: f32 = 0.45;
/// and a good part of it covered by noise: frames whose high/low band ratio
/// is this far (nats) above the one of the surrounding Daimoku.
const BRIDGE_NOISE_NATS: f32 = 3.0;
const BRIDGE_NOISY: f32 = 0.25;

/// Channel-mean protection: a frame is "noise only" when its high/low band
/// ratio is this far (nats) above the voice's usual one and it has no pitch
/// (voicing below this); the mean is not updated while at least this share
/// of the last `NOISE_SUSTAIN_FRAMES` frames were noise only (a consonant or
/// a breath is far shorter).
const NOISE_HF_NATS: f32 = 3.0;
const NOISE_VOICED_MAX: f32 = 0.7;
const NOISE_SUSTAIN_FRAMES: u32 = 30;
const NOISE_SUSTAIN_SHARE: f32 = 0.8;
const HF_BASE_RATE: f32 = 0.005;

const SLOW_CTRL_MIN_SPAN: u32 = 250;
/// ... and only in groups: another slow phrase within 12 s.
const SLOW_GROUP_FRAMES: u64 = 1200;
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
    /// The decoder went straight from this phrase's "kyo" into "Nam".
    next_nam: bool,
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
    /// Frames at which the decoder reached the end of a phrase.
    kyo_seen: VecDeque<u64>,
    /// Set while judging a phrase whose "kyo" led straight into "Nam".
    next_nam: bool,
    /// Typical share of each syllable (before "kyo") in the Daimoku of this
    /// session, and how many phrases it is based on.
    shape: [f32; N_SYL - 1],
    shape_n: u32,
    /// Current stretch of voice (for the segment review) and the quiet
    /// frames after it.
    seg_start: Option<u64>,
    seg_quiet: u32,
    /// Daimoku added by the segment review (for diagnostics).
    reviewed_added: u32,
    /// Long check: last Daimoku already examined, and additions.
    verified_upto: u64,
    verify_added: u32,
    /// Noise bridge: last gap end already examined.
    bridged_upto: u64,
    /// Phrases (start, end) counted on the word of the slow controller.
    slow_added: Vec<(u64, u64)>,
    /// Slow controller phrases waiting for a partner (see `merge_slow`).
    slow_wait: Vec<CountEvent>,
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
fn has_kyo(c: &Cycle) -> bool {
    let k = N_SYL - 1;
    if c.next_nam {
        // fast chanting: "kyo" runs straight into the next "Nam" and is
        // often clipped; the next phrase starting is proof enough
        return c.syl_act[k] >= 3;
    }
    c.syl_act[k] >= KYO_MIN_ACT && c.syl_llr[k] / c.syl_act[k].max(1) as f32 >= KYO_MIN_LLR
}

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

/// Two phrases (start, end) are the same Daimoku if they share at least
/// a third of the shorter one.
fn overlaps(a: (u64, u64), b: (u64, u64)) -> bool {
    let lo = a.0.max(b.0);
    let hi = a.1.min(b.1);
    let shorter = (a.1 - a.0).min(b.1 - b.0).max(1);
    hi > lo && (hi - lo) * 3 >= shorter
}

/// Share of each syllable (before "kyo") in the phrase.
fn syllable_shape(c: &Cycle) -> [f32; N_SYL - 1] {
    let tot: u32 = c.syl_frames[..N_SYL - 1].iter().sum();
    let mut out = [0.0f32; N_SYL - 1];
    for i in 0..N_SYL - 1 {
        out[i] = c.syl_frames[i] as f32 / tot.max(1) as f32;
    }
    out
}

fn shape_distance(a: &[f32; N_SYL - 1], b: &[f32; N_SYL - 1]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
}

fn judge(c: &Cycle, span: u32, recent: &VecDeque<f32>, in_run: bool, p: &Params) -> (Verdict, f32) {
    if !has_kyo(c) {
        let llr = if c.act > 0 { c.llr_sum / c.act as f32 } else { -1.0 };
        return (Verdict::Reject, llr);
    }
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
    /// Per-frame noise and voicing measures (see `RawFrame`).
    hf: Vec<f32>,
    voiced: Vec<f32>,
    /// Usual high/low band ratio of the voice (tracked on voiced frames)
    /// and how many frames it has seen.
    hf_base: f32,
    hf_n: u32,
    /// Last 64 frames: bit set = noise with no voice (see `step_inner`).
    nz_hist: u64,
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

    /// Slow controller: a second engine with a model trained only on slow
    /// Daimoku, fed the same frames (see `with_slow`).
    slow: Option<Box<Engine>>,
    slow_seen: usize,
    /// This engine is the slow controller of another one.
    is_child: bool,
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
            hf: vec![0.0; RING],
            voiced: vec![0.0; RING],
            hf_base: -2.0,
            hf_n: 0,
            nz_hist: 0,
            next_commit: 0,
            ev: Counter::default(),
            last_active: 0,
            active_frames: 0,
            finished: false,
            slow: None,
            slow_seen: 0,
            is_child: false,
        }
    }

    /// Adds the slow controller: a model trained only on slow Daimoku,
    /// run in parallel. Its long phrases (>= 2.5 s) are counted when the
    /// main engine has nothing at that point, never twice.
    pub fn with_slow(mut self, slow_model: &Model, sample_rate: u32) -> Self {
        let mut child = Engine::new(slow_model, sample_rate);
        child.is_child = true;
        self.slow = Some(Box::new(child));
        self
    }

    /// Takes over the slow controller's new long phrases. Slow Daimoku come
    /// in groups (the three at the start or end of a practice), so a phrase
    /// is only used once another slow one (from either engine) lies within
    /// `SLOW_GROUP_FRAMES` at a similar pace; until then it waits.
    fn merge_slow(&mut self) {
        let Some(child) = self.slow.as_ref() else { return };
        let n = child.ev.events.len();
        if n < self.slow_seen {
            // the child restarted (warm-up replay)
            self.slow_seen = 0;
            self.ev.slow_wait.clear();
        }
        let new: Vec<CountEvent> = child.ev.events[self.slow_seen..].to_vec();
        self.slow_seen = n;
        for e in new {
            if e.inferred || e.span < SLOW_CTRL_MIN_SPAN {
                continue;
            }
            let start = e.frame.saturating_sub(e.span as u64);
            let taken = self.ev.events.iter().any(|m| overlaps((m.frame.saturating_sub(m.span.max(30) as u64), m.frame), (start, e.frame)));
            if !taken && !self.ev.slow_wait.iter().any(|w| w.frame == e.frame) {
                self.ev.slow_wait.push(e);
            }
        }
        // confirm waiting phrases that have a slow partner
        let mut keep = Vec::new();
        let waiting = std::mem::take(&mut self.ev.slow_wait);
        for w in &waiting {
            let similar = |f: u64, sp: u32| {
                let near = f.abs_diff(w.frame) <= SLOW_GROUP_FRAMES && f != w.frame;
                let r = sp as f32 / w.span as f32;
                near && (0.7..=1.4).contains(&r)
            };
            let partner = self.ev.events.iter().any(|m| m.span >= SLOW_CTRL_MIN_SPAN && similar(m.frame, m.span))
                || waiting.iter().any(|o| similar(o.frame, o.span));
            if partner {
                let start = w.frame.saturating_sub(w.span as u64);
                let taken = self.ev.events.iter().any(|m| overlaps((m.frame.saturating_sub(m.span.max(30) as u64), m.frame), (start, w.frame)));
                if !taken {
                    let pos = self.ev.events.partition_point(|m| m.frame < w.frame);
                    self.ev.events.insert(pos, CountEvent { frame: w.frame, llr: w.llr, span: w.span, inferred: true });
                    self.ev.slow_added.push((start, w.frame));
                }
            } else if self.t.saturating_sub(w.frame) < 2 * SLOW_GROUP_FRAMES {
                keep.push(w.clone());
            }
        }
        self.ev.slow_wait = keep;
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
        if let Some(child) = self.slow.as_mut() {
            child.finish();
        }
        self.merge_slow();
        if let Some(a) = self.ev.seg_start.take() {
            let b = self.last_active;
            if b > a {
                self.review_segment(a, b);
            }
        }
        self.verify_window(true);
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
                    self.ev.next_nam = m < N_CHAIN && m / K == 0;
                    self.evaluate(u - 1, k);
                    self.ev.next_nam = false;
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
        let mut c = Cycle { next_nam: self.ev.next_nam, ..Default::default() };
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
        // every phrase that really reached "kyo", even if it is then
        // refused, is evidence for gap filling (see `missed_between`)
        if has_kyo(&c) && self.ev.kyo_seen.back().is_none_or(|&b| u > b + 30) {
            self.ev.kyo_seen.push_back(u);
            if self.ev.kyo_seen.len() > 64 {
                self.ev.kyo_seen.pop_front();
            }
        }
        let in_run = self
            .ev
            .last_count_frame
            .is_some_and(|last| c.start <= last + RUN_BREATH_FRAMES);
        let (mut verdict, llr) = judge(&c, span, &self.ev.recent_spans, in_run, &self.params);
        // Extra check on phrases accepted without strong evidence: at the
        // session's usual tempo the syllables must keep their usual
        // proportions (pieces of other words glued together rarely do).
        let shape = syllable_shape(&c);
        let at_tempo = median_of(&self.ev.recent_spans).is_some_and(|m| (0.7..=1.45).contains(&(span as f32 / m)));
        if verdict != Verdict::Reject && llr < self.params.llr_hi && at_tempo && self.ev.shape_n >= SHAPE_MIN_PHRASES {
            let d = shape_distance(&shape, &self.ev.shape);
            if d > SHAPE_MAX {
                verdict = Verdict::Reject;
            }
        }
        if verdict == Verdict::Accept && at_tempo {
            let a = if self.ev.shape_n == 0 { 1.0 } else { 0.2 };
            for i in 0..N_SYL - 1 {
                self.ev.shape[i] += a * (shape[i] - self.ev.shape[i]);
            }
            self.ev.shape_n += 1;
        }
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
                if self.ev.events.is_empty() && has_kyo(&c) {
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
        // each missed Daimoku must have left a trace: the decoder reached
        // the end of a phrase there, even if it then refused it
        let traces = self.ev.kyo_seen.iter().filter(|&&f| f > prev + 30 && f + 30 < u).count() as u32;
        if traces < missed {
            return 0;
        }
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
        let my_start = u.saturating_sub(span.max(30) as u64);
        if self.ev.slow_added.iter().any(|&(st, en)| overlaps((my_start, u), (st, en))) {
            // already counted on the word of the slow controller
            return;
        }
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
        if let Some(child) = self.slow.as_mut() {
            child.step(raw);
            self.merge_slow();
        }
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
        // Sustained noise with no voice in it (beads rubbed while pausing)
        // must not drag the channel mean away from the voice: after it the
        // counter would stay deaf until the mean has recovered (~3 s).
        if raw.active && raw.voiced > NOISE_VOICED_MAX {
            let rate = if self.hf_n < 100 { 0.05 } else { HF_BASE_RATE };
            self.hf_base += rate * (raw.hf - self.hf_base);
            self.hf_n += 1;
        }
        let noise_only = raw.active && raw.hf > self.hf_base + NOISE_HF_NATS && raw.voiced < NOISE_VOICED_MAX;
        self.nz_hist = (self.nz_hist << 1) | noise_only as u64;
        let recent = (self.nz_hist & ((1u64 << NOISE_SUSTAIN_FRAMES) - 1)).count_ones();
        let sustained = recent as f32 >= NOISE_SUSTAIN_SHARE * NOISE_SUSTAIN_FRAMES as f32;
        if !(noise_only && sustained) {
            self.cmn.update(&raw.cep, raw.active);
        }
        let feat = Feat {
            x: self.cmn.normalise(raw),
            active: raw.active,
        };
        let idx = (self.t as usize) % RING;
        let mut e = [0.0f32; N_NODES];
        self.scorer.emissions(&feat, &mut e);
        self.emit[idx] = e;
        self.act[idx] = raw.active;
        self.hf[idx] = raw.hf;
        self.voiced[idx] = raw.voiced;
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
        self.track_segment(raw.active);
        if self.t > 0 && self.t % VERIFY_EVERY == 0 {
            self.verify_window(false);
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
    count_samples_with(model, None, samples, sample_rate)
}

/// Like [`count_samples`], with the slow controller when a slow model is
/// given.
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
        events: e.events().to_vec(),
        mean_llr: e.mean_llr(),
        period_ms: e.period_ms(),
        rejected: e.rejected(),
        aborted: e.aborted(),
        frames: e.frames_seen(),
    }
}


// ---------------------------------------------------------------------------
// Segment review (second pass)
// ---------------------------------------------------------------------------

/// One phrase of a reviewed alignment.
struct AlignedCycle {
    start: u64,
    end: u64,
    llr_sum: f32,
    act: u32,
    /// Voiced frames of "kyo" and their evidence.
    kyo_act: u32,
    kyo_llr: f32,
}

impl Engine {
    /// Follows stretches of voice; when one ends with a pause (a breath),
    /// it is reviewed.
    fn track_segment(&mut self, active: bool) {
        let t = self.t;
        if active {
            if self.ev.seg_start.is_none() {
                self.ev.seg_start = Some(t);
            }
            self.ev.seg_quiet = 0;
            return;
        }
        let Some(a) = self.ev.seg_start else { return };
        self.ev.seg_quiet += 1;
        if self.ev.seg_quiet == REVIEW_QUIET {
            self.ev.seg_start = None;
            let b = t - REVIEW_QUIET as u64;
            self.review_segment(a, b);
        }
    }

    /// Second pass over the stretch of voice `a..=b` (between two
    /// breaths), with full hindsight. Finds how many complete Daimoku best
    /// explain it (acoustics plus the session's rhythm); if that is more
    /// than were counted, the missing ones are added.
    fn review_segment(&mut self, a: u64, b: u64) {
        if self.is_child {
            return;
        }
        let len = b.saturating_sub(a) + 1;
        if len < 60 || len > REVIEW_MAX_FRAMES || self.t.saturating_sub(a) + 2 >= RING as u64 {
            return;
        }
        let Some(span_med) = median_of(&self.ev.recent_spans) else { return };
        if self.ev.recent_gaps.len() < 3 {
            return;
        }
        let counted: Vec<u64> = self.ev.events.iter().map(|e| e.frame).filter(|&f| f >= a && f <= b + 10).collect();
        if counted.is_empty() {
            // not a stretch of Daimoku as far as the counter can tell
            return;
        }
        self.review_range(a, b, &counted, span_med);
    }

    /// Core of both reviews: if `a..=b` is best explained by more complete
    /// Daimoku than the `counted` ones, adds the missing ones.
    fn review_range(&mut self, a: u64, b: u64, counted: &[u64], span_med: f32) -> usize {
        let c = counted.len();
        let kmax = c + REVIEW_MAX_ADD;
        let Some(results) = self.count_alignments(a, b, kmax, span_med) else { return 0 };
        // results[k] = (score with prior, cycles)
        let score_c = results.get(c).and_then(|r| r.as_ref()).map(|r| r.0).unwrap_or(f32::MIN);
        let mut best_k = c;
        let mut best = score_c;
        for (k, r) in results.iter().enumerate().skip(c + 1) {
            if let Some((sc, _)) = r {
                if *sc > best {
                    best = *sc;
                    best_k = k;
                }
            }
        }
        if best_k <= c || best - score_c < REVIEW_MARGIN {
            return 0;
        }
        let cycles = &results[best_k].as_ref().unwrap().1;
        let min_llr = (REVIEW_LLR_FACTOR * self.params.ref_llr).max(0.2);
        for cy in cycles {
            let d = (cy.end - cy.start + 1) as f32 / span_med;
            let kyo_ok = cy.kyo_act >= KYO_MIN_ACT && cy.kyo_llr / cy.kyo_act as f32 >= KYO_MIN_LLR;
            if !kyo_ok || !(0.65..=1.5).contains(&d) || cy.act < 15 || cy.llr_sum / (cy.act as f32) < min_llr {
                return 0;
            }
        }
        // phrases of the alignment not matched by a counted Daimoku
        let tol = (span_med * 0.5) as u64;
        let mut added = Vec::new();
        for cy in cycles {
            let hit = counted.iter().any(|&f| f + tol >= cy.end && f <= cy.end + tol)
                || self.ev.events.iter().any(|e| e.frame + tol >= cy.end && e.frame <= cy.end + tol);
            if !hit {
                added.push(cy.end);
            }
        }
        let n_add = (best_k - c).min(added.len());
        for &f in added.iter().take(n_add) {
            let pos = self.ev.events.partition_point(|e| e.frame < f);
            self.ev.events.insert(pos, CountEvent { frame: f, llr: 0.0, span: 0, inferred: true });
            self.ev.reviewed_added += 1;
        }
        n_add
    }

    /// Long check (every `VERIFY_EVERY` frames, over the last ~20 s): with
    /// the tempo measured on all the Daimoku of the window, every interval
    /// between two counted Daimoku that is too long for the tempo (a stall,
    /// or a miss across a breath) is aligned again.
    fn verify_window(&mut self, at_end: bool) {
        if self.is_child {
            return;
        }
        let lo = self.t.saturating_sub(VERIFY_WINDOW);
        let hi = if at_end { self.t } else { self.t.saturating_sub(VERIFY_SETTLE) };
        let evs: Vec<(u64, u32, bool)> = self
            .ev
            .events
            .iter()
            .filter(|e| e.frame > lo && e.frame <= hi)
            .map(|e| (e.frame, e.span, e.inferred))
            .collect();
        if evs.len() < VERIFY_MIN_EVENTS {
            return;
        }
        // tempo of the window
        let mut gaps: Vec<f32> = evs.windows(2).map(|w| (w[1].0 - w[0].0) as f32).collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let period = gaps[gaps.len() / 2];
        let mut spans: Vec<f32> = evs.iter().filter(|e| !e.2 && e.1 > 0).map(|e| e.1 as f32).collect();
        if spans.len() < 3 {
            return;
        }
        spans.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let span_med = spans[spans.len() / 2];
        for w in evs.windows(2) {
            let (e0, e1) = (w[0].0, w[1].0);
            if e1 <= self.ev.verified_upto {
                continue;
            }
            self.ev.verified_upto = e1;
            if ((e1 - e0) as f32) < VERIFY_GAP * period {
                continue;
            }
            let a = e0 + 5;
            if e1 <= a + 60 || self.t.saturating_sub(a) + 2 >= RING as u64 {
                continue;
            }
            let n = self.review_range(a, e1, &[e1], span_med);
            self.ev.verify_added += n as u32;
        }
        self.bridge_noise(lo, hi, period, at_end);
    }

    /// Noise bridge: see `BRIDGE_*`.
    fn bridge_noise(&mut self, lo: u64, hi: u64, period: f32, at_end: bool) {
        let evs: Vec<u64> = self.ev.events.iter().map(|e| e.frame).filter(|&f| f > lo && f <= hi).collect();
        let (nb, na) = (BRIDGE_ANCHORS_BEFORE, BRIDGE_ANCHORS_AFTER);
        if evs.len() < nb + na + 2 || period < 50.0 {
            return;
        }
        let regular = |g: u64| ((g as f32 / period) - 1.0).abs() <= BRIDGE_ANCHOR_TOL;
        for i in nb..evs.len() - 1 {
            let (e0, e1) = (evs[i], evs[i + 1]);
            if e1 <= self.ev.bridged_upto || ((e1 - e0) as f32) < VERIFY_GAP * period {
                continue;
            }
            if i + 1 + na >= evs.len() {
                // anchors after the gap not heard yet
                if at_end {
                    self.ev.bridged_upto = e1;
                }
                break;
            }
            self.ev.bridged_upto = e1;
            if self.t.saturating_sub(evs[i - nb]) + 2 >= RING as u64 {
                continue;
            }
            let before = (i - nb..i).all(|j| regular(evs[j + 1] - evs[j]));
            let after = (i + 1..i + 1 + na).all(|j| regular(evs[j + 1] - evs[j]));
            if !before || !after {
                continue;
            }
            let x = (e1 - e0) as f32 / period;
            let k = x.round();
            if k < 2.0 || k > BRIDGE_MAX_PERIODS as f32 || (x - k).abs() > BRIDGE_FIT {
                continue;
            }
            // noise level of the surrounding Daimoku
            let mut base: Vec<f32> = (evs[i - nb]..=e0)
                .chain(e1..=evs[i + 1 + na])
                .map(|t| (t as usize) % RING)
                .filter(|&ix| self.act[ix])
                .map(|ix| self.hf[ix])
                .collect();
            if base.len() < 50 {
                continue;
            }
            base.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let base = base[base.len() / 2];
            let n = (e1 - e0 - 1) as f32;
            let (mut act, mut voiced, mut noisy, mut noisy_voiced) = (0u32, 0u32, 0u32, 0u32);
            for t in e0 + 1..e1 {
                let ix = (t as usize) % RING;
                let v = self.voiced[ix] > BRIDGE_VOICED_LEVEL;
                act += self.act[ix] as u32;
                voiced += v as u32;
                if self.hf[ix] > base + BRIDGE_NOISE_NATS {
                    noisy += 1;
                    noisy_voiced += v as u32;
                }
            }
            if (act as f32) < BRIDGE_ACTIVE * n
                || (voiced as f32) < BRIDGE_VOICED * n
                || (noisy as f32) < BRIDGE_NOISY * n
                || (noisy_voiced as f32) < BRIDGE_VOICED * noisy as f32
            {
                continue;
            }
            let k = k as u64;
            for j in 1..k {
                let f = e0 + (e1 - e0) * j / k;
                let pos = self.ev.events.partition_point(|e| e.frame < f);
                self.ev.events.insert(pos, CountEvent { frame: f, llr: 0.0, span: 0, inferred: true });
                self.ev.verify_added += 1;
            }
        }
    }

    /// Viterbi over frames `a..=b` that also counts phrases: for every
    /// number of complete Daimoku `k <= kmax`, the best alignment's score
    /// (plus the rhythm prior) and its phrases.
    #[allow(clippy::type_complexity)]
    fn count_alignments(&self, a: u64, b: u64, kmax: usize, span_med: f32) -> Option<Vec<Option<(f32, Vec<AlignedCycle>)>>> {
        let nn = self.nn;
        let nk = kmax + 1;
        let first = 0usize;
        let kyo_end = (0..nn).filter(|&n| self.node_model[n] == N_CHAIN - 1).max()?;
        let sil = (0..nn).find(|&n| self.node_model[n] == NODE_SIL)?;
        let gar = (0..nn).find(|&n| self.node_model[n] == NODE_GAR)?;
        let is_chain = |n: usize| self.node_model[n] < N_CHAIN;
        let is_pause = |n: usize| (NODE_PAUSE0..NODE_SIL).contains(&self.node_model[n]);
        let frames = (b - a + 1) as usize;
        let emit = |t: u64, n: usize| -> f32 {
            let idx = (t as usize) % RING;
            let m = self.node_model[n];
            let mut v = self.emit[idx][m];
            if (m == NODE_GAR || m == NODE_SIL) && self.act[idx] {
                v -= REVIEW_GAR_PEN;
            }
            v
        };
        // back-pointers: predecessor node, and whether k was incremented
        let mut bp: Vec<u16> = vec![0; frames * nk * nn];
        let mut d = vec![NEG; nk * nn];
        let mut nd = vec![NEG; nk * nn];
        for &(n, cst) in &self.graph.init {
            let k = if n == first { 1 } else { 0 };
            if k < nk {
                d[k * nn + n] = cst + emit(a, n);
            }
        }
        for (i, t) in (a + 1..=b).enumerate() {
            let fi = i + 1;
            for v in nd.iter_mut() {
                *v = NEG;
            }
            for n in 0..nn {
                let e = emit(t, n);
                for &(p, cst) in &self.graph.preds[n] {
                    // no leaving a phrase half-way inside a reviewed stretch
                    if n == gar && (is_pause(p) || (is_chain(p) && p != kyo_end)) {
                        continue;
                    }
                    let inc = n == first;
                    for k in 0..nk {
                        let kp = if inc {
                            if k == 0 {
                                continue;
                            }
                            k - 1
                        } else {
                            k
                        };
                        let v = d[kp * nn + p];
                        if v <= NEG / 2.0 {
                            continue;
                        }
                        let v = v + cst + e;
                        if v > nd[k * nn + n] {
                            nd[k * nn + n] = v;
                            bp[(fi * nk + k) * nn + n] = p as u16 | if inc { 1 << 15 } else { 0 };
                        }
                    }
                }
            }
            // keep numbers small
            let m = nd.iter().cloned().fold(NEG, f32::max);
            if m <= NEG / 2.0 {
                return None;
            }
            std::mem::swap(&mut d, &mut nd);
        }
        let _ = sil;
        let mut out = Vec::with_capacity(nk);
        for k in 0..nk {
            // end: after "kyo", or still inside it
            let mut best = NEG;
            let mut arg = usize::MAX;
            for n in 0..nn {
                let ok = n == kyo_end || n == sil || n == gar || (is_chain(n) && self.node_model[n] / K == N_SYL - 1);
                if ok && k > 0 && d[k * nn + n] > best {
                    best = d[k * nn + n];
                    arg = n;
                }
            }
            if arg == usize::MAX || best <= NEG / 2.0 {
                out.push(None);
                continue;
            }
            // trace back, collecting the phrases
            let mut cycles: Vec<AlignedCycle> = Vec::new();
            let mut cur_end: Option<u64> = None;
            let mut llr = 0.0f32;
            let mut act = 0u32;
            let mut kyo_act = 0u32;
            let mut kyo_llr = 0.0f32;
            let mut n = arg;
            let mut kk = k;
            let mut fi = frames - 1;
            loop {
                let t = a + fi as u64;
                let m = self.node_model[n];
                let idx = (t as usize) % RING;
                if m < N_CHAIN {
                    if cur_end.is_none() {
                        cur_end = Some(t);
                    }
                    if self.act[idx] {
                        let v = self.emit[idx][m] - self.emit[idx][NODE_GAR];
                        llr += v;
                        act += 1;
                        if m / K == N_SYL - 1 {
                            kyo_llr += v;
                            kyo_act += 1;
                        }
                    }
                }
                let entered = fi == 0 || (bp[(fi * nk + kk) * nn + n] >> 15) == 1;
                if n == first && entered {
                    if let Some(end) = cur_end.take() {
                        cycles.push(AlignedCycle { start: t, end, llr_sum: llr, act, kyo_act, kyo_llr });
                    }
                    llr = 0.0;
                    act = 0;
                    kyo_act = 0;
                    kyo_llr = 0.0;
                }
                if fi == 0 {
                    break;
                }
                let v = bp[(fi * nk + kk) * nn + n];
                if (v >> 15) == 1 {
                    kk -= 1;
                }
                n = (v & 0x7fff) as usize;
                fi -= 1;
            }
            cycles.reverse();
            if cycles.len() != k {
                out.push(None);
                continue;
            }
            let prior: f32 = cycles
                .iter()
                .map(|c| {
                    let r = ((c.end - c.start + 1) as f32 / span_med).ln() / REVIEW_PRIOR_SIGMA;
                    -0.5 * r * r
                })
                .sum();
            out.push(Some((best + REVIEW_PRIOR_W * prior, cycles)));
        }
        Some(out)
    }

    /// Daimoku added so far by the segment review, the long check and
    /// the slow controller (diagnostics).
    pub fn added_by_checks(&self) -> (u32, u32, usize) {
        (self.ev.reviewed_added, self.ev.verify_added, self.ev.slow_added.len())
    }
}

/// Slow-controller model (legacy builder): slowest base clip plus the
/// user's slow takes.
pub fn build_slow_model(base: &[crate::train::TrainItem], user: &[(crate::train::TrainItem, f32)]) -> Option<Model> {
    let mut items = vec![base.iter().max_by_key(|b| b.feats.len() / b.n_cycles.max(1))?.clone()];
    items.extend(user.iter().filter(|u| u.1 >= 2.5).map(|u| u.0.clone()));
    crate::train::train_model(&items, &[], None, 8).map(|t| t.model)
}
