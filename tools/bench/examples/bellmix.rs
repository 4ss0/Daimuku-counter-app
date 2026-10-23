// The bell of the Count screen rung while chanting: a session is counted
// as it is, with the bell mixed in, and with the bell mixed in and taken
// out again by the app's filter (src-tauri/src/bell.rs).
// usage: bellmix <session.wav> <bell.wav> <kind> <truth> [take.wav:n ...]
//   env GAIN: bell peak / voice peak (default 1), EVERY: seconds between
//   strikes (default 12), FIRST: first strike (s, default 1)
use daimoku_bench::base::*;
use daimoku_bench::bell::BellFilter;
use daimoku_bench::engine::{build_slow_model, Engine};
use daimoku_bench::model::Model;
use daimoku_bench::train::*;

fn load(f: &str) -> (Vec<f32>, u32) {
    decode_wav(&std::fs::read(f).unwrap_or_else(|_| panic!("missing {f}"))).unwrap()
}

fn models(takes: &[(String, usize)]) -> (Model, Option<Model>) {
    let base: Vec<TrainItem> = BASE_CLIPS
        .iter()
        .map(|c| {
            let (x, sr) = decode_wav(c.wav).unwrap();
            build_item(&x, sr, c.n_cycles, 1.0).unwrap()
        })
        .collect();
    let w = user_take_weight(takes.len());
    let user: Vec<(TrainItem, f32)> = takes
        .iter()
        .map(|(f, n)| {
            let (x, sr) = load(f);
            let secs = x.len() as f32 / sr as f32;
            (build_item(&x, sr, *n, w).unwrap(), secs / *n as f32)
        })
        .collect();
    let mut all = base.clone();
    all.extend(user.iter().map(|u| u.0.clone()));
    let main = train_model(&all, &[], None, 8).unwrap().model;
    let slow = build_slow_model(&base, &user);
    (main, slow)
}

fn count(m: &Model, slow: Option<&Model>, x: &[f32], sr: u32, strikes: &[usize], kind: Option<&str>) -> usize {
    let mut e = Engine::new(m, sr);
    if let Some(s) = slow {
        e = e.with_slow(s, sr);
    }
    let mut f = BellFilter::new(sr);
    let chunk = sr as usize / 100;
    for (i, c) in x.chunks(chunk).enumerate() {
        let mut c = c.to_vec();
        if let Some(k) = kind {
            if strikes.iter().any(|&s| s / chunk == i) {
                f.strike(k);
            }
            f.process(&mut c);
        }
        e.push(&c);
    }
    e.finish();
    e.count()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (x, sr) = load(&a[1]);
    let (bell, bsr) = load(&a[2]);
    assert_eq!(sr, bsr, "bell and session at different rates");
    let kind = a[3].as_str();
    let truth: usize = a[4].parse().unwrap();
    let takes: Vec<(String, usize)> = a[5..]
        .iter()
        .map(|s| {
            let (f, n) = s.rsplit_once(':').unwrap();
            (f.to_string(), n.parse().unwrap())
        })
        .collect();
    let env = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let gain = env("GAIN", 1.0);
    let every = env("EVERY", 12.0);
    let first = env("FIRST", 1.0);
    let (m, slow) = models(&takes);
    // the voice's peak level: 99th percentile of |x|
    let mut ab: Vec<f32> = x.iter().map(|v| v.abs()).collect();
    ab.sort_by(|p, q| p.partial_cmp(q).unwrap());
    let vpeak = ab[ab.len() * 99 / 100];
    let bpeak = bell.iter().fold(0.0f32, |p, v| p.max(v.abs()));
    let g = gain * vpeak / bpeak.max(1e-6);
    let mut strikes = Vec::new();
    let mut t = first;
    while ((t * sr as f32) as usize) < x.len() {
        strikes.push((t * sr as f32) as usize);
        t += every;
    }
    let mut y = x.clone();
    for &s in &strikes {
        for (k, b) in bell.iter().enumerate() {
            if s + k < y.len() {
                y[s + k] += g * b;
            }
        }
    }
    let clean = count(&m, slow.as_ref(), &x, sr, &[], None);
    let rung = count(&m, slow.as_ref(), &y, sr, &strikes, None);
    let filtered = count(&m, slow.as_ref(), &y, sr, &strikes, Some(kind));
    let clean_f = count(&m, slow.as_ref(), &x, sr, &strikes, Some(kind));
    println!(
        "{kind:6} gain {gain:3.1} every {every:4.1}s ({} strikes): true {truth} | clean {clean} | bell {rung} | bell+filter {filtered} | no bell, filter on {clean_f}",
        strikes.len()
    );
}
