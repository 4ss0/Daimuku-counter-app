// Phrase features from the full-hindsight Viterbi path, for designing the
// checkers' acceptance rules.
// usage: phrfeat spec... > out.csv
//   spec = path.wav[:rev|:shN.SEED]   (transformed inputs are negatives)
// env PROF="take.wav:n ..." adds user takes (weight 2.5); SLOWM=1 slow model
use daimoku_bench::base::*;
use daimoku_bench::features::*;
use daimoku_bench::hmm::*;
use daimoku_bench::model::*;
use daimoku_bench::train::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
fn shuffled(x: &[f32], chunk: usize, seed: u64) -> Vec<f32> {
    let mut c: Vec<&[f32]> = x.chunks(chunk).collect();
    let mut r = Rng(seed);
    for k in (1..c.len()).rev() {
        let j = (r.next() as usize) % (k + 1);
        c.swap(k, j);
    }
    c.concat()
}

fn load_truth() -> Vec<(String, f32, f32)> {
    let s = std::fs::read_to_string("truth/intervals.txt").unwrap_or_default();
    s.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let v: Vec<&str> = l.split_whitespace().collect();
            (v[0].to_string(), v[1].parse().unwrap(), v[2].parse().unwrap())
        })
        .collect()
}

fn main() {
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data")).unwrap();
    let a: Vec<String> = std::env::args().collect();
    let slow = std::env::var("SLOWM").is_ok();
    let mut items: Vec<TrainItem> = Vec::new();
    if slow {
        let (x, sr) = decode_wav(BASE_CLIPS[0].wav).unwrap();
        items.push(build_item(&x, sr, 1, 1.0).unwrap());
    } else {
        for i in 0..BASE_CLIPS.len() {
            let (x, sr) = decode_wav(BASE_CLIPS[i].wav).unwrap();
            items.push(build_item(&x, sr, BASE_CLIPS[i].n_cycles, 1.0).unwrap());
        }
    }
    if let Ok(p) = std::env::var("PROF") {
        for s in p.split_whitespace() {
            let (f, n) = s.split_once(':').unwrap();
            let n: usize = n.parse().unwrap();
            let (x, sr) = decode_wav(&std::fs::read(f).unwrap()).unwrap();
            let secs = x.len() as f32 / sr as f32;
            if slow && secs / (n as f32) < 2.5 {
                continue;
            }
            items.push(build_item(&x, sr, n, 2.5).unwrap());
        }
    }
    let m = train_model(&items, &[], None, 8).unwrap().model;
    let sc = Scorer::new(&m);
    let truth = load_truth();
    println!("src,label,t0,t1,dur,llr,minsyl,npos,kyof,kyollr,gap,actsh,ordc,disc_min,disc_mean,prevgap,nextgap,prevr,nextr,run,hf,voiced,s0,s1,s2,s3,s4,s5,f0,f1,f2,f3,f4,f5");
    for spec in &a[1..] {
        let (path, tr) = match spec.split_once(':') {
            Some((p, t)) => (p.to_string(), Some(t.to_string())),
            None => (spec.clone(), None),
        };
        let (mut x, sr) = decode_wav(&std::fs::read(&path).unwrap()).unwrap();
        if let Some(t) = &tr {
            if t == "rev" {
                x.reverse();
            } else if let Some(rest) = t.strip_prefix("sh") {
                let (ms, seed) = rest.split_once('.').unwrap();
                let ms: usize = ms.parse().unwrap();
                x = shuffled(&x, sr as usize * ms / 1000, seed.parse().unwrap());
            }
        }
        let base = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
        let src = match &tr {
            Some(t) => format!("{base}.{t}"),
            None => base.clone(),
        };
        let my_truth: Vec<(f32, f32)> = if tr.is_none() {
            truth.iter().filter(|t| t.0 == base).map(|t| (t.1, t.2)).collect()
        } else {
            Vec::new()
        };
        let has_truth = !my_truth.is_empty();
        let frames = extract_frames(&x, sr);
        let mut cmn = OnlineCmn::new(&m.cep_mean);
        let n = frames.len();
        let mut emit = vec![[0.0f32; N_NODES]; n];
        let mut act = vec![false; n];
        for (t, f) in frames.iter().enumerate() {
            cmn.update(&f.cep, f.active);
            let feat = Feat { x: cmn.normalise(f), active: f.active };
            sc.emissions(&feat, &mut emit[t]);
            act[t] = f.active;
        }
        // session noise baseline: median hf over voiced active frames
        let mut hfs: Vec<f32> = frames.iter().filter(|f| f.active && f.voiced > 0.7).map(|f| f.hf).collect();
        hfs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let hf_base = if hfs.is_empty() { 0.0 } else { hfs[hfs.len() / 2] };
        let g = Graph::decode_loop_min(2);
        let Some(path_n) = viterbi_offline(&g, &emit) else { continue };
        let nm: Vec<usize> = path_n.iter().map(|&q| g.emit_idx[q]).collect();
        // collect phrases
        struct Ph {
            t0: usize,
            t1: usize,
            sf: [u32; N_SYL],
            sa: [u32; N_SYL],
            sl: [f32; N_SYL],
            gap: u32,
            ordc: f32,
            disc: [f32; N_SYL],
        }
        let mut phs: Vec<Ph> = Vec::new();
        let mut t = 0;
        while t < n {
            let m0 = nm[t];
            if m0 < N_CHAIN && m0 / K == 0 && (t == 0 || !(nm[t - 1] < N_CHAIN && nm[t - 1] / K == 0)) {
                let start = t;
                let mut p = Ph { t0: start, t1: start, sf: [0; N_SYL], sa: [0; N_SYL], sl: [0.0; N_SYL], gap: 0, ordc: 0.0, disc: [0.0; N_SYL] };
                let mut last_syl = 0usize;
                let mut u = t;
                while u < n {
                    let mm = nm[u];
                    if mm < N_CHAIN {
                        let s = mm / K;
                        if s < last_syl && s == 0 {
                            break;
                        }
                        last_syl = s;
                        p.sf[s] += 1;
                        if act[u] {
                            let e = &emit[u];
                            p.sa[s] += 1;
                            p.sl[s] += e[mm] - e[NODE_GAR];
                            let best_all = (0..N_CHAIN).map(|q| e[q]).fold(f32::MIN, f32::max);
                            p.ordc += e[mm] - best_all;
                            let own = (s * K..(s + 1) * K).map(|q| e[q]).fold(f32::MIN, f32::max);
                            let other = (0..N_CHAIN).filter(|q| q / K != s).map(|q| e[q]).fold(f32::MIN, f32::max);
                            p.disc[s] += (own - other).clamp(-10.0, 10.0);
                        }
                    } else if (NODE_PAUSE0..NODE_SIL).contains(&mm) {
                        if act[u] {
                            p.gap += 1;
                        }
                    } else {
                        break;
                    }
                    u += 1;
                }
                p.t1 = u;
                if p.sf[N_SYL - 1] > 0 {
                    phs.push(p);
                }
                t = u.max(t + 1);
            } else {
                t += 1;
            }
        }
        // run lengths: chains of adjacent phrases with similar durations
        let durs: Vec<f32> = phs.iter().map(|p| (p.t1 - p.t0) as f32).collect();
        let adj = |i: usize, j: usize| -> bool {
            let gap = phs[j].t0 as i64 - phs[i].t1 as i64;
            let r = durs[j] / durs[i];
            gap < 40 && (0.7..=1.43).contains(&r)
        };
        let mut run = vec![1usize; phs.len()];
        let mut i = 0;
        while i < phs.len() {
            let mut j = i;
            while j + 1 < phs.len() && adj(j, j + 1) {
                j += 1;
            }
            for k in i..=j {
                run[k] = j - i + 1;
            }
            i = j + 1;
        }
        for (i, p) in phs.iter().enumerate() {
            let dur = (p.t1 - p.t0) as f32 / 100.0;
            let tot_a: u32 = p.sa.iter().sum();
            let tot_l: f32 = p.sl.iter().sum();
            let llr = tot_l / tot_a.max(1) as f32;
            let syl_m: Vec<f32> = (0..N_SYL).map(|s| if p.sa[s] > 0 { p.sl[s] / p.sa[s] as f32 } else { -10.0 }).collect();
            let minsyl = syl_m.iter().cloned().fold(f32::MAX, f32::min);
            let npos = syl_m.iter().filter(|&&v| v > 0.5).count();
            let disc: Vec<f32> = (0..N_SYL).map(|s| if p.sa[s] > 0 { p.disc[s] / p.sa[s] as f32 } else { -10.0 }).collect();
            let disc_min = disc.iter().cloned().fold(f32::MAX, f32::min);
            let disc_mean = disc.iter().sum::<f32>() / N_SYL as f32;
            let prevgap = if i > 0 { p.t0 as i64 - phs[i - 1].t1 as i64 } else { 9999 };
            let nextgap = if i + 1 < phs.len() { phs[i + 1].t0 as i64 - p.t1 as i64 } else { 9999 };
            let prevr = if i > 0 { durs[i] / durs[i - 1] } else { 0.0 };
            let nextr = if i + 1 < phs.len() { durs[i + 1] / durs[i] } else { 0.0 };
            let span_act = (p.t0..p.t1).filter(|&q| act[q]).count() as f32;
            let actsh = span_act / (p.t1 - p.t0) as f32;
            let mut hfv: Vec<f32> = (p.t0..p.t1).filter(|&q| act[q]).map(|q| frames[q].hf - hf_base).collect();
            hfv.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let hf = if hfv.is_empty() { 0.0 } else { hfv[hfv.len() / 2] };
            let voiced = (p.t0..p.t1).filter(|&q| act[q] && frames[q].voiced > 0.7).count() as f32 / span_act.max(1.0);
            let (t0s, t1s) = (p.t0 as f32 / 100.0, p.t1 as f32 / 100.0);
            let label: i32 = if tr.is_some() {
                0
            } else if has_truth {
                // matches a truth interval: overlap >= 50% of the shorter
                let ok = my_truth.iter().any(|&(a0, a1)| {
                    let ov = t1s.min(a1) - t0s.max(a0);
                    ov > 0.5 * (t1s - t0s).min(a1 - a0)
                });
                if ok { 1 } else { 0 }
            } else {
                -1
            };
            print!(
                "{src},{label},{t0s:.2},{t1s:.2},{dur:.2},{llr:.3},{minsyl:.3},{npos},{},{:.3},{},{actsh:.3},{:.3},{disc_min:.3},{disc_mean:.3},{prevgap},{nextgap},{prevr:.3},{nextr:.3},{},{hf:.2},{voiced:.3}",
                p.sf[N_SYL - 1],
                syl_m[N_SYL - 1],
                p.gap,
                p.ordc / tot_a.max(1) as f32,
                run[i]
            );
            for s in 0..N_SYL {
                print!(",{:.3}", syl_m[s]);
            }
            for s in 0..N_SYL {
                print!(",{}", p.sf[s]);
            }
            println!();
        }
    }
}
