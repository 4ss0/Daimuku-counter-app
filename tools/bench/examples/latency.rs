// How long does one Engine::push take? The microphone callback must return
// within a few milliseconds, or the audio system drops samples.
// usage: cargo run --release --example latency -- [minutes] [chunk]
//   feeds the built-in clips (looped, 48 kHz, `chunk` samples per call,
//   default 480 = 10 ms) and prints the distribution of push() times.
use daimoku_bench::base::*;
use daimoku_bench::engine::{build_slow_model, Engine};
use daimoku_bench::train::*;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let minutes: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let chunk: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(480);

    let base: Vec<TrainItem> = BASE_CLIPS
        .iter()
        .map(|c| {
            let (x, sr) = decode_wav(c.wav).unwrap();
            build_item(&x, sr, c.n_cycles, 1.0).unwrap()
        })
        .collect();
    let model = train_model(&base, &[], None, 8).unwrap().model;
    let slow = build_slow_model(&base, &[]);

    // the clips one after the other, 0.5 s apart, at 48 kHz (sample and hold)
    let sr = 48_000u32;
    let mut x: Vec<f32> = Vec::new();
    let mut n_true = 0;
    while (x.len() as f32) < minutes * 60.0 * sr as f32 {
        for c in BASE_CLIPS.iter() {
            let (y, ysr) = decode_wav(c.wav).unwrap();
            let k = (sr / ysr) as usize;
            for &s in &y {
                for _ in 0..k {
                    x.push(s);
                }
            }
            x.extend(std::iter::repeat(0.0).take(sr as usize / 2));
            n_true += c.n_cycles;
        }
    }

    let mut e = Engine::new(&model, sr);
    if let Some(s) = slow.as_ref() {
        e = e.with_slow(s, sr);
    }
    let mut times: Vec<f64> = Vec::with_capacity(x.len() / chunk + 1);
    let t0 = Instant::now();
    for c in x.chunks(chunk) {
        let t = Instant::now();
        e.push(c);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    e.finish();
    let total = t0.elapsed().as_secs_f64();
    let audio = x.len() as f64 / sr as f64;

    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| sorted[((sorted.len() - 1) as f64 * p) as usize];
    let budget = chunk as f64 / sr as f64 * 1000.0;
    let over = times.iter().filter(|&&t| t > budget).count();
    println!("audio {audio:.0} s, processed in {total:.2} s ({:.0}x real time)", audio / total);
    println!("push of {chunk} samples ({budget:.1} ms of audio): median {:.3} ms, p99 {:.3} ms, max {:.1} ms", q(0.5), q(0.99), q(1.0));
    println!("calls longer than the audio they carry: {over} of {}", times.len());
    println!("counted {} (clips contain {n_true})", e.count());
}
