//! Templates, activity events, daily check-ins, Five Precepts, context events.

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::jsonq::{query_json, query_one, snapshot};
use crate::util::{local_date_of, new_id, now_rfc3339, today_in_tz};
use crate::{settings, Error, Result};

// ---------------------------------------------------------------- audit

pub fn audit(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    action: &str,
    prior: Option<&Value>,
    reason: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_log (id, entity_type, entity_id, action, changed_at, prior_values, reason)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            new_id(), entity_type, entity_id, action, now_rfc3339(),
            prior.map(|v| v.to_string()), reason
        ],
    )?;
    Ok(())
}

pub fn audit_for(conn: &Connection, entity_type: &str, entity_id: &str) -> Result<Vec<Value>> {
    query_json(
        conn,
        "SELECT * FROM audit_log WHERE entity_type=?1 AND entity_id=?2 ORDER BY changed_at DESC",
        [entity_type, entity_id],
    )
}

// ---------------------------------------------------------------- templates

#[derive(Deserialize)]
pub struct TemplateInput {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub glyph: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub subtypes: Vec<String>,
    #[serde(default)]
    pub default_duration_seconds: Option<i64>,
    #[serde(default)]
    pub supports_intensity: bool,
    #[serde(default)]
    pub supports_body_state: bool,
}

pub fn list_templates(conn: &Connection, include_archived: bool) -> Result<Vec<Value>> {
    let sql = if include_archived {
        "SELECT * FROM activity_templates ORDER BY is_archived, sort_order, name"
    } else {
        "SELECT * FROM activity_templates WHERE is_archived=0 ORDER BY sort_order, name"
    };
    query_json(conn, sql, [])
}

pub fn save_template(conn: &Connection, input: TemplateInput) -> Result<Value> {
    let now = now_rfc3339();
    if input.name.trim().is_empty() {
        return Err(Error::invalid("template name cannot be empty"));
    }
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "activity_templates", id)?
                .ok_or_else(|| Error::not_found("template"))?;
            conn.execute(
                "UPDATE activity_templates SET name=?2, glyph=COALESCE(?3, glyph), color=COALESCE(?4, color),
                   subtypes=?5, default_duration_seconds=?6, supports_intensity=?7, supports_body_state=?8, updated_at=?9
                 WHERE id=?1",
                params![
                    id, input.name.trim(), input.glyph, input.color,
                    serde_json::to_string(&input.subtypes)?, input.default_duration_seconds,
                    input.supports_intensity as i64, input.supports_body_state as i64, now
                ],
            )?;
            audit(conn, "activity_template", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            let max_order: i64 = conn.query_row(
                "SELECT COALESCE(MAX(sort_order),0)+1 FROM activity_templates", [], |r| r.get(0))?;
            conn.execute(
                "INSERT INTO activity_templates
                   (id, name, category, glyph, color, subtypes, default_duration_seconds,
                    supports_intensity, supports_body_state, sort_order, is_builtin, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0,?11,?11)",
                params![
                    id, input.name.trim(), input.category.as_deref().unwrap_or("custom"),
                    input.glyph.as_deref().unwrap_or("·"), input.color.as_deref().unwrap_or("slate"),
                    serde_json::to_string(&input.subtypes)?, input.default_duration_seconds,
                    input.supports_intensity as i64, input.supports_body_state as i64, max_order, now
                ],
            )?;
            id
        }
    };
    query_one(conn, "SELECT * FROM activity_templates WHERE id=?1", [&id])?
        .ok_or_else(|| Error::not_found("template"))
}

pub fn set_template_archived(conn: &Connection, id: &str, archived: bool) -> Result<()> {
    // Archived templates stay resolvable for historical events (contract invariant).
    let n = conn.execute(
        "UPDATE activity_templates SET is_archived=?2, updated_at=?3 WHERE id=?1",
        params![id, archived as i64, now_rfc3339()],
    )?;
    if n == 0 {
        return Err(Error::not_found("template"));
    }
    Ok(())
}

pub fn reorder_templates(conn: &Connection, ordered_ids: Vec<String>) -> Result<()> {
    let now = now_rfc3339();
    for (i, id) in ordered_ids.iter().enumerate() {
        conn.execute(
            "UPDATE activity_templates SET sort_order=?2, updated_at=?3 WHERE id=?1",
            params![id, i as i64, now],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------- activity events

#[derive(Deserialize)]
pub struct EventInput {
    pub id: Option<String>,
    pub template_id: String,
    /// RFC3339 instant; None with `local_date` set means "time unknown".
    pub occurred_at: Option<String>,
    pub local_date: Option<String>,
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub intensity: Option<i64>,
    #[serde(default)]
    pub perceived_quality: Option<i64>,
    #[serde(default)]
    pub body_state_before: Option<String>,
    #[serde(default)]
    pub body_state_after: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>, // 'manual' | 'timer'
    #[serde(default)]
    pub edit_reason: Option<String>,
}

fn validate_scale(v: Option<i64>, name: &str) -> Result<()> {
    if let Some(x) = v {
        if !(1..=5).contains(&x) {
            return Err(Error::invalid(format!("{name} must be between 1 and 5")));
        }
    }
    Ok(())
}

pub fn save_event(conn: &Connection, input: EventInput) -> Result<Value> {
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    validate_scale(input.intensity, "intensity")?;
    validate_scale(input.perceived_quality, "perceived_quality")?;
    if let Some(d) = input.duration_seconds {
        if d < 0 {
            return Err(Error::invalid("duration cannot be negative"));
        }
    }
    let source = input.source.clone().unwrap_or_else(|| "manual".into());
    if !["manual", "timer"].contains(&source.as_str()) {
        return Err(Error::invalid("source must be manual or timer"));
    }
    let (occurred_at, local_date, time_known) = match (&input.occurred_at, &input.local_date) {
        (Some(at), _) => (Some(at.clone()), local_date_of(at, &tz)?, true),
        (None, Some(d)) => {
            crate::util::parse_date(d)?;
            (None, d.clone(), false)
        }
        (None, None) => return Err(Error::invalid("either occurred_at or local_date is required")),
    };
    let tags = serde_json::to_string(&input.context_tags)?;

    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "activity_events", id)?
                .ok_or_else(|| Error::not_found("activity event"))?;
            conn.execute(
                "UPDATE activity_events SET template_id=?2, occurred_at=?3, local_date=?4, time_known=?5,
                   duration_seconds=?6, subtype=?7, intensity=?8, perceived_quality=?9,
                   body_state_before=?10, body_state_after=?11, location=?12, note=?13,
                   context_tags=?14, plan_id=?15, updated_at=?16
                 WHERE id=?1 AND deleted_at IS NULL",
                params![
                    id, input.template_id, occurred_at, local_date, time_known as i64,
                    input.duration_seconds, input.subtype, input.intensity, input.perceived_quality,
                    input.body_state_before, input.body_state_after, input.location, input.note,
                    tags, input.plan_id, now
                ],
            )?;
            audit(conn, "activity_event", id, "update", Some(&prior), input.edit_reason.as_deref())?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO activity_events
                   (id, template_id, occurred_at, local_date, time_known, timezone, logged_at,
                    duration_seconds, subtype, intensity, perceived_quality, body_state_before,
                    body_state_after, location, note, context_tags, plan_id, source, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?19)",
                params![
                    id, input.template_id, occurred_at, local_date, time_known as i64, tz, now,
                    input.duration_seconds, input.subtype, input.intensity, input.perceived_quality,
                    input.body_state_before, input.body_state_after, input.location, input.note,
                    tags, input.plan_id, source, now
                ],
            )?;
            id
        }
    };

    // A plan link set on the event marks the plan completed_linked (explicit user action).
    if let Some(plan_id) = &input.plan_id {
        crate::repo_plans::link_event_to_plan(conn, plan_id, &id)?;
    }

    get_event(conn, &id)
}

pub fn get_event(conn: &Connection, id: &str) -> Result<Value> {
    query_one(
        conn,
        "SELECT e.*, t.name AS template_name, t.category AS template_category, t.glyph AS template_glyph,
                t.color AS template_color
         FROM activity_events e JOIN activity_templates t ON t.id = e.template_id
         WHERE e.id=?1",
        [id],
    )?
    .ok_or_else(|| Error::not_found("activity event"))
}

pub fn list_events(conn: &Connection, from: &str, to: &str) -> Result<Vec<Value>> {
    query_json(
        conn,
        "SELECT e.*, t.name AS template_name, t.category AS template_category, t.glyph AS template_glyph,
                t.color AS template_color
         FROM activity_events e JOIN activity_templates t ON t.id = e.template_id
         WHERE e.deleted_at IS NULL AND e.local_date >= ?1 AND e.local_date <= ?2
         ORDER BY e.local_date DESC, COALESCE(e.occurred_at, e.logged_at) DESC",
        [from, to],
    )
}

pub fn set_event_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    if !["completed", "cancelled"].contains(&status) {
        return Err(Error::invalid("status must be completed or cancelled"));
    }
    let prior = snapshot(conn, "activity_events", id)?.ok_or_else(|| Error::not_found("activity event"))?;
    conn.execute(
        "UPDATE activity_events SET status=?2, updated_at=?3 WHERE id=?1",
        params![id, status, now_rfc3339()],
    )?;
    audit(conn, "activity_event", id, "lifecycle", Some(&prior), None)?;
    Ok(())
}

pub fn delete_event(conn: &Connection, id: &str, hard: bool, reason: Option<&str>) -> Result<()> {
    let prior = snapshot(conn, "activity_events", id)?.ok_or_else(|| Error::not_found("activity event"))?;
    if hard {
        conn.execute("DELETE FROM plan_links WHERE activity_event_id=?1", [id])?;
        conn.execute("DELETE FROM activity_events WHERE id=?1", [id])?;
    } else {
        conn.execute(
            "UPDATE activity_events SET deleted_at=?2, updated_at=?2 WHERE id=?1",
            params![id, now_rfc3339()],
        )?;
    }
    audit(conn, "activity_event", id, "delete", Some(&prior), reason)?;
    // Any dependent analysis results are now stale rather than silently wrong.
    conn.execute("UPDATE analysis_results SET is_stale=1", [])?;
    Ok(())
}

// ---------------------------------------------------------------- daily check-ins

#[derive(Deserialize)]
pub struct CheckinInput {
    pub id: Option<String>,
    pub local_date: String,
    /// dimension_id -> value; omitted dimensions stay unknown.
    #[serde(default)]
    pub ratings: std::collections::BTreeMap<String, i64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sleep_start: Option<String>,
    #[serde(default)]
    pub sleep_end: Option<String>,
    #[serde(default)]
    pub sleep_duration_minutes: Option<i64>,
    #[serde(default)]
    pub sleep_quality: Option<i64>,
    #[serde(default)]
    pub awakenings: Option<i64>,
    #[serde(default)]
    pub context_tags: Vec<String>,
}

pub fn save_checkin(conn: &Connection, input: CheckinInput) -> Result<Value> {
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    crate::util::parse_date(&input.local_date)?;
    validate_scale(input.sleep_quality, "sleep_quality")?;
    for v in input.ratings.values() {
        if !(1..=5).contains(v) {
            return Err(Error::invalid("ratings must be between 1 and 5"));
        }
    }
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "daily_checkins", id)?.ok_or_else(|| Error::not_found("check-in"))?;
            conn.execute(
                "UPDATE daily_checkins SET note=?2, sleep_start=?3, sleep_end=?4, sleep_duration_minutes=?5,
                   sleep_quality=?6, awakenings=?7, context_tags=?8, updated_at=?9
                 WHERE id=?1",
                params![
                    id, input.note, input.sleep_start, input.sleep_end, input.sleep_duration_minutes,
                    input.sleep_quality, input.awakenings, serde_json::to_string(&input.context_tags)?, now
                ],
            )?;
            conn.execute("DELETE FROM checkin_ratings WHERE checkin_id=?1", [id])?;
            audit(conn, "daily_checkin", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO daily_checkins
                   (id, local_date, logged_at, timezone, note, sleep_start, sleep_end,
                    sleep_duration_minutes, sleep_quality, awakenings, context_tags, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)",
                params![
                    id, input.local_date, now, tz, input.note, input.sleep_start, input.sleep_end,
                    input.sleep_duration_minutes, input.sleep_quality, input.awakenings,
                    serde_json::to_string(&input.context_tags)?, now
                ],
            )?;
            id
        }
    };
    for (dim_id, value) in &input.ratings {
        conn.execute(
            "INSERT INTO checkin_ratings (id, checkin_id, dimension_id, value) VALUES (?1,?2,?3,?4)",
            params![new_id(), id, dim_id, value],
        )?;
    }
    get_checkin(conn, &id)
}

fn attach_ratings(conn: &Connection, checkin: &mut Value) -> Result<()> {
    let id = checkin["id"].as_str().unwrap_or_default().to_string();
    let ratings = query_json(
        conn,
        "SELECT r.dimension_id, r.value, d.key, d.label FROM checkin_ratings r
         JOIN checkin_dimensions d ON d.id = r.dimension_id WHERE r.checkin_id=?1",
        [&id],
    )?;
    checkin["ratings"] = json!(ratings);
    Ok(())
}

pub fn get_checkin(conn: &Connection, id: &str) -> Result<Value> {
    let mut v = query_one(conn, "SELECT * FROM daily_checkins WHERE id=?1", [id])?
        .ok_or_else(|| Error::not_found("check-in"))?;
    attach_ratings(conn, &mut v)?;
    Ok(v)
}

/// All check-ins in range, ratings attached; raw entries preserved even when
/// a day has several.
pub fn list_checkins(conn: &Connection, from: &str, to: &str) -> Result<Vec<Value>> {
    let mut rows = query_json(
        conn,
        "SELECT * FROM daily_checkins WHERE deleted_at IS NULL AND local_date >= ?1 AND local_date <= ?2
         ORDER BY local_date DESC, logged_at DESC",
        [from, to],
    )?;
    for row in &mut rows {
        attach_ratings(conn, row)?;
    }
    Ok(rows)
}

pub fn delete_checkin(conn: &Connection, id: &str, hard: bool) -> Result<()> {
    let prior = snapshot(conn, "daily_checkins", id)?.ok_or_else(|| Error::not_found("check-in"))?;
    if hard {
        conn.execute("DELETE FROM daily_checkins WHERE id=?1", [id])?;
    } else {
        conn.execute(
            "UPDATE daily_checkins SET deleted_at=?2, updated_at=?2 WHERE id=?1",
            params![id, now_rfc3339()],
        )?;
    }
    audit(conn, "daily_checkin", id, "delete", Some(&prior), None)?;
    conn.execute("UPDATE analysis_results SET is_stale=1", [])?;
    Ok(())
}

pub fn list_dimensions(conn: &Connection) -> Result<Vec<Value>> {
    query_json(conn, "SELECT * FROM checkin_dimensions ORDER BY sort_order", [])
}

pub fn configure_dimensions(conn: &Connection, enabled_ids: Vec<String>) -> Result<()> {
    let now = now_rfc3339();
    conn.execute("UPDATE checkin_dimensions SET is_enabled=0, updated_at=?1", [&now])?;
    for (i, id) in enabled_ids.iter().enumerate() {
        conn.execute(
            "UPDATE checkin_dimensions SET is_enabled=1, sort_order=?2, updated_at=?3 WHERE id=?1",
            params![id, i as i64, now],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------- Five Precepts

#[derive(Deserialize)]
pub struct PreceptEntryInput {
    pub precept_key: String,
    pub status: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct PreceptInput {
    pub local_date: String,
    pub entries: Vec<PreceptEntryInput>,
    #[serde(default)]
    pub overall_note: Option<String>,
}

const PRECEPT_STATUSES: &[&str] = &["observed", "not_observed", "uncertain", "not_reviewed"];

pub fn save_precepts(conn: &Connection, input: PreceptInput) -> Result<Value> {
    crate::util::parse_date(&input.local_date)?;
    let canonical: Vec<&str> = crate::seed::PRECEPT_KEYS.iter().map(|(k, _)| *k).collect();
    for e in &input.entries {
        if !canonical.contains(&e.precept_key.as_str()) {
            return Err(Error::invalid(format!("unknown precept '{}'", e.precept_key)));
        }
        if !PRECEPT_STATUSES.contains(&e.status.as_str()) {
            return Err(Error::invalid(format!("invalid precept status '{}'", e.status)));
        }
    }
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    let existing: Option<String> = conn
        .query_row("SELECT id FROM precept_records WHERE local_date=?1", [&input.local_date], |r| r.get(0))
        .ok();
    let record_id = match existing {
        Some(id) => {
            conn.execute(
                "UPDATE precept_records SET overall_note=?2, logged_at=?3, updated_at=?3 WHERE id=?1",
                params![id, input.overall_note, now],
            )?;
            id
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO precept_records (id, local_date, logged_at, timezone, overall_note, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?6)",
                params![id, input.local_date, now, tz, input.overall_note, now],
            )?;
            id
        }
    };
    for e in &input.entries {
        conn.execute(
            "INSERT INTO precept_entries (id, record_id, precept_key, status, note)
             VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(record_id, precept_key) DO UPDATE SET status=?4, note=?5",
            params![new_id(), record_id, e.precept_key, e.status, e.note],
        )?;
    }
    get_precepts(conn, &input.local_date)
}

/// Record for a date; entries default to `not_reviewed` presentation-side when
/// absent — absence is never stored as a status.
pub fn get_precepts(conn: &Connection, local_date: &str) -> Result<Value> {
    let rec = query_one(conn, "SELECT * FROM precept_records WHERE local_date=?1", [local_date])?;
    match rec {
        Some(mut r) => {
            let id = r["id"].as_str().unwrap_or_default().to_string();
            r["entries"] = json!(query_json(
                conn,
                "SELECT precept_key, status, note FROM precept_entries WHERE record_id=?1",
                [&id],
            )?);
            Ok(r)
        }
        None => Ok(Value::Null),
    }
}

pub fn list_precepts(conn: &Connection, from: &str, to: &str) -> Result<Vec<Value>> {
    let mut rows = query_json(
        conn,
        "SELECT * FROM precept_records WHERE local_date >= ?1 AND local_date <= ?2 ORDER BY local_date DESC",
        [from, to],
    )?;
    for r in &mut rows {
        let id = r["id"].as_str().unwrap_or_default().to_string();
        r["entries"] = json!(query_json(
            conn,
            "SELECT precept_key, status, note FROM precept_entries WHERE record_id=?1",
            [&id],
        )?);
    }
    Ok(rows)
}

// ---------------------------------------------------------------- context events

#[derive(Deserialize)]
pub struct ContextInput {
    pub id: Option<String>,
    pub kind: String,
    pub label: String,
    pub start_date: String,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

pub fn save_context_event(conn: &Connection, input: ContextInput) -> Result<Value> {
    crate::util::parse_date(&input.start_date)?;
    if let Some(e) = &input.end_date {
        if crate::util::parse_date(e)? < crate::util::parse_date(&input.start_date)? {
            return Err(Error::invalid("end date is before start date"));
        }
    }
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    let started_at = crate::util::local_to_instant(&input.start_date, "00:00", &tz)?;
    let ended_at = match &input.end_date {
        Some(d) => Some(crate::util::local_to_instant(d, "23:59", &tz)?),
        None => None,
    };
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "context_events", id)?.ok_or_else(|| Error::not_found("context event"))?;
            conn.execute(
                "UPDATE context_events SET kind=?2, label=?3, started_at=?4, ended_at=?5, start_date=?6,
                   end_date=?7, tags=?8, note=?9, updated_at=?10 WHERE id=?1",
                params![
                    id, input.kind, input.label, started_at, ended_at, input.start_date,
                    input.end_date, serde_json::to_string(&input.tags)?, input.note, now
                ],
            )?;
            audit(conn, "context_event", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO context_events
                   (id, kind, label, started_at, ended_at, start_date, end_date, tags, note, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
                params![
                    id, input.kind, input.label, started_at, ended_at, input.start_date,
                    input.end_date, serde_json::to_string(&input.tags)?, input.note, now
                ],
            )?;
            id
        }
    };
    query_one(conn, "SELECT * FROM context_events WHERE id=?1", [&id])?
        .ok_or_else(|| Error::not_found("context event"))
}

/// Context events overlapping the [from, to] date range (open-ended events count as ongoing).
pub fn list_context_events(conn: &Connection, from: &str, to: &str) -> Result<Vec<Value>> {
    query_json(
        conn,
        "SELECT * FROM context_events
         WHERE deleted_at IS NULL AND start_date <= ?2 AND (end_date IS NULL OR end_date >= ?1)
         ORDER BY start_date DESC",
        [from, to],
    )
}

pub fn delete_context_event(conn: &Connection, id: &str) -> Result<()> {
    let prior = snapshot(conn, "context_events", id)?.ok_or_else(|| Error::not_found("context event"))?;
    conn.execute(
        "UPDATE context_events SET deleted_at=?2, updated_at=?2 WHERE id=?1",
        params![id, now_rfc3339()],
    )?;
    audit(conn, "context_event", id, "delete", Some(&prior), None)?;
    Ok(())
}

// ---------------------------------------------------------------- helpers

pub fn today(conn: &Connection) -> Result<String> {
    today_in_tz(&settings::timezone(conn)?)
}
