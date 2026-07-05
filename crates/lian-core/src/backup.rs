//! Backup, restore, export, and permanent deletion.
//!
//! Backups: consistent SQLite snapshot via `VACUUM INTO`, plus a manifest
//! (created time, app version, schema version, SHA-256). Restore keeps a
//! safety copy of current data before replacing anything. Exports: CSV per
//! domain table with a data dictionary, raw assessment trials included, and
//! an analysis-friendly full SQLite copy.

use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::{new_id, now_rfc3339};
use crate::{db, settings, Error, Result, APP_VERSION};

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn stamp() -> String {
    chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn create_backup(conn: &Connection, dest_dir: &str) -> Result<Value> {
    let dir = PathBuf::from(dest_dir);
    fs::create_dir_all(&dir)?;
    let name = format!("lian-backup-{}", stamp());
    let db_path = dir.join(format!("{name}.sqlite3"));
    let manifest_path = dir.join(format!("{name}.manifest.json"));

    conn.execute("VACUUM INTO ?1", [db_path.to_string_lossy().as_ref()])?;

    let checksum = sha256_file(&db_path)?;
    let size = fs::metadata(&db_path)?.len() as i64;
    let schema_version = db::schema_version(conn)?;
    let created_at = now_rfc3339();
    let manifest = json!({
        "kind": "lian_backup",
        "created_at": created_at,
        "app_version": APP_VERSION,
        "schema_version": schema_version,
        "database_file": db_path.file_name().unwrap().to_string_lossy(),
        "checksum_sha256": checksum,
        "size_bytes": size,
    });
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    conn.execute(
        "INSERT INTO backups_log (id, created_at, path, app_version, schema_version, checksum_sha256, size_bytes, ok)
         VALUES (?1,?2,?3,?4,?5,?6,?7,1)",
        rusqlite::params![
            new_id(), created_at, db_path.to_string_lossy(), APP_VERSION,
            schema_version, checksum, size
        ],
    )?;
    settings::set(conn, "last_backup_at", &json!(created_at))?;
    settings::set(conn, "backup_dir", &json!(dest_dir))?;

    Ok(json!({
        "ok": true,
        "path": db_path.to_string_lossy(),
        "manifest_path": manifest_path.to_string_lossy(),
        "manifest": manifest,
    }))
}

pub fn list_backups(conn: &Connection) -> Result<Vec<Value>> {
    let mut rows = crate::jsonq::query_json(
        conn,
        "SELECT * FROM backups_log ORDER BY created_at DESC LIMIT 50",
        [],
    )?;
    for r in &mut rows {
        let exists = r["path"].as_str().map(|p| Path::new(p).exists()).unwrap_or(false);
        r["file_exists"] = json!(exists);
    }
    Ok(rows)
}

/// Verify a backup file: integrity check + checksum against its manifest.
pub fn verify_backup(backup_path: &str) -> Result<Value> {
    let path = Path::new(backup_path);
    if !path.exists() {
        return Err(Error::not_found("backup file"));
    }
    let test = Connection::open(path)?;
    let integrity: String = test.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let schema_version: i64 = test.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    drop(test);

    let checksum = sha256_file(path)?;
    let manifest_path = path.with_extension("").with_extension("manifest.json");
    let manifest_matches = match fs::read_to_string(&manifest_path) {
        Ok(m) => serde_json::from_str::<Value>(&m)
            .map(|m| m["checksum_sha256"].as_str() == Some(checksum.as_str()))
            .unwrap_or(false),
        Err(_) => false, // manifest optional for restore, reported honestly
    };
    Ok(json!({
        "ok": integrity == "ok",
        "integrity": integrity,
        "schema_version": schema_version,
        "current_schema_version": db::SCHEMA_VERSION,
        "checksum_sha256": checksum,
        "manifest_found_and_matches": manifest_matches,
    }))
}

/// Restore is performed by the shell (it must close/reopen the connection):
/// this prepares by verifying the backup and writing a safety copy of the
/// live database next to it. Returns what the shell needs to finish.
pub fn prepare_restore(conn: &Connection, live_db_path: &str, backup_path: &str) -> Result<Value> {
    let verification = verify_backup(backup_path)?;
    if verification["ok"].as_bool() != Some(true) {
        return Err(Error::invalid("backup failed integrity check; restore refused"));
    }
    let safety_dir = Path::new(live_db_path).parent().unwrap_or(Path::new(".")).join("safety-copies");
    fs::create_dir_all(&safety_dir)?;
    let safety_path = safety_dir.join(format!("pre-restore-{}.sqlite3", stamp()));
    conn.execute("VACUUM INTO ?1", [safety_path.to_string_lossy().as_ref()])?;
    Ok(json!({
        "verification": verification,
        "safety_copy": safety_path.to_string_lossy(),
        "live_db_path": live_db_path,
        "backup_path": backup_path,
    }))
}

// ---------------------------------------------------------------- export

/// Tables included in CSV export, with a short dictionary description.
const EXPORT_TABLES: &[(&str, &str)] = &[
    ("activity_templates", "Activity type definitions (including archived)"),
    ("activity_events", "Completed/cancelled activities; occurred_at empty means time unknown; duration empty means unknown"),
    ("daily_checkins", "Daily subjective state entries; every entry preserved, omitted ratings are unknown"),
    ("checkin_ratings", "Individual dimension ratings per check-in"),
    ("checkin_dimensions", "Rating dimension definitions with anchors and scale version"),
    ("precept_records", "Five Precepts daily reflection records (private)"),
    ("precept_entries", "Per-precept status per day; absence of a record means not reviewed, never non-observance"),
    ("context_events", "Life context: illness, travel, workload and similar"),
    ("determinations", "Voluntary personal commitments with lifecycle"),
    ("determination_revisions", "Prior wordings preserved on edit"),
    ("determination_reviews", "Private review entries (only where a review rule exists)"),
    ("determination_links", "Explicit links from determinations to other records; not compliance evidence"),
    ("plan_series", "Recurring plan definitions"),
    ("plans", "Planned activities/occurrences; plans are intentions, not completions"),
    ("plan_links", "Explicit plan-to-actual links"),
    ("assessment_sessions", "Assessment attempts with protocol version, validity state and reasons"),
    ("assessment_trials", "Raw trial-level assessment data"),
    ("assessment_schedules", "Assessment scheduling configuration"),
    ("research_protocols", "Predefined research protocols with versions"),
    ("analysis_results", "Generated analysis results with evidence labels and caveats"),
    ("weekly_reflections", "Weekly review notes"),
    ("reminder_rules", "Reminder configuration"),
    ("audit_log", "Edit/delete audit trail with prior values"),
];

pub fn export_csv(conn: &Connection, dest_dir: &str) -> Result<Value> {
    let dir = PathBuf::from(dest_dir).join(format!("lian-export-{}", stamp()));
    fs::create_dir_all(&dir)?;
    let mut table_manifest = Vec::new();

    for (table, description) in EXPORT_TABLES {
        let file = dir.join(format!("{table}.csv"));
        let mut writer = csv::Writer::from_path(&file)?;
        let sql = format!("SELECT * FROM {table}");
        let mut stmt = conn.prepare(&sql)?;
        let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        writer.write_record(&names)?;
        let mut rows = stmt.query([])?;
        let mut count = 0i64;
        while let Some(row) = rows.next()? {
            let mut record: Vec<String> = Vec::with_capacity(names.len());
            for i in 0..names.len() {
                let cell = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => String::new(),
                    rusqlite::types::ValueRef::Integer(v) => v.to_string(),
                    rusqlite::types::ValueRef::Real(v) => v.to_string(),
                    rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
                    rusqlite::types::ValueRef::Blob(_) => "<blob>".into(),
                };
                record.push(cell);
            }
            writer.write_record(&record)?;
            count += 1;
        }
        writer.flush()?;
        table_manifest.push(json!({
            "table": table, "file": format!("{table}.csv"), "rows": count, "description": description,
        }));
    }

    // Analysis-friendly full SQLite copy.
    let sqlite_copy = dir.join("lian-data.sqlite3");
    conn.execute("VACUUM INTO ?1", [sqlite_copy.to_string_lossy().as_ref()])?;

    let manifest = json!({
        "kind": "lian_export",
        "created_at": now_rfc3339(),
        "app_version": APP_VERSION,
        "schema_version": db::schema_version(conn)?,
        "timezone": settings::timezone(conn)?,
        "conventions": {
            "timestamps": "RFC3339 with UTC offset; offset preserved from entry time",
            "local_dates": "YYYY-MM-DD in the user's configured timezone at entry time",
            "missing": "Empty cells mean unknown/not recorded — never zero, false, or 'did not happen'",
            "booleans": "0/1 integers",
            "json_columns": "Some columns contain JSON (tags, metrics, definitions)",
        },
        "tables": table_manifest,
        "sqlite_copy": "lian-data.sqlite3",
    });
    fs::write(dir.join("export-manifest.json"), serde_json::to_string_pretty(&manifest)?)?;

    Ok(json!({ "ok": true, "path": dir.to_string_lossy(), "manifest": manifest }))
}

/// Permanently delete all user data (schema is recreated empty + reseeded).
/// The shell must reopen its connection afterwards.
pub fn purge_all_data(live_db_path: &str) -> Result<Value> {
    let path = Path::new(live_db_path);
    for suffix in ["", "-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{}", path.to_string_lossy(), suffix));
        if p.exists() {
            fs::remove_file(&p)?;
        }
    }
    Ok(json!({ "ok": true, "deleted": live_db_path }))
}
