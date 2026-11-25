//! MAP records model turns in its local SQLite store. Read only lifecycle metadata.

use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

pub const BUNDLE_ID: &str = "com.xiaolinhaha.map";
const BADGE_MS: u64 = 25_000;
const HISTORY_MS: u64 = 30 * 60_000;

const TRACKED: &str = "
    WITH tracked AS (
        SELECT id, updated_at, lower(substr(model, instr(model, ':') + 1)) AS model
        FROM sessions WHERE archived = 0 AND is_draft = 0
          AND harness IN ('codex', 'antigravity')
    ), models AS (
        SELECT *, CASE WHEN model LIKE 'gpt-%' THEN 'GPT' ELSE 'Gemini' END AS family
        FROM tracked WHERE model LIKE 'gpt-%' OR model LIKE 'gemini-%'
    )";

pub fn read(started: Option<u64>, now: u64) -> Value {
    let path = crate::home::home()
        .join("Library/Application Support")
        .join(BUNDLE_ID)
        .join("monocode.db");
    read_store(&path, started, now).unwrap_or_else(|_| {
        if started.is_some() {
            json!({ "running": true, "models": [], "completions": [], "sessions": [] })
        } else {
            Value::Null
        }
    })
}

fn read_store(path: &Path, started: Option<u64>, now: u64) -> rusqlite::Result<Value> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    db.busy_timeout(Duration::from_millis(100))?;
    let sql = format!(
        "{TRACKED}
        SELECT m.family, m.model, e.event_type, e.created_at,
            CASE WHEN json_valid(e.payload_json) THEN json_extract(e.payload_json, '$.outcome') END
        FROM models m LEFT JOIN session_events e ON e.session_id = m.id AND e.sequence = (
            SELECT sequence FROM session_events WHERE session_id = m.id
                AND event_type IN ('turn.started', 'turn.ended') ORDER BY sequence DESC LIMIT 1)
        ORDER BY m.updated_at DESC, m.id"
    );
    let mut query = db.prepare(&sql)?;
    let mut rows = query.query([])?;
    let resting = if started.is_some() { "idle" } else { "offline" };
    let mut selected = [None, None];
    while let Some(row) = rows.next()? {
        let family: String = row.get(0)?;
        let model: String = row.get(1)?;
        let kind: Option<String> = row.get(2)?;
        let at = row.get::<_, Option<u64>>(3)?.unwrap_or_default();
        let outcome: Option<String> = row.get(4)?;
        let state = match kind.as_deref() {
            // Discard abandoned starts from a previous MAP process.
            Some("turn.started") if started.is_some_and(|launch| at >= launch) => "running",
            Some("turn.ended") if now.saturating_sub(at) < BADGE_MS => match outcome.as_deref() {
                Some("completed") => "succeeded",
                Some("failed") => "failed",
                Some("cancelled") => "interrupted",
                _ => resting,
            },
            _ => resting,
        };
        let rank = match state {
            "running" => (2, at),
            "idle" | "offline" => (0, 0),
            _ => (1, at),
        };
        let slot = &mut selected[usize::from(family == "Gemini")];
        if slot.as_ref().is_none_or(|(previous, _)| rank > *previous) {
            *slot = Some((
                rank,
                json!({ "family": family, "model": model, "status": state, "at": at }),
            ));
        }
    }
    let models: Vec<_> = selected
        .into_iter()
        .flatten()
        .map(|(_, model)| model)
        .collect();

    let sql = format!("{TRACKED}
        SELECT m.id, e.sequence, e.created_at, m.model
        FROM models m JOIN session_events e ON e.session_id = m.id
        WHERE e.event_type = 'turn.ended' AND e.created_at >= ?1
          AND CASE WHEN json_valid(e.payload_json) THEN json_extract(e.payload_json, '$.outcome') END = 'completed'
        ORDER BY e.created_at DESC, m.id, e.sequence DESC LIMIT 3");
    let mut query = db.prepare(&sql)?;
    let completions = query
        .query_map([now.saturating_sub(HISTORY_MS)], |row| {
            let session: String = row.get(0)?;
            let sequence: u64 = row.get(1)?;
            Ok(
                json!({ "id": format!("map/{session}/{sequence}"), "at": row.get::<_, u64>(2)?,
            "model": row.get::<_, String>(3)? }),
            )
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let sql_sessions = "
        SELECT 
            s.id,
            s.harness,
            s.model,
            s.title,
            s.cwd,
            s.updated_at,
            e.event_type,
            e.created_at,
            CASE WHEN json_valid(e.payload_json) THEN json_extract(e.payload_json, '$.outcome') END
        FROM sessions s
        LEFT JOIN session_events e ON e.session_id = s.id AND e.sequence = (
            SELECT sequence FROM session_events WHERE session_id = s.id
            AND event_type IN ('turn.started', 'turn.ended')
            ORDER BY sequence DESC LIMIT 1
        )
        WHERE s.archived = 0 AND s.is_draft = 0
        ORDER BY s.updated_at DESC
        LIMIT 10";
    let mut query = db.prepare(sql_sessions)?;
    let sessions = query
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let harness: String = row.get(1)?;
            let model_raw: String = row.get(2)?;
            let title: String = row.get(3)?;
            let cwd: String = row.get(4)?;
            let updated_at: u64 = row.get(5)?;
            let kind: Option<String> = row.get(6)?;
            let at = row.get::<_, Option<u64>>(7)?.unwrap_or_default();
            let outcome: Option<String> = row.get(8)?;

            let state = match kind.as_deref() {
                Some("turn.started") if now.saturating_sub(at) < 30 * 60_000 => "running",
                Some("turn.ended") => match outcome.as_deref() {
                    Some("completed") => "completed",
                    Some("failed") => "failed",
                    Some("cancelled") => "interrupted",
                    _ => resting,
                },
                _ => resting,
            };

            let model = model_raw.split(':').nth(1).unwrap_or(&model_raw).to_string();

            Ok(json!({
                "id": id,
                "harness": harness,
                "model": model,
                "title": title,
                "cwd": cwd,
                "updated_at": updated_at,
                "status": state,
                "at": at,
                "outcome": outcome,
            }))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(json!({ "running": started.is_some(), "models": models, "completions": completions, "sessions": sessions }))
}
