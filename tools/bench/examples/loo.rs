// Leave-one-out check of a set of voice takes: each take is counted by the
// profile trained without it (as the app builds profiles), and so are its
// negatives (reversed, cut in 100/200 ms pieces and shuffled), which must
// count nothing.
// usage: loo <take.wav:n_daimoku> ...
use daimoku_bench::base::*;
use daimoku_bench::engine::{build_slow_model, Engine};
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

fn count(m: &Model, slow: Option<&Model>, x: &[f32], sr: u32) -> usize {
    let mut e = Engine::new(m, sr);
    if let Some(s) = slow {
        e = e.with_slow(s, sr);
    }
    for c in x.chunks(sr as usize / 100) {
        e.push(c);
    }
    e.finish();
    e.count()
}

fn main() {
    let takes: Vec<(Vec<f32>, u32, usize, String)> = std::env::args()
        .skip(1)
        .map(|s| {
            let (f, n) = s.rsplit_once(':').expect("take.wav:n");
            let (x, sr) = decode_wav(&std::fs::read(f).unwrap()).unwrap();
            (x, sr, n.parse().unwrap(), f.rsplit('/').next().unwrap().to_string())
        })
        .collect();
    let base: Vec<TrainItem> = BASE_CLIPS
        .iter()
        .map(|c| {
            let (x, sr) = decode_wav(c.wav).unwrap();
            build_item(&x, sr, c.n_cycles, 1.0).unwrap()
        })
        .collect();
    let (mut exact, mut err, mut truth, mut fp) = (0, 0i64, 0, 0);
    for (i, (x, sr, n, name)) in takes.iter().enumerate() {
        let others: Vec<&(Vec<f32>, u32, usize, String)> = takes.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, t)| t).collect();
        let w = user_take_weight(others.len());
        let user: Vec<(TrainItem, f32)> = others
            .iter()
            .map(|(y, ysr, k, _)| (build_item(y, *ysr, *k, w).unwrap(), y.len() as f32 / *ysr as f32 / *k as f32))
            .collect();
        let mut all = base.clone();
        all.extend(user.iter().map(|u| u.0.clone()));
        let m = train_model(&all, &[], None, 8).unwrap().model;
        let slow = build_slow_model(&base, &user);
        let c = count(&m, slow.as_ref(), x, *sr);
        let rev: Vec<f32> = x.iter().rev().cloned().collect();
        let negs = [
            count(&m, slow.as_ref(), &rev, *sr),
            count(&m, slow.as_ref(), &shuffled(x, *sr as usize / 10, 7 + i as u64), *sr),
            count(&m, slow.as_ref(), &shuffled(x, *sr as usize / 5, 11 + i as u64), *sr),
        ];
        println!("{name:12} true {n:2} counted {c:2}   negatives (reversed, 100 ms, 200 ms): {negs:?}");
        exact += (c == *n) as usize;
        err += (c as i64 - *n as i64).abs();
        truth += n;
        fp += negs.iter().sum::<usize>();
    }
    println!("takes counted exactly {exact}/{}, |error| {err} of {truth} Daimoku, negatives counted {fp}", takes.len());
}
