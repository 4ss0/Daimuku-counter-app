// Runs one recorded session through the engine with a personal profile
// built exactly as the app builds it (src-tauri/src/profile.rs), and lists
// every Daimoku counted with the controller that found it.
// usage: session <session.wav> [take.wav:n_daimoku ...]
//   env TRUTH=n prints the error against the true count
//   env BASEONLY=1 also runs the base-only model (no profile)
//   env NEG=1 also counts the session reversed and cut in 100/200/500 ms
//   pieces and shuffled (all of which should count nothing)
use daimoku_bench::base::*;
use daimoku_bench::engine::{build_slow_model, Engine, Source};
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

fn run(label: &str, m: &Model, slow: Option<&Model>, x: &[f32], sr: u32, truth: Option<usize>) {
    let mut e = Engine::new(m, sr);
    if let Some(s) = slow {
        e = e.with_slow(s, sr);
    }
    // microphone-sized pieces, as the app receives them
    for c in x.chunks(sr as usize / 100) {
        e.push(c);
    }
    e.finish();
    let ev = e.events();
    println!("== {label}: counted {}{}", ev.len(), truth.map_or(String::new(), |t| format!(" (true {t}, error {:+})", ev.len() as i64 - t as i64)));
    let mut by = [0usize; 4];
    let mut prev: Option<f32> = None;
    for v in ev {
        let t = v.frame as f32 / 100.0;
        let src = match v.source {
            Source::Base => 'B',
            Source::Fast => 'F',
            Source::Slow => 'S',
            Source::Noise => 'N',
        };
        by[match src { 'B' => 0, 'F' => 1, 'S' => 2, _ => 3 }] += 1;
        let gap = prev.map_or(String::new(), |p| format!("  +{:.2}", t - p));
        println!(
            "  {t:7.2} s  {src}  span {:.2} s  late {:.1} s{gap}",
            v.span as f32 / 100.0,
            v.added_at.saturating_sub(v.frame) as f32 / 100.0
        );
        prev = Some(t);
    }
    println!("  by controller: B {} F {} S {} N {}", by[0], by[1], by[2], by[3]);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (x, sr) = load(&a[1]);
    let takes: Vec<(String, usize)> = a[2..]
        .iter()
        .map(|s| {
            let (f, n) = s.rsplit_once(':').expect("take.wav:n");
            (f.to_string(), n.parse().unwrap())
        })
        .collect();
    let truth = std::env::var("TRUTH").ok().and_then(|t| t.parse().ok());
    println!("{}: {:.1} s at {sr} Hz, profile of {} takes", a[1], x.len() as f32 / sr as f32, takes.len());
    let (m, slow) = models(&takes);
    run("profile", &m, slow.as_ref(), &x, sr, truth);
    if std::env::var("NEG").is_ok() {
        let rev: Vec<f32> = x.iter().rev().cloned().collect();
        let mut out = Vec::new();
        for (name, y) in [
            ("reversed", rev),
            ("100 ms", shuffled(&x, sr as usize / 10, 7)),
            ("200 ms", shuffled(&x, sr as usize / 5, 11)),
            ("500 ms", shuffled(&x, sr as usize / 2, 13)),
        ] {
            let mut e = Engine::new(&m, sr);
            if let Some(s) = slow.as_ref() {
                e = e.with_slow(s, sr);
            }
            for c in y.chunks(sr as usize / 100) {
                e.push(c);
            }
            e.finish();
            out.push(format!("{name} {}", e.count()));
        }
        println!("== negatives (profile): {}", out.join(", "));
    }
    if std::env::var("BASEONLY").is_ok() {
        let (m, slow) = models(&[]);
        run("base only", &m, slow.as_ref(), &x, sr, truth);
    }
}
