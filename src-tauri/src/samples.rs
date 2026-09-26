use crate::audio::RecordedAudio;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct SampleMeta {
    pub index: usize,
    pub samples_count: usize,
    pub duration_ms: u64,
    pub sample_rate: u32,
}

pub struct SampleStore {
    samples: Mutex<Vec<RecordedAudio>>,
}

impl SampleStore {
    pub fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::new()),
        }
    }

    pub fn add(&self, audio: RecordedAudio) -> SampleMeta {
        let mut samples = self.samples.lock().unwrap();
        let index = samples.len();
        let meta = meta_for(index, &audio);
        samples.push(audio);
        meta
    }

    pub fn list(&self) -> Vec<SampleMeta> {
        self.samples
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, a)| meta_for(i, a))
            .collect()
    }

    pub fn clear(&self) -> usize {
        let mut samples = self.samples.lock().unwrap();
        let count = samples.len();
        samples.clear();
        count
    }

    pub fn count(&self) -> usize {
        self.samples.lock().unwrap().len()
    }
}

fn meta_for(index: usize, audio: &RecordedAudio) -> SampleMeta {
    let samples_count = audio.samples.len();
    let duration_ms = if audio.sample_rate > 0 {
        (samples_count as u64 * 1000) / audio.sample_rate as u64
    } else {
        0
    };
    SampleMeta {
        index,
        samples_count,
        duration_ms,
        sample_rate: audio.sample_rate,
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
    fn add_and_list_returns_correct_metadata() {
        let store = SampleStore::new();
        store.add(fake_audio(1.0, 48000));
        store.add(fake_audio(2.5, 48000));

        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].index, 0);
        assert_eq!(list[0].duration_ms, 1000);
        assert_eq!(list[1].index, 1);
        assert_eq!(list[1].duration_ms, 2500);
    }

    #[test]
    fn clear_returns_removed_count() {
        let store = SampleStore::new();
        store.add(fake_audio(1.0, 48000));
        store.add(fake_audio(1.0, 48000));

        assert_eq!(store.clear(), 2);
        assert_eq!(store.count(), 0);
        assert!(store.list().is_empty());
    }

    #[test]
    fn count_reflects_additions() {
        let store = SampleStore::new();
        assert_eq!(store.count(), 0);
        store.add(fake_audio(1.0, 48000));
        assert_eq!(store.count(), 1);
        store.add(fake_audio(1.0, 48000));
        assert_eq!(store.count(), 2);
    }
}