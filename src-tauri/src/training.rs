use crate::audio::RecordedAudio;
use crate::features::{calibrate_voice, VoiceBand};
use crate::profile::TrainingNoise;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// One part of a guided recording (see `TrainingStore::add_guided`): the
/// user chanted `n` Daimoku between `start_ms` and `end_ms` of it.
#[derive(Debug, Clone, Deserialize)]
pub struct GuidedStage {
    pub start_ms: u64,
    pub end_ms: u64,
    pub n: u32,
    /// "slow" | "medium" | "fast"
    pub speed: Option<String>,
    /// The noise made on purpose while chanting, if any.
    pub noise: Option<TrainingNoise>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrainingTakeMeta {
    pub id: usize,
    pub expected_daimoku_count: u32,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub created_at: DateTime<Utc>,
}

struct TrainingTake {
    speed: Option<String>,
    noise: Option<TrainingNoise>,
    voice_band: Option<VoiceBand>,
    audio: RecordedAudio,
    expected_daimoku_count: u32,
    created_at: DateTime<Utc>,
}

pub struct TrainingStore {
    takes: Mutex<Vec<TrainingTake>>,
    /// Partials of the bell heard in the last guided recording.
    bell: Mutex<Vec<f32>>,
}

impl TrainingStore {
    pub fn new() -> Self {
        Self {
            takes: Mutex::new(Vec::new()),
            bell: Mutex::new(Vec::new()),
        }
    }

    /// Cuts a guided recording into one take per stage. The recording
    /// starts with `NOISE_SECONDS` of silence (the room, for the voice
    /// band of the stages without noise); the stages with a bell are also
    /// where the bell's partials are measured. Returns the takes and how
    /// many partials of a bell were found.
    pub fn add_guided(&self, audio: RecordedAudio, stages: &[GuidedStage]) -> Result<(Vec<TrainingTakeMeta>, usize), String> {
        let sr = audio.sample_rate as usize;
        let room = sr * crate::features::NOISE_SECONDS as usize;
        if sr < 8000 || stages.is_empty() || audio.samples.len() < room + sr / 2 {
            return Err("calibration-too-short".into());
        }
        let at = |ms: u64| ((ms as usize).saturating_mul(sr) / 1000).min(audio.samples.len());
        let mut metas = Vec::with_capacity(stages.len());
        let mut with_bell: Vec<f32> = Vec::new();
        for st in stages {
            let (a, b) = (at(st.start_ms).max(room), at(st.end_ms));
            if st.n == 0 || b < a + sr / 2 {
                return Err("calibration-too-short".into());
            }
            let speed = st.speed.clone();
            let meta = match st.noise {
                Some(noise) => {
                    if matches!(noise, TrainingNoise::Bell | TrainingNoise::Both) {
                        with_bell.extend_from_slice(&audio.samples[a..b]);
                    }
                    // a quarter of a second before the stage, as `add_noisy`
                    let samples = audio.samples[a.saturating_sub(sr / 4).max(room)..b].to_vec();
                    self.add_take(RecordedAudio { samples, sample_rate: audio.sample_rate }, st.n, None, Some(noise), speed)
                }
                None => {
                    // the room's silence, then the stage: what `calibrate_voice` expects
                    let mut samples = audio.samples[..room].to_vec();
                    samples.extend_from_slice(&audio.samples[a..b]);
                    let (start, band) = calibrate_voice(&samples, audio.sample_rate)?;
                    samples.drain(..start);
                    self.add_take(RecordedAudio { samples, sample_rate: audio.sample_rate }, st.n, Some(band), None, speed)
                }
            };
            metas.push(meta);
        }
        let bell = crate::bell::learn_partials(&with_bell, audio.sample_rate);
        let found = bell.len();
        *self.bell.lock().unwrap() = bell;
        Ok((metas, found))
    }

    /// Partials of the bell heard in the last guided recording (empty: none).
    pub fn bell(&self) -> Vec<f32> {
        self.bell.lock().unwrap().clone()
    }

    pub fn speed(&self, index: usize) -> Result<Option<String>, String> {
        self.takes.lock().unwrap().get(index).map(|t| t.speed.clone())
            .ok_or_else(|| format!("training take {index} not found"))
    }

    pub fn add(&self, audio: RecordedAudio, expected_daimoku_count: u32) -> TrainingTakeMeta {
        self.add_with_band(audio, expected_daimoku_count, None, None)
    }

    pub fn add_calibrated(&self, mut audio: RecordedAudio, expected: u32) -> Result<TrainingTakeMeta, String> {
        let (start, band) = calibrate_voice(&audio.samples, audio.sample_rate)?;
        audio.samples.drain(..start);
        Ok(self.add_with_band(audio, expected, Some(band), None))
    }

    /// Keep the same countdown/pre-roll as clean training, but do not infer
    /// the voice band from a mixture containing bell or bead frequencies.
    pub fn add_noisy(&self, mut audio: RecordedAudio, expected: u32, noise: TrainingNoise) -> Result<TrainingTakeMeta, String> {
        let sr = audio.sample_rate as usize;
        let boundary = sr * crate::features::NOISE_SECONDS as usize;
        if sr < 8000 || audio.samples.len() < boundary + sr / 2 {
            return Err("calibration-too-short".into());
        }
        audio.samples.drain(..boundary.saturating_sub(sr / 4));
        Ok(self.add_with_band(audio, expected, None, Some(noise)))
    }

    pub fn noise(&self, index: usize) -> Result<Option<TrainingNoise>, String> {
        self.takes.lock().unwrap().get(index).map(|t| t.noise)
            .ok_or_else(|| format!("training take {index} not found"))
    }

    pub fn voice_band(&self, index: usize) -> Result<Option<VoiceBand>, String> {
        self.takes.lock().unwrap().get(index).map(|t| t.voice_band)
            .ok_or_else(|| format!("training take {index} not found"))
    }

    fn add_with_band(&self, audio: RecordedAudio, expected_daimoku_count: u32, voice_band: Option<VoiceBand>, noise: Option<TrainingNoise>) -> TrainingTakeMeta {
        self.add_take(audio, expected_daimoku_count, voice_band, noise, None)
    }

    fn add_take(&self, audio: RecordedAudio, expected_daimoku_count: u32, voice_band: Option<VoiceBand>, noise: Option<TrainingNoise>, speed: Option<String>) -> TrainingTakeMeta {
        let mut takes = self.takes.lock().unwrap();
        let id = takes.len();
        let created_at = Utc::now();
        let meta = meta_for(id, &audio, expected_daimoku_count, created_at);
        takes.push(TrainingTake {
            speed,
            noise,
            voice_band,
            audio,
            expected_daimoku_count,
            created_at,
        });
        meta
    }

    pub fn list(&self) -> Vec<TrainingTakeMeta> {
        self.takes
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, t)| meta_for(i, &t.audio, t.expected_daimoku_count, t.created_at))
            .collect()
    }

    pub fn clear(&self) -> usize {
        let mut takes = self.takes.lock().unwrap();
        let count = takes.len();
        takes.clear();
        self.bell.lock().unwrap().clear();
        count
    }

    pub fn get(&self, index: usize) -> Result<RecordedAudio, String> {
        let takes = self.takes.lock().unwrap();
        takes
            .get(index)
            .map(|t| t.audio.clone())
            .ok_or_else(|| format!("training take {index} not found"))
    }

    /// Returns the audio plus the expected Daimoku count declared during
    /// recording. Needed by the validation step.
    pub fn get_with_expected(&self, index: usize) -> Result<(RecordedAudio, u32), String> {
        let takes = self.takes.lock().unwrap();
        takes
            .get(index)
            .map(|t| (t.audio.clone(), t.expected_daimoku_count))
            .ok_or_else(|| format!("training take {index} not found"))
    }
}

impl Default for TrainingStore {
    fn default() -> Self {
        Self::new()
    }
}

fn meta_for(
    id: usize,
    audio: &RecordedAudio,
    expected_daimoku_count: u32,
    created_at: DateTime<Utc>,
) -> TrainingTakeMeta {
    let duration_ms = if audio.sample_rate > 0 {
        (audio.samples.len() as u64 * 1000) / audio.sample_rate as u64
    } else {
        0
    };
    TrainingTakeMeta {
        id,
        expected_daimoku_count,
        duration_ms,
        sample_rate: audio.sample_rate,
        created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_audio(seconds: f32, rate: u32) -> RecordedAudio {
        let n = (seconds * rate as f32) as usize;
        RecordedAudio {
            samples: vec![0.0; n],
            sample_rate: rate,
        }
    }

    #[test]
    fn add_and_list_returns_metadata_with_expected_count() {
        let store = TrainingStore::new();
        store.add(fake_audio(40.0, 48000), 10);
        store.add(fake_audio(20.0, 48000), 5);

        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].expected_daimoku_count, 10);
        assert_eq!(list[0].duration_ms, 40_000);
        assert_eq!(list[1].expected_daimoku_count, 5);
        assert_eq!(list[1].duration_ms, 20_000);
    }

    #[test]
    fn a_guided_recording_is_cut_into_its_stages() {
        let store = TrainingStore::new();
        let sr = 16_000u32;
        // 3 s of silence, then a tone standing in for the voice
        let mut samples = vec![0.0f32; 3 * sr as usize];
        samples.extend((0..20 * sr as usize).map(|i| 0.2 * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / sr as f32).sin()));
        let stage = |start_ms, end_ms, n, noise| GuidedStage { start_ms, end_ms, n, speed: Some("slow".into()), noise };
        let stages = [stage(4_000, 10_000, 3, Some(TrainingNoise::Bell)), stage(12_000, 18_000, 3, None)];
        let (metas, bell) = store.add_guided(RecordedAudio { samples, sample_rate: sr }, &stages).expect("guided");
        assert_eq!(metas.len(), 2);
        assert_eq!(bell, 0, "a plain tone is not a bell");
        // the noisy stage keeps 0.25 s before it; the clean one 0.25 s of the room
        assert_eq!(metas[0].duration_ms, 6_250);
        assert_eq!(metas[1].duration_ms, 6_250);
        assert_eq!(metas[0].expected_daimoku_count, 3);
        assert_eq!(store.noise(0).unwrap(), Some(TrainingNoise::Bell));
        assert_eq!(store.noise(1).unwrap(), None);
        assert!(store.voice_band(1).unwrap().is_some());
        assert_eq!(store.speed(1).unwrap().as_deref(), Some("slow"));
        // a stage too short to hold anything is an error, nothing is added
        let short = [stage(4_000, 4_100, 3, None)];
        assert!(store.add_guided(RecordedAudio { samples: vec![0.0; 10 * sr as usize], sample_rate: sr }, &short).is_err());
    }

    #[test]
    fn clear_returns_removed_count() {
        let store = TrainingStore::new();
        store.add(fake_audio(10.0, 48000), 5);
        assert_eq!(store.clear(), 1);
        assert!(store.list().is_empty());
    }

    #[test]
    fn get_returns_stored_audio() {
        let store = TrainingStore::new();
        store.add(fake_audio(2.0, 48000), 5);
        let audio = store.get(0).expect("get failed");
        assert_eq!(audio.samples.len(), 96_000);
        assert_eq!(audio.sample_rate, 48000);
    }

    #[test]
    fn get_errors_on_missing_index() {
        let store = TrainingStore::new();
        assert!(store.get(0).is_err());
    }

    #[test]
    fn get_with_expected_returns_count() {
        let store = TrainingStore::new();
        store.add(fake_audio(1.0, 48000), 7);
        let (_, count) = store.get_with_expected(0).expect("get failed");
        assert_eq!(count, 7);
    }
}
