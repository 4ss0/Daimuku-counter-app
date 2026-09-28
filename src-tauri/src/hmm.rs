//! Decoding graphs and the offline Viterbi search.
//!
//! Two graphs share the same node types:
//!   - the **loop graph** used for live counting: garbage/silence, then
//!     the phrase, then straight back to the start of the phrase (or to
//!     garbage), forever;
//!   - the **unrolled graph** used for training: exactly `n` phrases in a
//!     row (the user told us how many were recited).

use crate::model::*;

pub const NEG: f32 = -1.0e30;

/// Cost of starting the phrase out of silence/garbage.
pub const ENTER_PEN: f32 = -6.0;
/// Cost of abandoning the phrase halfway (goes to garbage). High on
/// purpose: in slow recitation a long vowel can briefly sound more like
/// "generic chant" than like its syllable, and a cheaper abort let the
/// decoder drop the phrase and restart it mid-word. A phrase that really
/// is abandoned still never counts: `engine::accept` judges the evidence.
pub const ABORT_PEN: f32 = -150.0;
/// Cost of the optional pause between two syllables.
pub const PAUSE_PEN: f32 = -1.5;
/// Cost of switching between silence and garbage.
pub const SWITCH_PEN: f32 = -2.0;
/// Cost of leaving the phrase after "kyo".
pub const EXIT_PEN: f32 = -1.0;

pub struct Graph {
    /// Emission index (into `[f32; N_NODES]`) used by each node.
    pub emit_idx: Vec<usize>,
    /// (predecessor, transition score) for every node.
    pub preds: Vec<Vec<(usize, f32)>>,
    /// Nodes allowed at the first frame, with their entry score.
    pub init: Vec<(usize, f32)>,
    /// Nodes allowed at the last frame.
    pub last: Vec<usize>,
}

fn add_chain_internal(preds: &mut [Vec<(usize, f32)>], chain: impl Fn(usize, usize) -> usize, pause: impl Fn(usize) -> usize) {
    for i in 0..N_SYL {
        for j in 0..K {
            let n = chain(i, j);
            preds[n].push((n, 0.0));
            if j > 0 {
                preds[n].push((chain(i, j - 1), 0.0));
            } else if i > 0 {
                preds[n].push((chain(i - 1, K - 1), 0.0));
                preds[n].push((pause(i - 1), 0.0));
            }
        }
    }
    for i in 0..N_PAUSE {
        let p = pause(i);
        preds[p].push((p, 0.0));
        preds[p].push((chain(i, K - 1), PAUSE_PEN));
    }
}

impl Graph {
    /// Live-counting graph, nodes are the `N_NODES` model nodes.
    pub fn decode_loop() -> Graph {
        let mut preds: Vec<Vec<(usize, f32)>> = vec![Vec::new(); N_NODES];
        let chain = |i: usize, j: usize| i * K + j;
        let pause = |i: usize| NODE_PAUSE0 + i;
        add_chain_internal(&mut preds, chain, pause);

        let kyo_end = chain(N_SYL - 1, K - 1);
        let first = chain(0, 0);
        preds[first].push((kyo_end, 0.0)); // loop: kyo -> Nam without pause
        preds[first].push((NODE_SIL, ENTER_PEN));
        preds[first].push((NODE_GAR, ENTER_PEN));

        preds[NODE_SIL].push((NODE_SIL, 0.0));
        preds[NODE_SIL].push((NODE_GAR, SWITCH_PEN));
        preds[NODE_SIL].push((kyo_end, EXIT_PEN));

        preds[NODE_GAR].push((NODE_GAR, 0.0));
        preds[NODE_GAR].push((NODE_SIL, SWITCH_PEN));
        preds[NODE_GAR].push((kyo_end, EXIT_PEN));
        for s in 0..N_CHAIN {
            if s != kyo_end {
                preds[NODE_GAR].push((s, ABORT_PEN));
            }
        }
        for p in 0..N_PAUSE {
            preds[NODE_GAR].push((NODE_PAUSE0 + p, ABORT_PEN));
        }

        Graph {
            emit_idx: (0..N_NODES).collect(),
            preds,
            init: vec![(NODE_SIL, 0.0), (NODE_GAR, -2.0), (first, ENTER_PEN)],
            last: (0..N_NODES).collect(),
        }
    }

    /// Live-counting graph with a minimum duration: every chain state must
    /// last at least `dur` frames (so a syllable at least `K * dur`).
    /// Node layout: chain state `s`, copy `c` -> `s * dur + c`; then the
    /// pauses, silence and garbage. `emit_idx` maps every node back to its
    /// model node (0..N_NODES), which is also what it *means*.
    pub fn decode_loop_min(dur: usize) -> Graph {
        let dur = dur.max(1);
        let nc = N_CHAIN * dur;
        let pause = |i: usize| nc + i;
        let sil = nc + N_PAUSE;
        let gar = sil + 1;
        let total = gar + 1;
        let node = |s: usize, c: usize| s * dur + c;
        let mut preds: Vec<Vec<(usize, f32)>> = vec![Vec::new(); total];
        let mut emit_idx = vec![0usize; total];
        for s in 0..N_CHAIN {
            for c in 0..dur {
                let n = node(s, c);
                emit_idx[n] = s;
                if c > 0 {
                    preds[n].push((node(s, c - 1), 0.0));
                }
                if c == dur - 1 {
                    preds[n].push((n, 0.0));
                }
                if c == 0 && s > 0 {
                    preds[n].push((node(s - 1, dur - 1), 0.0));
                    if s % K == 0 {
                        // first state of a syllable: may follow a pause
                        preds[n].push((pause(s / K - 1), 0.0));
                    }
                }
            }
        }
        for i in 0..N_PAUSE {
            let p = pause(i);
            emit_idx[p] = NODE_PAUSE0 + i;
            preds[p].push((p, 0.0));
            preds[p].push((node((i + 1) * K - 1, dur - 1), PAUSE_PEN));
        }
        emit_idx[sil] = NODE_SIL;
        emit_idx[gar] = NODE_GAR;
        let kyo_end = node(N_CHAIN - 1, dur - 1);
        let first = node(0, 0);
        preds[first].push((kyo_end, 0.0));
        preds[first].push((sil, ENTER_PEN));
        preds[first].push((gar, ENTER_PEN));
        preds[sil].push((sil, 0.0));
        preds[sil].push((gar, SWITCH_PEN));
        preds[sil].push((kyo_end, EXIT_PEN));
        preds[gar].push((gar, 0.0));
        preds[gar].push((sil, SWITCH_PEN));
        preds[gar].push((kyo_end, EXIT_PEN));
        for n in 0..nc {
            if n != kyo_end {
                preds[gar].push((n, ABORT_PEN));
            }
        }
        for i in 0..N_PAUSE {
            preds[gar].push((pause(i), ABORT_PEN));
        }
        Graph {
            emit_idx,
            preds,
            init: vec![(sil, 0.0), (gar, -2.0), (first, ENTER_PEN)],
            last: (0..total).collect(),
        }
    }

    const BLOCK: usize = N_CHAIN + N_PAUSE + 1;

    /// Node id of chain state `s` (0..N_CHAIN) in cycle `c` of the
    /// unrolled graph.
    pub fn unrolled_chain(c: usize, s: usize) -> usize {
        1 + c * Self::BLOCK + s
    }

    /// Graph for exactly `n` consecutive phrases with optional silence
    /// before, between and after them.
    pub fn unrolled(n: usize) -> Graph {
        let total = 1 + n * Self::BLOCK;
        let mut preds: Vec<Vec<(usize, f32)>> = vec![Vec::new(); total];
        let mut emit_idx = vec![NODE_SIL; total];

        // node 0: leading silence
        preds[0].push((0, 0.0));

        for c in 0..n {
            let base = 1 + c * Self::BLOCK;
            let chain = |i: usize, j: usize| base + i * K + j;
            let pause = |i: usize| base + N_CHAIN + i;
            let sil = base + N_CHAIN + N_PAUSE;
            add_chain_internal(&mut preds, chain, pause);

            for i in 0..N_SYL {
                for j in 0..K {
                    emit_idx[chain(i, j)] = i * K + j;
                }
            }
            for i in 0..N_PAUSE {
                emit_idx[pause(i)] = NODE_PAUSE0 + i;
            }
            emit_idx[sil] = NODE_SIL;

            let first = chain(0, 0);
            let prev_sil = if c == 0 { 0 } else { base - 1 };
            preds[first].push((prev_sil, 0.0));
            if c > 0 {
                let prev_kyo = 1 + (c - 1) * Self::BLOCK + (N_CHAIN - 1);
                preds[first].push((prev_kyo, 0.0));
            }

            preds[sil].push((sil, 0.0));
            preds[sil].push((chain(N_SYL - 1, K - 1), 0.0));
        }

        let last_kyo = Self::unrolled_chain(n - 1, N_CHAIN - 1);
        let last_sil = total - 1;
        Graph {
            emit_idx,
            preds,
            init: vec![(0, 0.0), (Self::unrolled_chain(0, 0), 0.0)],
            last: vec![last_kyo, last_sil],
        }
    }

    pub fn n_nodes(&self) -> usize {
        self.preds.len()
    }
}

/// Full Viterbi over `emit` (one `[f32; N_NODES]` per frame). Returns the
/// best node sequence, or `None` if no path fits (e.g. too few frames).
pub fn viterbi_offline(g: &Graph, emit: &[[f32; N_NODES]]) -> Option<Vec<usize>> {
    let t_len = emit.len();
    let n = g.n_nodes();
    if t_len == 0 {
        return None;
    }
    let mut d = vec![NEG; n];
    let mut nd = vec![NEG; n];
    let mut bp = vec![0u32; t_len * n];

    for &(s, c) in &g.init {
        d[s] = c + emit[0][g.emit_idx[s]];
    }
    for t in 1..t_len {
        let e = &emit[t];
        for s in 0..n {
            let mut best = NEG;
            let mut arg = 0usize;
            for &(p, c) in &g.preds[s] {
                let v = d[p] + c;
                if v > best {
                    best = v;
                    arg = p;
                }
            }
            if best <= NEG / 2.0 {
                nd[s] = NEG;
            } else {
                nd[s] = best + e[g.emit_idx[s]];
            }
            bp[t * n + s] = arg as u32;
        }
        std::mem::swap(&mut d, &mut nd);
        // keep numbers small
        let m = d.iter().cloned().fold(NEG, f32::max);
        if m > NEG / 2.0 {
            for v in d.iter_mut() {
                if *v > NEG / 2.0 {
                    *v -= m;
                }
            }
        }
    }

    let mut best = NEG;
    let mut s = usize::MAX;
    for &l in &g.last {
        if d[l] > best {
            best = d[l];
            s = l;
        }
    }
    if s == usize::MAX || best <= NEG / 2.0 {
        return None;
    }
    let mut path = vec![0usize; t_len];
    path[t_len - 1] = s;
    for t in (1..t_len).rev() {
        s = bp[t * n + s] as usize;
        path[t - 1] = s;
    }
    Some(path)
}

/// Maps an unrolled-graph node to its chain state (0..N_CHAIN), if any.
pub fn unrolled_node_to_chain(node: usize) -> Option<(usize, usize)> {
    if node == 0 {
        return None;
    }
    let idx = node - 1;
    let c = idx / Graph::BLOCK;
    let r = idx % Graph::BLOCK;
    if r < N_CHAIN {
        Some((c, r))
    } else {
        None
    }
}
