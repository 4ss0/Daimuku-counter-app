//! Saved chanting sessions (what the statistics are computed from) and
//! the user's daily goal. One small JSON file, written atomically.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionRecord {
    pub id: u64,
    pub started_at: DateTime<Utc>,
    pub duration_secs: f32,
    /// Daimoku credited for the session (the user may have corrected it).
    pub count: u32,
    /// What the counter detected, kept for reference.
    pub detected: u32,
    /// Added by hand, without the counter.
    #[serde(default)]
    pub manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    next_id: u64,
    daily_goal: u32,
    sessions: Vec<SessionRecord>,
}

impl Default for HistoryFile {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            daily_goal: DEFAULT_DAILY_GOAL,
            sessions: Vec::new(),
        }
    }
}

pub const DEFAULT_DAILY_GOAL: u32 = 100;
const MAX_COUNT: u32 = 1_000_000;

pub struct History {
    path: Option<PathBuf>,
    inner: Mutex<HistoryFile>,
}

impl History {
    /// Loads from the app data folder (see `crate::paths`).
    pub fn load() -> Self {
        Self::load_from(crate::paths::data_dir().map(|d| d.join("history.json")))
    }

    /// `None` = keep everything in memory only (no data folder available).
    pub fn load_from(path: Option<PathBuf>) -> Self {
        let data = path.as_deref().map(read_file).unwrap_or_default();
        Self {
            path,
            inner: Mutex::new(data),
        }
    }

    pub fn list(&self) -> Vec<SessionRecord> {
        self.inner.lock().unwrap().sessions.clone()
    }

    /// Records a finished session. `detected` is what the counter found,
    /// `count` what is credited (normally the same).
    pub fn add(
        &self,
        started_at: DateTime<Utc>,
        duration_secs: f32,
        count: u32,
        detected: u32,
        manual: bool,
    ) -> Result<SessionRecord, String> {
        let mut g = self.inner.lock().unwrap();
        let rec = SessionRecord {
            id: g.next_id,
            started_at,
            duration_secs: duration_secs.max(0.0),
            count: count.min(MAX_COUNT),
            detected: detected.min(MAX_COUNT),
            manual,
        };
        g.next_id += 1;
        g.sessions.push(rec.clone());
        self.persist(&g)?;
        Ok(rec)
    }

    /// Adds `count` Daimoku chanted without the counter, now.
    pub fn add_manual(&self, count: u32) -> Result<SessionRecord, String> {
        if count == 0 {
            return Err("il numero deve essere almeno 1".to_string());
        }
        self.add(Utc::now(), 0.0, count, 0, true)
    }

    pub fn set_count(&self, id: u64, count: u32) -> Result<SessionRecord, String> {
        let mut g = self.inner.lock().unwrap();
        let rec = g
            .sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("sessione {id} non trovata"))?;
        rec.count = count.min(MAX_COUNT);
        let out = rec.clone();
        self.persist(&g)?;
        Ok(out)
    }

    pub fn delete(&self, id: u64) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        let before = g.sessions.len();
        g.sessions.retain(|s| s.id != id);
        if g.sessions.len() == before {
            return Err(format!("sessione {id} non trovata"));
        }
        self.persist(&g)
    }

    pub fn daily_goal(&self) -> u32 {
        self.inner.lock().unwrap().daily_goal
    }

    pub fn set_daily_goal(&self, goal: u32) -> Result<u32, String> {
        let mut g = self.inner.lock().unwrap();
        g.daily_goal = goal.clamp(1, MAX_COUNT);
        let v = g.daily_goal;
        self.persist(&g)?;
        Ok(v)
    }

    /// The whole file (sessions + goal), for a backup.
    pub fn to_backup(&self) -> serde_json::Value {
        serde_json::to_value(&*self.inner.lock().unwrap()).unwrap_or(serde_json::Value::Null)
    }

    /// Replaces sessions and goal with those of a backup. Returns the
    /// number of sessions restored.
    pub fn restore_backup(&self, v: serde_json::Value) -> Result<usize, String> {
        let mut data: HistoryFile =
            serde_json::from_value(v).map_err(|e| format!("backup-invalid: sessions: {e}"))?;
        normalize(&mut data);
        let n = data.sessions.len();
        let mut g = self.inner.lock().unwrap();
        self.persist(&data)?;
        *g = data;
        Ok(n)
    }

    fn persist(&self, data: &HistoryFile) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string(data).map_err(|e| format!("serialize: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| format!("write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
    }
}

fn normalize(h: &mut HistoryFile) {
    // never reuse an id, even if the file was edited by hand
    let max_id = h.sessions.iter().map(|s| s.id).max().unwrap_or(0);
    h.next_id = h.next_id.max(max_id + 1);
    if h.daily_goal == 0 {
        h.daily_goal = DEFAULT_DAILY_GOAL;
    }
    h.daily_goal = h.daily_goal.min(MAX_COUNT);
    for s in &mut h.sessions {
        s.count = s.count.min(MAX_COUNT);
    }
}

fn read_file(path: &Path) -> HistoryFile {
    match fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<HistoryFile>(&s) {
            Ok(mut h) => {
                normalize(&mut h);
                h
            }
            Err(e) => {
                eprintln!("[history] cannot parse {}: {e} - kept as .bak", path.display());
                let _ = fs::rename(path, path.with_extension("json.bak"));
                HistoryFile::default()
            }
        },
        Err(_) => HistoryFile::default(),
    }
}

/// Start time of a session that has just ended after `duration_secs`.
pub fn started_before_now(duration_secs: f32) -> DateTime<Utc> {
    Utc::now() - Duration::milliseconds((duration_secs.max(0.0) * 1000.0) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("daimuku-hist-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d.join("history.json")
    }

    #[test]
    fn add_edit_delete_and_reload() {
        let p = tmp("crud");
        let h = History::load_from(Some(p.clone()));
        let a = h.add(Utc::now(), 60.0, 50, 50, false).unwrap();
        let b = h.add_manual(20).unwrap();
        assert_ne!(a.id, b.id);
        h.set_count(a.id, 53).unwrap();
        h.set_daily_goal(300).unwrap();

        let h2 = History::load_from(Some(p.clone()));
        let l = h2.list();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].count, 53);
        assert_eq!(l[0].detected, 50);
        assert!(l[1].manual);
        assert_eq!(h2.daily_goal(), 300);

        h2.delete(a.id).unwrap();
        assert!(h2.delete(a.id).is_err());
        let c = h2.add_manual(1).unwrap();
        assert!(c.id > b.id, "ids are never reused");
        assert_eq!(History::load_from(Some(p)).list().len(), 2);
    }

    #[test]
    fn corrupt_file_is_set_aside() {
        let p = tmp("corrupt");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "{ not json").unwrap();
        let h = History::load_from(Some(p.clone()));
        assert!(h.list().is_empty());
        assert_eq!(h.daily_goal(), DEFAULT_DAILY_GOAL);
        assert!(p.with_extension("json.bak").exists());
    }

    #[test]
    fn in_memory_when_no_folder() {
        let h = History::load_from(None);
        h.add_manual(5).unwrap();
        assert_eq!(h.list().len(), 1);
        assert!(h.add_manual(0).is_err());
    }
}
