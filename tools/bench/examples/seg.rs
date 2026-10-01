// Offline analysis: full-hindsight Viterbi over a whole file with the live
// loop graph; prints every phrase on the best path with per-syllable stats.
// usage: seg <test.wav> [train files "file:n" ...]   (env SLOWM=1: slow model)
use daimoku_bench::base::*;
use daimoku_bench::features::*;
use daimoku_bench::hmm::*;
use daimoku_bench::model::*;
use daimoku_bench::train::*;

fn main() {
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data")).unwrap();
    let a: Vec<String> = std::env::args().collect();
    let mut items: Vec<TrainItem> = (0..4)
        .map(|i| {
            let (x, sr) = decode_wav(BASE_CLIPS[i].wav).unwrap();
            build_item(&x, sr, BASE_CLIPS[i].n_cycles, 1.0).unwrap()
        })
        .collect();
    for s in &a[2..] {
        let (f, n) = s.split_once(':').unwrap();
        let (x, sr) = decode_wav(&std::fs::read(f).unwrap()).unwrap();
        items.push(build_item(&x, sr, n.parse().unwrap(), 2.5).unwrap());
    }
    let m = train_model(&items, &[], None, 8).unwrap().model;
    let sc = Scorer::new(&m);
    let (x, sr) = decode_wav(&std::fs::read(&a[1]).unwrap()).unwrap();
    let frames = extract_frames(&x, sr);
    let mut cmn = OnlineCmn::new(&m.cep_mean);
    let mut emit = vec![[0.0f32; N_NODES]; frames.len()];
    let mut act = vec![false; frames.len()];
    for (t, f) in frames.iter().enumerate() {
        cmn.update(&f.cep, f.active);
        let feat = Feat { x: cmn.normalise(f), active: f.active };
        sc.emissions(&feat, &mut emit[t]);
        act[t] = f.active;
    }
    let g = Graph::decode_loop_min(2);
    let em: Vec<[f32; N_NODES]> = emit.clone();
    // the min-duration graph maps nodes -> model nodes via emit_idx; viterbi_offline indexes emit by emit_idx
    let path = viterbi_offline(&g, &em).unwrap();
    let nm: Vec<usize> = path.iter().map(|&n| g.emit_idx[n]).collect();
    // walk phrases
    let mut t = 0;
    let n = nm.len();
    let mut out = String::new();
    while t < n {
        let m0 = nm[t];
        if m0 < N_CHAIN && m0 / K == 0 && (t == 0 || !(nm[t - 1] < N_CHAIN && nm[t - 1] / K == 0)) {
            // phrase start: follow until leaving the chain/pause region or re-entering Nam from kyo
            let start = t;
            let mut syl_f = [0u32; N_SYL];
            let mut syl_l = [0f32; N_SYL];
            let mut syl_a = [0u32; N_SYL];
            let mut gap = 0u32;
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
                    syl_f[s] += 1;
                    if act[u] {
                        syl_a[s] += 1;
                        syl_l[s] += emit[u][mm] - emit[u][NODE_GAR];
                    }
                } else if (NODE_PAUSE0..NODE_SIL).contains(&mm) {
                    if act[u] {
                        gap += 1;
                    }
                } else {
                    break;
                }
                u += 1;
            }
            let reached_kyo = syl_f[N_SYL - 1] > 0;
            let tot_a: u32 = syl_a.iter().sum();
            let tot_l: f32 = syl_l.iter().sum();
            out.push_str(&format!(
                "{:6.2}-{:6.2} {:4.2}s {} llr {:5.2} gap {:3} | ",
                start as f32 / 100.0,
                u as f32 / 100.0,
                (u - start) as f32 / 100.0,
                if reached_kyo { "KYO" } else { "---" },
                tot_l / tot_a.max(1) as f32,
                gap
            ));
            for s in 0..N_SYL {
                out.push_str(&format!("{}:{:3}/{:5.1} ", SYLLABLES[s], syl_f[s], syl_l[s] / syl_a[s].max(1) as f32));
            }
            out.push('\n');
            t = u.max(t + 1);
        } else {
            t += 1;
        }
    }
    print!("{}", out);
    // garbage share
    let gar = nm.iter().zip(&act).filter(|(&m, &a)| m == NODE_GAR && a).count();
    let tot = act.iter().filter(|&&a| a).count();
    println!("active {} garbage {} ({:.0}%)  ref_llr {:.2}", tot, gar, 100.0 * gar as f32 / tot.max(1) as f32, m.ref_llr);
}
