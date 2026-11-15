// The user's own bell (src-tauri/src/bell.rs: `learn_partials`, and the
// filter that hears the bell by itself).
// usage: bellauto learn <wav> ...
//          the partials found in each recording
//        bellauto count <wav> <f1,f2,...|learn|none> [take.wav:n ...]
//          the recording counted through the filter; with `learn` the
//          partials are those found in the recording itself
//   env BELLTRAIN=1: the takes go through the same filter before training
use daimoku_bench::base::*;
use daimoku_bench::bell::{learn_partials, without_bell, BellFilter};
use daimoku_bench::engine::{build_slow_model, Engine, Source};
use daimoku_bench::model::Model;
use daimoku_bench::train::*;

fn load(f: &str) -> (Vec<f32>, u32) {
    decode_wav(&std::fs::read(f).unwrap_or_else(|_| panic!("missing {f}"))).unwrap()
}

fn models(takes: &[(String, usize)], partials: &[f32]) -> (Model, Option<Model>) {
    let through = std::env::var("BELLTRAIN").is_ok();
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
            let x = if through { without_bell(&x, sr, partials) } else { x };
            let secs = x.len() as f32 / sr as f32;
            (build_item(&x, sr, *n, w).unwrap(), secs / *n as f32)
        })
        .collect();
    let mut all = base.clone();
    all.extend(user.iter().map(|u| u.0.clone()));
    (train_model(&all, &[], None, 8).unwrap().model, build_slow_model(&base, &user))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let name = |f: &str| std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
    if a[1] == "learn" {
        for f in &a[2..] {
            let (x, sr) = load(f);
            let p = learn_partials(&x, sr);
            println!("{:<22} {}", name(f), if p.is_empty() { "no bell".to_string() } else { p.iter().map(|v| format!("{v:.1}")).collect::<Vec<_>>().join(", ") });
        }
        return;
    }
    let (x, sr) = load(&a[2]);
    let partials: Vec<f32> = match a[3].as_str() {
        "none" => Vec::new(),
        "learn" => learn_partials(&x, sr),
        list => list.split(',').map(|v| v.parse().unwrap()).collect(),
    };
    let takes: Vec<(String, usize)> = a[4..]
        .iter()
        .map(|s| {
            let (f, n) = s.rsplit_once(':').unwrap();
            (f.to_string(), n.parse().unwrap())
        })
        .collect();
    let (m, slow) = models(&takes, &partials);
    let mut e = Engine::new(&m, sr);
    if let Some(s) = slow.as_ref() {
        e = e.with_slow(s, sr);
    }
    let mut f = BellFilter::new(sr);
    f.set_partials(&partials);
    // microphone-sized pieces, as the app receives them
    let (mut on, mut blocks) = (0usize, 0usize);
    for c in x.chunks(sr as usize / 100) {
        let mut c = c.to_vec();
        f.process(&mut c);
        on += f.own_ringing() as usize;
        blocks += 1;
        e.push(&c);
    }
    e.finish();
    let mut by = [0usize; 4];
    for v in e.events() {
        by[match v.source {
            Source::Base => 0,
            Source::Fast => 1,
            Source::Slow => 2,
            Source::Noise => 3,
        }] += 1;
    }
    println!(
        "{:<22} bell [{}]: counted {:3} | B {} F {} S {} N {} | filter on {:.0}% of the time",
        name(&a[2]),
        partials.iter().map(|v| format!("{v:.0}")).collect::<Vec<_>>().join(","),
        e.count(),
        by[0],
        by[1],
        by[2],
        by[3],
        100.0 * on as f32 / blocks.max(1) as f32
    );
}
