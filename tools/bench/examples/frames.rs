// Front-end values every 0.1 s of a file: energy, activity, voicing, bands.
// usage: frames <file.wav> [from_s] [to_s]
use daimoku_bench::base::decode_wav;
use daimoku_bench::features::extract_frames;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (x, sr) = decode_wav(&std::fs::read(&a[1]).unwrap()).unwrap();
    let from: f32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let to: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(f32::MAX);
    let fr = extract_frames(&x, sr);
    for (k, c) in fr.chunks(10).enumerate() {
        let t = k as f32 / 10.0;
        if t < from || t > to {
            continue;
        }
        let act = c.iter().filter(|f| f.active).count();
        let e = c.iter().map(|f| f.energy).sum::<f32>() / c.len() as f32;
        let v = c.iter().map(|f| f.voiced).sum::<f32>() / c.len() as f32;
        let lf = c.iter().map(|f| f.lf).sum::<f32>() / c.len() as f32;
        let hf = c.iter().map(|f| f.hf).sum::<f32>() / c.len() as f32;
        println!("{t:6.1}  energy {e:6.2}  active {act:2}/10  voiced {v:4.2}  lf {lf:6.2}  hf {hf:6.2}  {}", "#".repeat(((e + 16.0).max(0.0) * 3.0) as usize));
    }
}
