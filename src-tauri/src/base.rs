//! The four reference recordings shipped with the app, used to bootstrap
//! the phrase model before the user has recorded anything.

use std::io::Cursor;

pub struct BaseClip {
    pub name: &'static str,
    /// Number of Daimoku in the clip.
    pub n_cycles: usize,
    pub wav: &'static [u8],
}

pub const BASE_CLIPS: [BaseClip; 11] = [
    BaseClip { name: "slow", n_cycles: 1, wav: include_bytes!("assets/base_slow.wav") },
    BaseClip { name: "medium", n_cycles: 1, wav: include_bytes!("assets/base_medium.wav") },
    BaseClip { name: "fast", n_cycles: 1, wav: include_bytes!("assets/base_fast.wav") },
    BaseClip { name: "chant7", n_cycles: 7, wav: include_bytes!("assets/base_chant7.wav") },
    // more examples at every tempo (16 kHz mono), so that the model works
    // well before the user records anything
    BaseClip { name: "t3", n_cycles: 10, wav: include_bytes!("assets/base_t3.wav") },
    BaseClip { name: "t4", n_cycles: 10, wav: include_bytes!("assets/base_t4.wav") },
    BaseClip { name: "t5", n_cycles: 3, wav: include_bytes!("assets/base_t5.wav") },
    BaseClip { name: "slow3", n_cycles: 3, wav: include_bytes!("assets/base_user_slow3.wav") },
    BaseClip { name: "fast10", n_cycles: 10, wav: include_bytes!("assets/base_u_fast10.wav") },
    BaseClip { name: "med5", n_cycles: 5, wav: include_bytes!("assets/base_u_med5.wav") },
    BaseClip { name: "slow3b", n_cycles: 3, wav: include_bytes!("assets/base_u_slow3b.wav") },
];

/// Decodes a WAV (16-bit int or 32-bit float, any channel count) to mono f32.
pub fn decode_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let mut r = hound::WavReader::new(Cursor::new(bytes)).map_err(|e| format!("bad wav: {e}"))?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let raw: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, _) => r.samples::<f32>().filter_map(|s| s.ok()).collect(),
        (hound::SampleFormat::Int, 16) => r.samples::<i16>().filter_map(|s| s.ok()).map(|s| s as f32 / 32768.0).collect(),
        (hound::SampleFormat::Int, b) => {
            let scale = (1i64 << (b - 1)) as f32;
            r.samples::<i32>().filter_map(|s| s.ok()).map(|s| s as f32 / scale).collect()
        }
    };
    let mono = if ch == 1 {
        raw
    } else {
        raw.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect()
    };
    Ok((mono, spec.sample_rate))
}
