use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{self, File, Metadata};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RECENT: Duration = Duration::from_secs(30 * 60);
const BADGE_MS: u64 = 25_000;
const TAIL_BYTES: u64 = 2 * 1024 * 1024;
const INITIAL_TAIL_BYTES: u64 = 32 * 1024 * 1024;
const HISTORY_SIZE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Running,
    Succeeded,
    Interrupted,
    Failed,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Turn {
    phase: Phase,
    at: u64,
}

#[derive(Debug, Serialize)]
struct Completion {
    id: String,
    at: u64,
    message: String,
}

#[derive(Default)]
struct Updates {
    scanned: u64,
    turn: Option<Turn>,
    completions: Vec<Completion>,
}

struct Rollout {
    modified: SystemTime,
    len: u64,
    subagent: bool,
    updates: Updates,
}

pub struct Monitor {
    sessions: PathBuf,
    cached: HashMap<PathBuf, Rollout>,
}

impl Monitor {
    pub fn new() -> Self {
        let codex_home = std::env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map_or_else(|| crate::home::home().join(".codex"), PathBuf::from);
        Self {
            sessions: codex_home.join("sessions"),
            cached: HashMap::new(),
        }
    }

    pub fn poll(&mut self, now: SystemTime, runtime_started: Option<u64>) -> Value {
        let now_ms = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut previous = std::mem::take(&mut self.cached);
        let mut running = false;
        let mut last_result: Option<Turn> = None;
        visit_rollouts(&self.sessions, 3, &mut |path, metadata| {
            let Ok(modified) = metadata.modified() else {
                return;
            };
            if now.duration_since(modified).unwrap_or_default() >= RECENT {
                return;
            }
            let old = previous.remove(&path);
            let len = metadata.len();
            let rollout = match old {
                Some(old) if old.modified == modified && old.len == len => old,
                old => {
                    let Ok(mut file) = File::open(&path) else {
                        return;
                    };
                    let old = old.filter(|old| old.len < len);
                    let subagent = old
                        .as_ref()
                        .map_or_else(|| is_subagent(&mut file), |old| old.subagent);
                    let mut updates = old.map_or_else(Updates::default, |old| old.updates);
                    if !subagent && updates.read(&mut file, len).is_err() {
                        return;
                    }
                    Rollout {
                        modified,
                        len,
                        subagent,
                        updates,
                    }
                }
            };
            if let Some(turn) = rollout.updates.turn {
                if turn.phase == Phase::Running {
                    running |= runtime_started.is_some_and(|started| turn.at / 1000 + 5 >= started);
                } else if now_ms.saturating_sub(turn.at) < RECENT.as_millis() as u64
                    && last_result.is_none_or(|last| (turn.at, turn.phase) > (last.at, last.phase))
                {
                    last_result = Some(turn);
                }
            }
            self.cached.insert(path, rollout);
        });

        let badge = last_result.filter(|turn| now_ms.saturating_sub(turn.at) < BADGE_MS);
        let idle = if runtime_started.is_some() {
            "idle"
        } else {
            "offline"
        };
        let status = |result: Option<Turn>| {
            if running {
                "running"
            } else {
                result.map_or(idle, |turn| turn.phase.label())
            }
        };
        let mut completions: Vec<_> = self
            .cached
            .values()
            .flat_map(|rollout| &rollout.updates.completions)
            .filter(|item| now_ms.saturating_sub(item.at) < RECENT.as_millis() as u64)
            .collect();
        completions.sort_unstable_by(|a, b| (b.at, &b.id).cmp(&(a.at, &a.id)));
        completions.dedup_by(|a, b| a.id == b.id);
        completions.truncate(HISTORY_SIZE);
        json!({ "status": status(badge), "detail": status(last_result), "completions": completions })
    }
}

fn visit_rollouts(dir: &Path, depth: u8, visit: &mut impl FnMut(PathBuf, Metadata)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() && depth > 0 {
            visit_rollouts(&entry.path(), depth - 1, visit);
        } else if kind.is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
            && let Ok(metadata) = entry.metadata()
        {
            visit(entry.path(), metadata);
        }
    }
}

fn is_subagent(file: &mut File) -> bool {
    let mut header = Vec::new();
    if BufReader::new(file.take(TAIL_BYTES))
        .read_until(b'\n', &mut header)
        .is_err()
    {
        return false;
    }
    let Ok(record) = serde_json::from_slice::<Value>(&header) else {
        return false;
    };
    record["type"] == "session_meta" && record["payload"]["source"].get("subagent").is_some()
}

impl Updates {
    fn read(&mut self, file: &mut File, len: u64) -> std::io::Result<()> {
        // On launch, a running turn's start can be behind megabytes of tool results.
        // Once caught up, only the unprocessed tail needs reading.
        let limit = if self.scanned == 0 {
            INITIAL_TAIL_BYTES
        } else {
            TAIL_BYTES
        };
        let start = self.scanned.max(len.saturating_sub(limit));
        file.seek(SeekFrom::Start(start))?;
        let mut tail = Vec::new();
        file.take(len - start).read_to_end(&mut tail)?;
        let Some(end) = tail.iter().rposition(|byte| *byte == b'\n') else {
            return Ok(());
        };
        // Leave any incomplete record for the next poll. The offset always lands on a line boundary.
        let mut lines = tail[..end].split(|byte| *byte == b'\n');
        if start > self.scanned {
            lines.next(); // The tail may begin inside a record.
        }
        for (turn, completion) in lines.filter_map(parse_turn) {
            self.turn = Some(turn);
            if let Some(completion) = completion {
                self.completions.push(completion);
                if self.completions.len() > HISTORY_SIZE {
                    self.completions.remove(0);
                }
            }
        }
        self.scanned = start + end as u64 + 1;
        Ok(())
    }
}

fn parse_turn(line: &[u8]) -> Option<(Turn, Option<Completion>)> {
    #[derive(Deserialize)]
    struct Record<'a> {
        timestamp: &'a str,
        #[serde(rename = "type")]
        kind: &'a str,
        #[serde(borrow)]
        payload: Event<'a>,
    }
    #[derive(Deserialize)]
    struct Event<'a> {
        #[serde(rename = "type")]
        kind: &'a str,
        error: Option<serde::de::IgnoredAny>,
        turn_id: Option<&'a str>,
        #[serde(borrow)]
        last_agent_message: Option<Cow<'a, str>>,
    }

    let record: Record<'_> = serde_json::from_slice(line).ok()?;
    if record.kind != "event_msg" {
        return None;
    }
    let has_error = record.payload.error.is_some();
    let phase = match record.payload.kind {
        "task_started" | "turn_started" => Phase::Running,
        "task_complete" | "turn_completed" if has_error => Phase::Failed,
        "task_complete" | "turn_completed" => Phase::Succeeded,
        "turn_aborted" if has_error => Phase::Failed,
        "turn_aborted" => Phase::Interrupted,
        _ => return None,
    };
    let at = chrono::DateTime::parse_from_rfc3339(record.timestamp)
        .ok()?
        .timestamp_millis()
        .try_into()
        .ok()?;
    let completion = (phase == Phase::Succeeded).then(|| Completion {
        id: record.payload.turn_id.unwrap_or(record.timestamp).into(),
        at,
        message: record
            .payload
            .last_agent_message
            .as_deref()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or_default()
            .chars()
            .take(96)
            .collect(),
    });
    Some((Turn { phase, at }, completion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn recovers_partial_writes_and_tracks_concurrent_turns() -> std::io::Result<()> {
        let path = std::env::temp_dir().join(format!(
            "nook-codex-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path)?;
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(path.clone());
        let mut monitor = Monitor {
            sessions: path.clone(),
            cached: HashMap::new(),
        };
        let mut a = File::create(path.join("rollout-a.jsonl"))?;
        let mut b = File::create(path.join("rollout-b.jsonl"))?;
        let now = SystemTime::now();
        let timestamp = chrono::DateTime::<chrono::Utc>::from(now).to_rfc3339();
        let runtime = Some(
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        let event = |kind, error| {
            json!({
                "type": "event_msg",
                "timestamp": timestamp,
                "payload": { "type": kind, "error": error },
            })
        };

        assert_eq!(monitor.poll(now, None)["status"], "offline");
        assert_eq!(monitor.poll(now, runtime)["status"], "idle");
        writeln!(a, "{}", event("task_started", Value::Null))?;
        write!(b, "{}", event("task_started", Value::Null))?;
        assert_eq!(monitor.poll(now, runtime)["status"], "running");
        assert!(
            monitor.cached[&path.join("rollout-b.jsonl")]
                .updates
                .turn
                .is_none()
        );
        writeln!(b)?;
        writeln!(a, "{}", event("task_complete", Value::Null))?;
        assert_eq!(monitor.poll(now, runtime)["status"], "running");
        writeln!(
            b,
            "{}",
            event("task_complete", json!({ "message": "failed" }))
        )?;
        assert_eq!(monitor.poll(now, runtime)["status"], "failed");
        let expired = monitor.poll(now + Duration::from_secs(26), runtime);
        assert_eq!(expired["status"], "idle");
        assert_eq!(expired["detail"], "failed");
        assert_eq!(expired["completions"].as_array().unwrap().len(), 1);
        assert_eq!(
            parse_turn(event("turn_aborted", Value::Null).to_string().as_bytes())
                .unwrap()
                .0
                .phase,
            Phase::Interrupted
        );

        writeln!(a, "{}", event("task_started", Value::Null))?;
        writeln!(
            a,
            "{{\"type\":\"response_item\",\"payload\":\"{}\"}}",
            "x".repeat(TAIL_BYTES as usize + 10)
        )?;
        let mut restarted = Monitor {
            sessions: path,
            cached: HashMap::new(),
        };
        assert_eq!(restarted.poll(now, runtime)["status"], "running");
        Ok(())
    }
}
