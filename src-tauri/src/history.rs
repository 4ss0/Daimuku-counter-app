//! Saved chanting sessions (what the statistics are computed from) and
//! the user's daily goal. One small JSON file, written atomically.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The session of the day a record belongs to: chosen on the Count
/// screen (to continue the morning or the evening session) or when it is
/// added by hand. Records without it go by their start time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Half {
    Morning,
    Evening,
}

impl Half {
    pub fn parse(s: &str) -> Result<Half, String> {
        match s {
            "morning" => Ok(Half::Morning),
            "evening" => Ok(Half::Evening),
            _ => Err(format!("unknown session {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionRecord {
    pub id: u64,
    #[serde(default)]
    pub running: bool,
    pub started_at: DateTime<Utc>,
    pub duration_secs: f32,
    /// Daimoku credited for the session (the user may have corrected it).
    pub count: u32,
    /// What the counter detected, kept for reference.
    pub detected: u32,
    /// Added by hand, without the counter.
    #[serde(default)]
    pub manual: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half: Option<Half>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    next_id: u64,
    daily_goal: u32,
    sessions: Vec<SessionRecord>,
    #[serde(skip)]
    active: Option<u64>,
}

impl Default for HistoryFile {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            daily_goal: DEFAULT_DAILY_GOAL,
            sessions: Vec::new(),
            active: None,
        }
    }
}

pub const DEFAULT_DAILY_GOAL: u32 = 100;
const MAX_COUNT: u32 = 1_000_000;
/// A session added by hand lasts at most a day.
const MAX_MANUAL_SECS: f32 = 86_400.0;

#[derive(Clone)]
pub struct History {
    path: Option<PathBuf>,
    inner: Arc<Mutex<HistoryFile>>,
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
            inner: Arc::new(Mutex::new(data)),
        }
    }

    pub fn list(&self) -> Vec<SessionRecord> {
        self.inner.lock().unwrap().sessions.iter()
            .filter(|s| !s.running || s.count > 0).cloned().collect()
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
        half: Option<Half>,
    ) -> Result<SessionRecord, String> {
        let mut g = self.inner.lock().unwrap();
        let rec = SessionRecord {
            id: g.next_id,
            running: false,
            started_at,
            duration_secs: duration_secs.max(0.0),
            count: count.min(MAX_COUNT),
            detected: detected.min(MAX_COUNT),
            manual,
            half,
        };
        g.next_id += 1;
        g.sessions.push(rec.clone());
        self.persist(&g)?;
        Ok(rec)
    }

    /// Reserve one durable record; a restart recovers it as a finished session.
    pub fn begin_live(&self, half: Option<Half>) -> Result<SessionRecord, String> {
        let mut g = self.inner.lock().unwrap();
        if g.active.is_some() { return Err("a session is still active".into()); }
        let mut next = g.clone();
        let rec = SessionRecord { id: next.next_id, running: true, started_at: Utc::now(),
            duration_secs: 0.0, count: 0, detected: 0, manual: false, half };
        next.next_id += 1;
        next.active = Some(rec.id);
        next.sessions.push(rec.clone());
        self.persist(&next)?;
        *g = next;
        Ok(rec)
    }

    pub fn checkpoint_live(&self, id: u64, secs: f32, count: u32, finished: bool) -> Result<Option<SessionRecord>, String> {
        let mut g = self.inner.lock().unwrap();
        if g.active != Some(id) { return Err("session is not active".into()); }
        let mut next = g.clone();
        let rec = next.sessions.iter_mut().find(|s| s.id == id).ok_or("active session missing")?;
        rec.duration_secs = secs.max(0.0);
        rec.count = count.min(MAX_COUNT);
        rec.detected = rec.count;
        rec.running = !finished;
        let result = if finished && count == 0 { None } else { Some(rec.clone()) };
        if finished {
            next.active = None;
            if count == 0 { next.sessions.retain(|s| s.id != id); }
        }
        self.persist(&next)?;
        *g = next;
        Ok(result)
    }

    /// Adds `count` Daimoku chanted without the counter, now.
    pub fn add_manual(&self, count: u32) -> Result<SessionRecord, String> {
        self.add_manual_at(count, None, 0.0, None)
    }

    /// Adds `count` Daimoku chanted without the counter, started at
    /// `started_at` (now if `None`; never in the future) and lasting
    /// `duration_secs` (0 if not known), for the session `half` of its day
    /// (`None`: by its start time).
    pub fn add_manual_at(
        &self,
        count: u32,
        started_at: Option<DateTime<Utc>>,
        duration_secs: f32,
        half: Option<Half>,
    ) -> Result<SessionRecord, String> {
        if count == 0 {
            return Err("the count must be at least 1".to_string());
        }
        let now = Utc::now();
        let at = started_at.unwrap_or(now);
        if at > now + Duration::minutes(1) {
            return Err("the session cannot be in the future".to_string());
        }
        let secs = if duration_secs.is_finite() { duration_secs.clamp(0.0, MAX_MANUAL_SECS) } else { 0.0 };
        self.add(at, secs, count, 0, true, half)
    }

    pub fn set_count(&self, id: u64, count: u32) -> Result<SessionRecord, String> {
        let mut g = self.inner.lock().unwrap();
        if g.active == Some(id) { return Err("stop the session before editing it".into()); }
        let rec = g
            .sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("session {id} not found"))?;
        rec.count = count.min(MAX_COUNT);
        let out = rec.clone();
        self.persist(&g)?;
        Ok(out)
    }

    pub fn delete(&self, id: u64) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        if g.active == Some(id) { return Err("stop the session before deleting it".into()); }
        let before = g.sessions.len();
        g.sessions.retain(|s| s.id != id);
        if g.sessions.len() == before {
            return Err(format!("session {id} not found"));
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
        if g.active.is_some() { return Err("stop the session before restoring a backup".into()); }
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
    h.active = None;
    h.sessions.retain(|s| !s.running || s.count > 0);
    for s in &mut h.sessions { s.running = false; }
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
        let a = h.add(Utc::now(), 60.0, 50, 50, false, None).unwrap();
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
    fn live_checkpoint_survives_restart_without_double_credit() {
        let path = tmp("live-recovery");
        let history = History::load_from(Some(path.clone()));
        let live = history.begin_live(Some(Half::Evening)).unwrap();
        history.checkpoint_live(live.id, 10.0, 8, false).unwrap();
        // Simulate process death: no Stop, no cleanup, just reload disk.
        let recovered = History::load_from(Some(path.clone()));
        let records = recovered.list();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].count, 8);
        assert_eq!(records[0].half, Some(Half::Evening));
        assert!(!records[0].running);
        // The still-running original process updates the SAME record at Stop.
        history.clone().checkpoint_live(live.id, 15.0, 12, true).unwrap();
        let records = History::load_from(Some(path)).list();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, live.id);
        assert_eq!(records[0].count, 12);
        assert_eq!(records[0].detected, 12);
        assert!(!records[0].running);
        assert!(history.set_count(live.id, 13).is_ok());
    }

    #[test]
    fn live_session_cannot_be_edited_deleted_or_replaced() {
        let history = History::load_from(None);
        let backup = history.to_backup();
        let live = history.begin_live(None).unwrap();
        assert!(history.begin_live(None).is_err());
        assert!(history.set_count(live.id, 99).is_err());
        assert!(history.delete(live.id).is_err());
        assert!(history.restore_backup(backup.clone()).is_err());
        // Other records remain editable during counting.
        let other = history.add_manual(3).unwrap();
        history.set_count(other.id, 4).unwrap();
        history.checkpoint_live(live.id, 2.0, 1, true).unwrap();
        history.delete(live.id).unwrap();
        history.restore_backup(backup).unwrap();
    }

    #[test]
    fn empty_live_sessions_are_not_kept_after_stop_or_restart() {
        let path = tmp("live-empty");
        let history = History::load_from(Some(path.clone()));
        let live = history.begin_live(None).unwrap();
        assert!(History::load_from(Some(path)).list().is_empty());
        assert!(history.checkpoint_live(live.id, 10.0, 0, true).unwrap().is_none());
        assert!(history.list().is_empty());
        assert!(history.begin_live(None).unwrap().id > live.id);
    }

    #[test]
    fn failed_checkpoint_keeps_previous_record_and_can_be_retried() {
        let path = tmp("live-failure");
        let history = History::load_from(Some(path.clone()));
        let live = history.begin_live(None).unwrap();
        history.checkpoint_live(live.id, 3.0, 2, false).unwrap();
        // A directory at the temporary-file path makes the next write fail.
        let blocked = path.with_extension("json.tmp");
        fs::create_dir(&blocked).unwrap();
        assert!(history.checkpoint_live(live.id, 5.0, 4, true).is_err());
        assert_eq!(history.list()[0].count, 2);
        assert!(history.list()[0].running);
        assert!(history.delete(live.id).is_err());
        fs::remove_dir(&blocked).unwrap();
        history.checkpoint_live(live.id, 5.0, 4, true).unwrap();
        assert_eq!(History::load_from(Some(path)).list()[0].count, 4);
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
    fn past_session_by_hand() {
        let h = History::load_from(None);
        let at = Utc::now() - Duration::days(3);
        let s = h.add_manual_at(500, Some(at), 1800.0, Some(Half::Evening)).unwrap();
        assert_eq!(s.started_at, at);
        assert_eq!(s.duration_secs, 1800.0);
        assert!(s.manual);
        assert_eq!(s.detected, 0);
        assert_eq!(s.half, Some(Half::Evening));
        assert!(h.add_manual_at(5, Some(Utc::now() + Duration::hours(2)), 0.0, None).is_err());
        assert_eq!(h.add_manual_at(5, None, f32::NAN, None).unwrap().duration_secs, 0.0);
        assert_eq!(h.add_manual_at(5, None, 1e9, None).unwrap().duration_secs, MAX_MANUAL_SECS);
    }

    #[test]
    fn half_is_kept_and_optional_in_the_file() {
        let p = tmp("half");
        let h = History::load_from(Some(p.clone()));
        h.add(Utc::now(), 10.0, 3, 3, false, Some(Half::Morning)).unwrap();
        h.add(Utc::now(), 10.0, 4, 4, false, None).unwrap();
        let json = fs::read_to_string(&p).unwrap();
        assert!(json.contains(r#""half":"morning""#));
        assert_eq!(json.matches("half").count(), 1, "absent when not set: {json}");
        let l = History::load_from(Some(p)).list();
        assert_eq!(l[0].half, Some(Half::Morning));
        assert_eq!(l[1].half, None);
        assert_eq!(Half::parse("evening"), Ok(Half::Evening));
        assert!(Half::parse("noon").is_err());
    }

    #[test]
    fn in_memory_when_no_folder() {
        let h = History::load_from(None);
        h.add_manual(5).unwrap();
        assert_eq!(h.list().len(), 1);
        assert!(h.add_manual(0).is_err());
    }
}
