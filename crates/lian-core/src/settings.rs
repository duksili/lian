use rusqlite::Connection;
use serde_json::{json, Value};

use crate::util::now_rfc3339;
use crate::Result;

/// Defaults are editable at any time from Settings; onboarding pre-fills them.
pub fn defaults() -> Value {
    json!({
        "onboarding_complete": false,
        "timezone": "UTC",
        "quiet_hours_start": "21:30",
        "quiet_hours_end": "07:30",
        "notifications_paused": false,
        "notifications_pause_until": null,
        "lock_screen_minimal": true,
        "baseline_start": null,
        "baseline_weeks": 5,
        "close_to_tray": true,
        "backup_dir": null,
        "last_backup_at": null,
        "assessment_input_method": "keyboard_spacebar",
        "association_min_pairs": 14,
        "theme": "dawn"
    })
}

pub fn get_all(conn: &Connection) -> Result<Value> {
    let mut out = defaults();
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let raw: String = row.get(1)?;
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            out[key] = v;
        }
    }
    Ok(out)
}

pub fn get(conn: &Connection, key: &str) -> Result<Value> {
    Ok(get_all(conn)?[key].clone())
}

pub fn get_string(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(get(conn, key)?.as_str().map(|s| s.to_string()))
}

pub fn timezone(conn: &Connection) -> Result<String> {
    Ok(get_string(conn, "timezone")?.unwrap_or_else(|| "UTC".into()))
}

pub fn set(conn: &Connection, key: &str, value: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
        rusqlite::params![key, serde_json::to_string(value)?, now_rfc3339()],
    )?;
    Ok(())
}

pub fn set_many(conn: &Connection, patch: &Value) -> Result<Value> {
    if let Some(obj) = patch.as_object() {
        for (k, v) in obj {
            set(conn, k, v)?;
        }
    }
    get_all(conn)
}
