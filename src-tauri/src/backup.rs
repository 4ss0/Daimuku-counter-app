//! Backup and restore of everything the user has created: saved sessions,
//! daily goal, preferences and the voice-training recordings.
//!
//! A backup is one JSON file, readable by any text editor. The voice
//! recordings are embedded as base64 16-bit WAV (half the size of the
//! 32-bit float files kept on disk, with no audible difference for the
//! recogniser).

use crate::history::History;
use crate::profile::{ProfileState, TakeRecord};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;

pub const FORMAT: &str = "daimoku-counter-backup";
pub const VERSION: u32 = 1;
/// Refuse anything bigger: a normal backup is a few MB.
pub const MAX_BACKUP_BYTES: usize = 200 * 1024 * 1024;
/// A voice profile on its own (the user's recordings), to share or move
/// to another device without touching sessions and preferences.
pub const VOICE_FORMAT: &str = "daimoku-counter-voice";

#[derive(Serialize, Deserialize)]
struct BackupTake {
    #[serde(flatten)]
    record: TakeRecord,
    wav_base64: String,
}

#[derive(Serialize, Deserialize)]
struct BackupFile {
    format: String,
    version: u32,
    app_version: String,
    created_at: chrono::DateTime<chrono::Utc>,
    history: serde_json::Value,
    #[serde(default)]
    prefs: serde_json::Value,
    #[serde(default)]
    takes: Vec<BackupTake>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RestoreSummary {
    pub sessions: usize,
    pub takes: usize,
    pub takes_skipped: usize,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn wav16_bytes(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cur = Cursor::new(Vec::new());
    {
        let mut w = hound::WavWriter::new(&mut cur, spec).map_err(|e| format!("wav: {e}"))?;
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            w.write_sample(v).map_err(|e| format!("wav: {e}"))?;
        }
        w.finalize().map_err(|e| format!("wav: {e}"))?;
    }
    Ok(cur.into_inner())
}

pub fn read_prefs() -> serde_json::Value {
    crate::paths::data_dir()
        .and_then(|d| std::fs::read_to_string(d.join("prefs.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub fn write_prefs(prefs: &serde_json::Value) -> Result<(), String> {
    if !prefs.is_object() {
        return Err("prefs must be an object".to_string());
    }
    let dir = crate::paths::data_dir().ok_or_else(|| "no data directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("prefs.json.tmp");
    std::fs::write(&tmp, prefs.to_string()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join("prefs.json")).map_err(|e| e.to_string())
}

/// Builds the backup as a JSON string.
pub fn build(history: &History, profile: &ProfileState, app_version: &str) -> Result<String, String> {
    let mut takes = Vec::new();
    for (record, samples, sr) in profile.export_takes() {
        takes.push(BackupTake {
            record,
            wav_base64: b64().encode(wav16_bytes(&samples, sr)?),
        });
    }
    let file = BackupFile {
        format: FORMAT.to_string(),
        version: VERSION,
        app_version: app_version.to_string(),
        created_at: chrono::Utc::now(),
        history: history.to_backup(),
        prefs: read_prefs(),
        takes,
    };
    serde_json::to_string(&file).map_err(|e| format!("serialize: {e}"))
}

/// Restores a backup, replacing the current data. Everything is validated
/// before anything is overwritten.
#[derive(Serialize, Deserialize)]
struct VoiceFile {
    format: String,
    version: u32,
    app_version: String,
    created_at: chrono::DateTime<chrono::Utc>,
    takes: Vec<BackupTake>,
}

fn encode_takes(profile: &ProfileState) -> Result<Vec<BackupTake>, String> {
    let mut takes = Vec::new();
    for (record, samples, sr) in profile.export_takes() {
        takes.push(BackupTake {
            record,
            wav_base64: b64().encode(wav16_bytes(&samples, sr)?),
        });
    }
    Ok(takes)
}

fn decode_takes(takes: Vec<BackupTake>) -> (Vec<(TakeRecord, Vec<f32>, u32)>, usize) {
    let mut out = Vec::new();
    let mut skipped = 0;
    for t in takes {
        match b64()
            .decode(t.wav_base64.as_bytes())
            .ok()
            .and_then(|b| crate::base::decode_wav(&b).ok())
        {
            Some((samples, sr)) => out.push((t.record, samples, sr)),
            None => skipped += 1,
        }
    }
    (out, skipped)
}

/// The voice profile alone, as JSON.
pub fn build_voice(profile: &ProfileState, app_version: &str) -> Result<String, String> {
    let file = VoiceFile {
        format: VOICE_FORMAT.to_string(),
        version: VERSION,
        app_version: app_version.to_string(),
        created_at: chrono::Utc::now(),
        takes: encode_takes(profile)?,
    };
    serde_json::to_string(&file).map_err(|e| format!("serialize: {e}"))
}

/// Replaces the voice recordings with those of a voice file (or of a full
/// backup); sessions and preferences are left alone.
pub fn restore_voice(content: &str, profile: &ProfileState) -> Result<RestoreSummary, String> {
    if content.len() > MAX_BACKUP_BYTES {
        return Err("voice-invalid: file too large".to_string());
    }
    let v: serde_json::Value = serde_json::from_str(content.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("voice-invalid: {e}"))?;
    let format = v.get("format").and_then(|f| f.as_str()).unwrap_or("");
    if format != VOICE_FORMAT && format != FORMAT {
        return Err("voice-invalid: not a Daimoku Counter voice".to_string());
    }
    if v.get("version").and_then(|x| x.as_u64()).unwrap_or(0) > VERSION as u64 {
        return Err("backup-newer: made by a newer version of the app".to_string());
    }
    let takes: Vec<BackupTake> = match v.get("takes") {
        Some(t) => serde_json::from_value(t.clone()).map_err(|e| format!("voice-invalid: {e}"))?,
        None => Vec::new(),
    };
    if takes.is_empty() {
        return Err("voice-invalid: no recordings".to_string());
    }
    let (takes, skipped) = decode_takes(takes);
    let n_in = takes.len();
    let kept = profile.replace_takes(takes)?;
    Ok(RestoreSummary { sessions: 0, takes: kept, takes_skipped: skipped + (n_in - kept) })
}

pub fn restore(content: &str, history: &History, profile: &ProfileState) -> Result<RestoreSummary, String> {
    if content.len() > MAX_BACKUP_BYTES {
        return Err("backup-invalid: file too large".to_string());
    }
    let file: BackupFile = serde_json::from_str(content.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("backup-invalid: {e}"))?;
    if file.format != FORMAT {
        return Err("backup-invalid: not a Daimoku Counter backup".to_string());
    }
    if file.version > VERSION {
        return Err("backup-newer: made by a newer version of the app".to_string());
    }
    // decode all audio first
    let mut takes = Vec::new();
    let mut skipped = 0;
    for t in file.takes {
        match b64()
            .decode(t.wav_base64.as_bytes())
            .ok()
            .and_then(|b| crate::base::decode_wav(&b).ok())
        {
            Some((samples, sr)) => takes.push((t.record, samples, sr)),
            None => skipped += 1,
        }
    }
    let n_in = takes.len();
    // history is validated by the parse inside restore_backup; do it first
    // so a malformed backup changes nothing
    let sessions = history.restore_backup(file.history)?;
    if file.prefs.is_object() {
        write_prefs(&file.prefs)?;
    }
    let kept = profile.replace_takes(takes)?;
    Ok(RestoreSummary {
        sessions,
        takes: kept,
        takes_skipped: skipped + (n_in - kept),
    })
}

/// A file name that is safe on every platform.
pub fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect();
    let s = s.trim_matches('.').to_string();
    if s.is_empty() { "export".to_string() } else { s.chars().take(80).collect() }
}

/// Writes an export file where the user can find it (desktop) or where the
/// Android side can pick it up to save/share it (mobile).
pub fn write_export(name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = crate::paths::export_dir().ok_or_else(|| "cannot determine output directory".to_string())?;
    let path = dir.join(safe_name(name));
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_names() {
        assert_eq!(safe_name("daimoku-backup-2026-09-28.json"), "daimoku-backup-2026-09-28.json");
        assert_eq!(safe_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(safe_name(""), "export");
    }

    #[test]
    fn wav16_roundtrip() {
        let samples: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
        let bytes = wav16_bytes(&samples, 16000).unwrap();
        let (back, sr) = crate::base::decode_wav(&bytes).unwrap();
        assert_eq!(sr, 16000);
        assert_eq!(back.len(), samples.len());
        let err = samples.iter().zip(&back).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
        assert!(err < 1e-3, "max error {err}");
    }

    #[test]
    fn rejects_foreign_json() {
        let h = History::load_from(None);
        let p = ProfileState::empty();
        let e = restore("{\"hello\":1}", &h, &p).unwrap_err();
        assert!(e.starts_with("backup-invalid"), "{e}");
        let e = restore("not json", &h, &p).unwrap_err();
        assert!(e.starts_with("backup-invalid"), "{e}");
    }

    #[test]
    fn full_roundtrip() {
        let _guard = crate::profile::ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!("daimuku-test-bk-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let _ = std::fs::remove_dir_all(&tmp);

        let h = History::load_from(Some(tmp.join("h.json")));
        h.add_manual(108).unwrap();
        h.add_manual(52).unwrap();
        h.set_daily_goal(300).unwrap();
        write_prefs(&serde_json::json!({"lang":"ja","theme":"light"})).unwrap();
        let p = ProfileState::new();
        let (samples, sr) = crate::base::decode_wav(crate::base::BASE_CLIPS[1].wav).unwrap();
        p.add_take(&samples, sr, 1).unwrap();
        p.add_take(&samples, sr, 1).unwrap();

        let json = build(&h, &p, "0.2.0").unwrap();

        // wipe everything, then restore
        let h2 = History::load_from(Some(tmp.join("h2.json")));
        p.clear().unwrap();
        write_prefs(&serde_json::json!({})).unwrap();
        let sum = restore(&json, &h2, &p).unwrap();
        assert_eq!(sum, RestoreSummary { sessions: 2, takes: 2, takes_skipped: 0 });
        assert_eq!(h2.list().iter().map(|s| s.count).sum::<u32>(), 160);
        assert_eq!(h2.daily_goal(), 300);
        assert_eq!(read_prefs()["lang"], "ja");
        assert_eq!(p.snapshot().takes.len(), 2);
        // new sessions get fresh ids after a restore
        let s = h2.add_manual(1).unwrap();
        assert!(h2.list().iter().filter(|x| x.id == s.id).count() == 1);
        // a reload from disk sees the restored takes
        let p2 = ProfileState::new();
        assert_eq!(p2.snapshot().takes.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
