use crate::audio::RecordedAudio;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct TrainingTakeMeta {
    pub id: usize,
    pub expected_daimoku_count: u32,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub created_at: DateTime<Utc>,
}

struct TrainingTake {
    audio: RecordedAudio,
    expected_daimoku_count: u32,
    created_at: DateTime<Utc>,
}

pub struct TrainingStore {
    takes: Mutex<Vec<TrainingTake>>,
}

impl TrainingStore {
    pub fn new() -> Self {
        Self {
            takes: Mutex::new(Vec::new()),
        }
    }

    pub fn add(&self, audio: RecordedAudio, expected_daimoku_count: u32) -> TrainingTakeMeta {
        let mut takes = self.takes.lock().unwrap();
        let id = takes.len();
        let created_at = Utc::now();
        let meta = meta_for(id, &audio, expected_daimoku_count, created_at);
        takes.push(TrainingTake {
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
