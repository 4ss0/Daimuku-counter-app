// Evaluation bench for the counter.
// usage: bench [old|new] [A|P3|P1 ...]      env: BASE=old|new (default new for new engine)
//   FULL=1 also runs the loose (sh500) negatives and the talk tests
use daimoku_bench::base::*;
use daimoku_bench::model::Model;
use daimoku_bench::train::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 {
        (self.next() % 20001) as f32 / 10000.0 - 1.0
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
fn load(f: &str) -> (Vec<f32>, u32) {
    decode_wav(&std::fs::read(f).unwrap_or_else(|_| panic!("missing {f}"))).unwrap()
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct Ev {
    t: f32,
    span: f32,
    delay: f32,
    src: char,
}

thread_local! { static GENERIC: std::cell::RefCell<Option<Model>> = const { std::cell::RefCell::new(None) }; }

fn run(kind: &str, m: &Model, slow: Option<&Model>, x: &[f32], sr: u32, live_seed: Option<u64>) -> Vec<Ev> {
    if kind == "old" {
        let mut e = daimoku_bench::engine_old::Engine::new(m, sr);
        if let Some(s) = slow {
            e = e.with_slow(s, sr);
        }
        feed(x, live_seed, |c| e.push(c));
        e.finish();
        e.events()
            .iter()
            .map(|v| Ev { t: v.frame as f32 / 100.0, span: v.span as f32 / 100.0, delay: 0.0, src: if v.inferred { '*' } else { 'B' } })
            .collect()
    } else {
        let mut e = daimoku_bench::engine::Engine::new(m, sr);
        if let Some(s) = slow {
            e = e.with_slow(s, sr);
        }
        let _ = GENERIC.with(|g| g.borrow().is_some());
        feed(x, live_seed, |c| e.push(c));
        e.finish();
        e.events()
            .iter()
            .map(|v| Ev {
                t: v.frame as f32 / 100.0,
                span: v.span as f32 / 100.0,
                delay: v.added_at.saturating_sub(v.frame) as f32 / 100.0,
                src: match v.source {
                    daimoku_bench::engine::Source::Base => 'B',
                    daimoku_bench::engine::Source::Fast => 'F',
                    daimoku_bench::engine::Source::Slow => 'S',
                    daimoku_bench::engine::Source::Noise => 'N',
                },
            })
            .collect()
    }
}

fn feed(x: &[f32], live_seed: Option<u64>, mut push: impl FnMut(&[f32])) {
    match live_seed {
        None => {
            for c in x.chunks(2048) {
                push(c);
            }
        }
        Some(s) => {
            let mut r = Rng(s);
            let mut i = 0;
            while i < x.len() {
                let k = 64 + (r.next() % 4000) as usize;
                let j = (i + k).min(x.len());
                push(&x[i..j]);
                i = j;
            }
        }
    }
}

struct Models {
    main: Model,
    slow: Option<Model>,
    generic: Option<Model>,
}

fn base_items(enriched: bool) -> Vec<TrainItem> {
    // the app's built-in clips; the old engine was trained on the first 4
    let n = if enriched { BASE_CLIPS.len() } else { 4 };
    BASE_CLIPS[..n]
        .iter()
        .map(|c| {
            let (x, sr) = decode_wav(c.wav).unwrap();
            build_item(&x, sr, c.n_cycles, 1.0).unwrap()
        })
        .collect()
}

fn takes(prof: &str) -> Vec<(String, usize)> {
    match prof {
        "P3" => vec![("pcprof3/take-0.wav", 3), ("pcprof3/take-1.wav", 3), ("pcprof3/take-2.wav", 5), ("pcprof3/take-3.wav", 10), ("pcprof3/take-4.wav", 3)],
        _ => vec![],
    }
    .into_iter()
    .map(|(a, b)| (a.to_string(), b))
    .collect()
}

fn build_models(kind: &str, prof: &str, enriched: bool) -> Models {
    let base = base_items(enriched);
    let mut user: Vec<(TrainItem, f32)> = Vec::new();
    for (f, n) in takes(prof) {
        let (x, sr) = load(&f);
        let secs = x.len() as f32 / sr as f32;
        user.push((build_item(&x, sr, n, 2.5).unwrap(), secs / n as f32));
    }
    let mut all = base.clone();
    all.extend(user.iter().map(|u| u.0.clone()));
    let main = train_model(&all, &[], None, 8).unwrap().model;
    let slow = if kind == "old" {
        // as the app did: slowest base clip + slow takes
        let mut items = vec![base.iter().max_by_key(|b| b.feats.len() / b.n_cycles.max(1)).unwrap().clone()];
        items.extend(user.iter().filter(|u| u.1 >= 2.5).map(|u| u.0.clone()));
        train_model(&items, &[], None, 8).map(|t| t.model)
    } else {
        daimoku_bench::engine::build_slow_model(&base, &user.iter().map(|u| (u.0.clone(), u.1)).collect::<Vec<_>>())
    };
    let generic = if std::env::var("GEN").is_ok() && !user.is_empty() { train_model(&base, &[], None, 8).map(|t| t.model) } else { None };
    Models { main, slow, generic }
}

fn truth_intervals() -> Vec<(String, f32, f32)> {
    std::fs::read_to_string("truth/intervals.txt")
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let v: Vec<&str> = l.split_whitespace().collect();
            (v[0].to_string(), v[1].parse().unwrap(), v[2].parse().unwrap())
        })
        .collect()
}

/// One-to-one matching of events to truth phrases: an event counts for a
/// phrase if it lies within [start, end + 0.8].
fn match_events(ev: &[Ev], tr: &[(f32, f32)]) -> (usize, usize, Vec<usize>, Vec<f32>) {
    let mut used = vec![false; tr.len()];
    let mut tp = 0;
    let mut fp_t = Vec::new();
    for e in ev {
        let mut hit = None;
        for (i, &(a, b)) in tr.iter().enumerate() {
            if !used[i] && e.t >= a && e.t <= b + 0.8 {
                hit = Some(i);
                break;
            }
        }
        match hit {
            Some(i) => {
                used[i] = true;
                tp += 1;
            }
            None => fp_t.push(e.t),
        }
    }
    let missed: Vec<usize> = (0..tr.len()).filter(|&i| !used[i]).collect();
    (tp, fp_t.len(), missed, fp_t)
}

fn main() {
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data")).unwrap();
    let args: Vec<String> = std::env::args().collect();
    let kind = args.get(1).cloned().unwrap_or("new".into());
    let profs: Vec<String> = if args.len() > 2 { args[2..].to_vec() } else { vec!["A".into(), "P3".into()] };
    let enriched = std::env::var("BASE").map(|v| v == "new").unwrap_or(kind == "new");
    let full = std::env::var("FULL").is_ok();
    let verbose = std::env::var("V").is_ok();
    let truth = truth_intervals();
    let ifiles = ["c28", "r29", "r36", "j13", "j14"];
    let cfiles: [(&str, usize); 5] = [("wa0929", 148), ("live51", 55), ("pc0929", 13), ("noisy1", 17), ("fp13", 9)];
    let info = ["t1212", "live2"];
    let mut audio: std::collections::HashMap<String, (Vec<f32>, u32)> = Default::default();
    for f in ifiles.iter().chain(cfiles.iter().map(|c| &c.0)).chain(info.iter()) {
        audio.insert(f.to_string(), load(&format!("{f}.wav")));
    }
    // negatives
    let mut strict: Vec<(String, Vec<f32>, u32)> = Vec::new();
    for f in ["c28", "r29", "j14", "wa0929", "live51"] {
        let (x, sr) = &audio[f];
        strict.push((format!("{f}.rev"), x.iter().rev().cloned().collect(), *sr));
    }
    for f in ["wa0929", "live51"] {
        let (x, sr) = &audio[f];
        for s in [1u64, 7] {
            strict.push((format!("{f}.sh100.{s}"), shuffled(x, *sr as usize / 10, s), *sr));
        }
        for s in [1u64, 7, 99] {
            strict.push((format!("{f}.sh200.{s}"), shuffled(x, *sr as usize / 5, s), *sr));
        }
    }
    for e in std::fs::read_dir("neg").unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with("tts_") && !n.contains("daimoku") && n.ends_with(".wav") {
            let (x, sr) = load(&format!("neg/{n}"));
            strict.push((n, x, sr));
        }
    }
    let mut r = Rng(42);
    strict.push(("white".into(), (0..16000 * 30).map(|_| r.f() * 0.05).collect(), 16000));
    let mut sp = Vec::new();
    let mut ph = 0.0f32;
    for k in 0..(16000 * 30) {
        let t = k as f32 / 16000.0;
        let env = ((t * 3.1).sin() * (t * 1.7).sin()).abs();
        let f0 = 110.0 + 40.0 * (t * 0.9).sin();
        ph += f0 / 16000.0;
        let buzz = (ph.fract() - 0.5) * 2.0;
        sp.push(env * (0.6 * buzz + 0.4 * r.f()) * 0.2);
    }
    strict.push(("buzz".into(), sp, 16000));
    if std::path::Path::new("neg/juzu_only.wav").exists() {
        let (x, sr) = load("neg/juzu_only.wav");
        strict.push(("juzu_only".into(), x, sr));
    }
    let mut loose: Vec<(String, Vec<f32>, u32)> = Vec::new();
    if full {
        for f in ["wa0929", "live51"] {
            let (x, sr) = &audio[f];
            for s in [1u64, 7, 99] {
                loose.push((format!("{f}.sh500.{s}"), shuffled(x, *sr as usize / 2, s), *sr));
            }
        }
    }

    for prof in &profs {
        let t0 = std::time::Instant::now();
        let ms = build_models(&kind, prof, enriched);
        GENERIC.with(|g| *g.borrow_mut() = ms.generic.clone());
        let slow = ms.slow.as_ref();
        let mut line = format!("{kind}/{prof}{}:", if enriched { "+" } else { "" });
        let (mut tp_all, mut fp_all, mut tot_all) = (0, 0, 0);
        let (mut nb, mut nall) = (0usize, 0usize);
        let mut delays: Vec<f32> = Vec::new();
        for f in ifiles {
            let (x, sr) = &audio[f];
            let ev = run(&kind, &ms.main, slow, x, *sr, None);
            let tr: Vec<(f32, f32)> = truth.iter().filter(|t| t.0 == f).map(|t| (t.1, t.2)).collect();
            let (tp, fp, missed, fpt) = match_events(&ev, &tr);
            nall += ev.len();
            nb += ev.iter().filter(|e| e.src == 'B').count();
            delays.extend(ev.iter().filter(|e| e.src != 'B').map(|e| e.delay));
            // slow part = phrases longer than 2 s
            let slow_tot = tr.iter().filter(|t| t.1 - t.0 > 2.0).count();
            let slow_miss = missed.iter().filter(|&&i| tr[i].1 - tr[i].0 > 2.0).count();
            tp_all += tp;
            fp_all += fp;
            tot_all += tr.len();
            line += &format!(" {f}={tp}/{}", tr.len());
            if slow_tot > 0 {
                line += &format!("(L{}/{})", slow_tot - slow_miss, slow_tot);
            }
            if fp > 0 {
                line += &format!("+{fp}fp");
            }
            if verbose {
                let ms: Vec<String> = missed.iter().map(|&i| format!("{:.1}", tr[i].0)).collect();
                let fs: Vec<String> = fpt.iter().map(|t| format!("{t:.1}")).collect();
                let evs: Vec<String> = ev.iter().map(|e| format!("{:.1}{}", e.t, if e.src == 'B' { "".to_string() } else { e.src.to_string() })).collect();
                eprintln!("  {prof} {f}: missed@[{}] fp@[{}] ev: {}", ms.join(" "), fs.join(" "), evs.join(" "));
            }
        }
        delays.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let dmed = delays.get(delays.len() / 2).cloned().unwrap_or(0.0);
        let dmax = delays.last().cloned().unwrap_or(0.0);
        line += &format!(" | TP {tp_all}/{tot_all} FP {fp_all} B{}% d{dmed:.1}/{dmax:.1}s ||", 100 * nb / nall.max(1));
        let mut err = 0i64;
        for (f, want) in cfiles {
            let (x, sr) = &audio[f];
            let c = run(&kind, &ms.main, slow, x, *sr, None).len();
            err += (c as i64 - want as i64).abs();
            line += &format!(" {f}={c}/{want}");
        }
        line += &format!(" (|err| {err}) ||");
        for f in info {
            let (x, sr) = &audio[f];
            let c = run(&kind, &ms.main, slow, x, *sr, None).len();
            line += &format!(" {f}={c}");
        }
        // live chunking consistency on wa0929
        let (x, sr) = &audio["wa0929"];
        let lv = run(&kind, &ms.main, slow, x, *sr, Some(5)).len();
        line += &format!(" wa-live={lv}");
        println!("{line}");
        let mut fp = 0;
        let mut fpl = Vec::new();
        for (n, x, sr) in &strict {
            let ev = run(&kind, &ms.main, slow, x, *sr, None);
            let c = ev.len();
            if c > 0 {
                fp += c;
                let srcs: String = ev.iter().map(|e| e.src).collect();
                fpl.push(format!("{n}:{c}{srcs}"));
            }
        }
        let mut lfp = 0;
        let mut lfpl = Vec::new();
        for (n, x, sr) in &loose {
            let ev = run(&kind, &ms.main, slow, x, *sr, None);
            let c = ev.len();
            if c > 0 {
                lfp += c;
                let srcs: String = ev.iter().map(|e| e.src).collect();
                lfpl.push(format!("{n}:{c}{srcs}"));
            }
        }
        let mut extra = String::new();
        if full {
            let (wa, wsr) = &audio["wa0929"];
            let rs = |x: &[f32], from: f32, to: f32, sr: u32| -> Vec<f32> {
                let a = (from * sr as f32) as usize;
                let b = ((to * sr as f32) as usize).min(x.len());
                x[a..b].to_vec()
            };
            let p1 = rs(wa, 0.0, 30.0, *wsr);
            let p2 = rs(wa, 30.0, 60.0, *wsr);
            let (tx, tsr) = load("neg/tts_it0_0.wav");
            let ratio = *wsr as f32 / tsr as f32;
            let talk: Vec<f32> = (0..((tx.len() as f32 * ratio) as usize).min(*wsr as usize * 12)).map(|i| tx[((i as f32 / ratio) as usize).min(tx.len() - 1)] * 0.25).collect();
            let mix: Vec<f32> = p1.iter().chain(talk.iter()).chain(p2.iter()).cloned().collect();
            let c1 = run(&kind, &ms.main, slow, &p1, *wsr, None).len();
            let c2 = run(&kind, &ms.main, slow, &p2, *wsr, None).len();
            let cm = run(&kind, &ms.main, slow, &mix, *wsr, None).len();
            extra += &format!(" | talk-mix {cm} vs {c1}+{c2}");
            let seg = rs(wa, 8.9, 36.7, *wsr);
            let cs = run(&kind, &ms.main, slow, &seg, *wsr, None).len();
            let mut pre = String::new();
            for (tf, secs) in [("neg/tts_it0_0.wav", 1.2f32), ("neg/tts_it0_1.wav", 2.0), ("neg/tts_en1_0.wav", 1.5), ("neg/tts_it3_0.wav", 1.3), ("neg/tts_ja2_1.wav", 1.0)] {
                let (tx, tsr) = load(tf);
                let ratio = *wsr as f32 / tsr as f32;
                let sp: Vec<f32> = (0..(secs * *wsr as f32) as usize).map(|i| tx[((i as f32 / ratio) as usize).min(tx.len() - 1)] * 0.25).collect();
                let x: Vec<f32> = std::iter::repeat(0.0)
                    .take(*wsr as usize / 3)
                    .chain(sp.into_iter())
                    .chain(std::iter::repeat(0.0).take(*wsr as usize / 4))
                    .chain(seg.iter().cloned())
                    .collect();
                pre += &format!("{} ", run(&kind, &ms.main, slow, &x, *wsr, None).len());
            }
            extra += &format!(" | speech-before: {pre}vs {cs}");
        }
        println!("   strictFP {fp} {:?}{}{}  [{:.1}s]", fpl, if full { format!(" | looseFP {lfp} {:?}", lfpl) } else { String::new() }, extra, t0.elapsed().as_secs_f32());
    }
}
