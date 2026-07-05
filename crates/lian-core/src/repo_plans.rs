//! Planning: one-off plans, recurring series with materialized occurrences,
//! derived statuses, and explicit plan↔actual links.
//!
//! Recurrence model: a `plan_series` row holds the rule; concrete occurrences
//! are materialized into `plans` rows on demand (idempotent per
//! (series_id, occurrence_date)). Editing a series regenerates only future,
//! untouched occurrences — past occurrences and anything linked, skipped, or
//! cancelled are immutable history.

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::Value;

use crate::jsonq::{query_json, query_one, snapshot};
use crate::repo_daily::audit;
use crate::util::{add_days, local_to_instant, new_id, now_rfc3339, parse_date, weekday_index};
use crate::{settings, Error, Result};

pub const PLAN_KINDS: &[&str] = &["activity", "assessment", "recovery", "commitment", "custom"];

// ---------------------------------------------------------------- one-off plans

#[derive(Deserialize)]
pub struct PlanInput {
    pub id: Option<String>,
    pub title: String,
    pub kind: String,
    #[serde(default)]
    pub activity_template_id: Option<String>,
    #[serde(default)]
    pub assessment_kind: Option<String>,
    pub local_date: String,
    /// 'HH:MM' local; None = date-only plan.
    #[serde(default)]
    pub time_of_day: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub target_duration_seconds: Option<i64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub determination_id: Option<String>,
    #[serde(default)]
    pub protocol_id: Option<String>,
    #[serde(default)]
    pub reminder_offset_minutes: Option<i64>,
}

pub fn save_plan(conn: &Connection, input: PlanInput) -> Result<Value> {
    if !PLAN_KINDS.contains(&input.kind.as_str()) {
        return Err(Error::invalid(format!("unknown plan kind '{}'", input.kind)));
    }
    if input.title.trim().is_empty() {
        return Err(Error::invalid("plan title cannot be empty"));
    }
    parse_date(&input.local_date)?;
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    let (scheduled_start, scheduled_end, date_only) = match &input.time_of_day {
        Some(t) => {
            let start = local_to_instant(&input.local_date, t, &tz)?;
            let end = input.duration_minutes.map(|m| {
                let st = crate::util::parse_instant(&start).unwrap();
                (st + chrono::Duration::minutes(m)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            });
            (Some(start), end, false)
        }
        None => (None, None, true),
    };
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "plans", id)?.ok_or_else(|| Error::not_found("plan"))?;
            conn.execute(
                "UPDATE plans SET title=?2, kind=?3, activity_template_id=?4, assessment_kind=?5,
                   scheduled_start=?6, scheduled_end=?7, local_date=?8, date_only=?9,
                   target_duration_seconds=?10, note=?11, determination_id=?12, protocol_id=?13,
                   reminder_offset_minutes=?14, updated_at=?15
                 WHERE id=?1 AND deleted_at IS NULL",
                params![
                    id, input.title.trim(), input.kind, input.activity_template_id, input.assessment_kind,
                    scheduled_start, scheduled_end, input.local_date, date_only as i64,
                    input.target_duration_seconds, input.note, input.determination_id, input.protocol_id,
                    input.reminder_offset_minutes, now
                ],
            )?;
            audit(conn, "plan", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO plans
                   (id, title, kind, activity_template_id, assessment_kind, scheduled_start, scheduled_end,
                    local_date, date_only, timezone, target_duration_seconds, status, note,
                    determination_id, protocol_id, reminder_offset_minutes, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'upcoming',?12,?13,?14,?15,?16,?16)",
                params![
                    id, input.title.trim(), input.kind, input.activity_template_id, input.assessment_kind,
                    scheduled_start, scheduled_end, input.local_date, date_only as i64, tz,
                    input.target_duration_seconds, input.note, input.determination_id,
                    input.protocol_id, input.reminder_offset_minutes, now
                ],
            )?;
            id
        }
    };
    get_plan(conn, &id)
}

// ---------------------------------------------------------------- series

#[derive(Deserialize)]
pub struct SeriesInput {
    pub id: Option<String>,
    pub title: String,
    pub kind: String,
    #[serde(default)]
    pub activity_template_id: Option<String>,
    #[serde(default)]
    pub assessment_kind: Option<String>,
    pub frequency: String, // daily | weekly | monthly
    #[serde(default = "one")]
    pub interval: i64,
    #[serde(default)]
    pub weekdays: Vec<i64>, // Mon=0
    #[serde(default)]
    pub month_day: Option<i64>,
    #[serde(default)]
    pub time_of_day: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<i64>,
    pub starts_on: String,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub target_duration_seconds: Option<i64>,
    #[serde(default)]
    pub determination_id: Option<String>,
    #[serde(default)]
    pub protocol_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub reminder_offset_minutes: Option<i64>,
}

fn one() -> i64 {
    1
}

pub fn save_series(conn: &Connection, input: SeriesInput) -> Result<Value> {
    if !PLAN_KINDS.contains(&input.kind.as_str()) {
        return Err(Error::invalid(format!("unknown plan kind '{}'", input.kind)));
    }
    if !["daily", "weekly", "monthly"].contains(&input.frequency.as_str()) {
        return Err(Error::invalid("frequency must be daily, weekly, or monthly"));
    }
    if input.frequency == "weekly" && input.weekdays.is_empty() {
        return Err(Error::invalid("weekly recurrence needs at least one weekday"));
    }
    parse_date(&input.starts_on)?;
    if let Some(u) = &input.until {
        parse_date(u)?;
    }
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    let weekdays = serde_json::to_string(&input.weekdays)?;
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "plan_series", id)?.ok_or_else(|| Error::not_found("plan series"))?;
            conn.execute(
                "UPDATE plan_series SET title=?2, kind=?3, activity_template_id=?4, assessment_kind=?5,
                   frequency=?6, interval=?7, weekdays=?8, month_day=?9, time_of_day=?10,
                   duration_minutes=?11, starts_on=?12, until=?13, target_duration_seconds=?14,
                   determination_id=?15, protocol_id=?16, note=?17, reminder_offset_minutes=?18, updated_at=?19
                 WHERE id=?1",
                params![
                    id, input.title.trim(), input.kind, input.activity_template_id, input.assessment_kind,
                    input.frequency, input.interval.max(1), weekdays, input.month_day, input.time_of_day,
                    input.duration_minutes, input.starts_on, input.until, input.target_duration_seconds,
                    input.determination_id, input.protocol_id, input.note, input.reminder_offset_minutes, now
                ],
            )?;
            audit(conn, "plan_series", id, "update", Some(&prior), None)?;
            // Regenerate only future, untouched occurrences.
            let today = crate::repo_daily::today(conn)?;
            conn.execute(
                "DELETE FROM plans WHERE series_id=?1 AND local_date > ?2 AND status IN ('upcoming','due')
                 AND id NOT IN (SELECT plan_id FROM plan_links)",
                params![id, today],
            )?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO plan_series
                   (id, title, kind, activity_template_id, assessment_kind, frequency, interval, weekdays,
                    month_day, time_of_day, duration_minutes, timezone, starts_on, until,
                    target_duration_seconds, determination_id, protocol_id, note, reminder_offset_minutes,
                    created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?20)",
                params![
                    id, input.title.trim(), input.kind, input.activity_template_id, input.assessment_kind,
                    input.frequency, input.interval.max(1), weekdays, input.month_day, input.time_of_day,
                    input.duration_minutes, tz, input.starts_on, input.until, input.target_duration_seconds,
                    input.determination_id, input.protocol_id, input.note, input.reminder_offset_minutes, now
                ],
            )?;
            id
        }
    };
    query_one(conn, "SELECT * FROM plan_series WHERE id=?1", [&id])?
        .ok_or_else(|| Error::not_found("plan series"))
}

pub fn list_series(conn: &Connection) -> Result<Vec<Value>> {
    query_json(conn, "SELECT * FROM plan_series WHERE is_active=1 ORDER BY created_at", [])
}

/// Deactivate a series and remove future unlinked, untouched occurrences.
/// Past occurrences remain historical record.
pub fn end_series(conn: &Connection, id: &str) -> Result<()> {
    let prior = snapshot(conn, "plan_series", id)?.ok_or_else(|| Error::not_found("plan series"))?;
    let today = crate::repo_daily::today(conn)?;
    conn.execute(
        "UPDATE plan_series SET is_active=0, updated_at=?2 WHERE id=?1",
        params![id, now_rfc3339()],
    )?;
    conn.execute(
        "DELETE FROM plans WHERE series_id=?1 AND local_date > ?2 AND status IN ('upcoming','due')
         AND id NOT IN (SELECT plan_id FROM plan_links)",
        params![id, today],
    )?;
    audit(conn, "plan_series", id, "lifecycle", Some(&prior), None)?;
    Ok(())
}

fn series_dates(series: &Value, from: &str, to: &str) -> Result<Vec<String>> {
    let starts_on = series["starts_on"].as_str().unwrap_or(from);
    let until = series["until"].as_str();
    let frequency = series["frequency"].as_str().unwrap_or("daily");
    let interval = series["interval"].as_i64().unwrap_or(1).max(1);
    let weekdays: Vec<i64> = serde_json::from_value(series["weekdays"].clone()).unwrap_or_default();
    let month_day = series["month_day"].as_i64();

    let lo = if starts_on > from { starts_on } else { from };
    let hi = match until {
        Some(u) if u < to => u,
        _ => to,
    };
    let mut out = Vec::new();
    if parse_date(lo)? > parse_date(hi)? {
        return Ok(out);
    }
    let start = parse_date(starts_on)?;
    let mut d = parse_date(lo)?;
    let end = parse_date(hi)?;
    while d <= end {
        let ds = d.format("%Y-%m-%d").to_string();
        let delta_days = (d - start).num_days();
        let matches = match frequency {
            "daily" => delta_days % interval == 0,
            "weekly" => {
                let week_delta = delta_days.div_euclid(7);
                weekdays.contains(&(weekday_index(&ds)? as i64)) && week_delta % interval == 0
            }
            "monthly" => {
                use chrono::Datelike;
                let target = month_day.unwrap_or_else(|| start.day() as i64);
                d.day() as i64 == target
            }
            _ => false,
        };
        if matches {
            out.push(ds);
        }
        d += chrono::Duration::days(1);
    }
    Ok(out)
}

/// Idempotently materialize series occurrences in [from, to].
pub fn ensure_occurrences(conn: &Connection, from: &str, to: &str) -> Result<()> {
    let now = now_rfc3339();
    let tz = settings::timezone(conn)?;
    let series = list_series(conn)?;
    for s in series {
        let sid = s["id"].as_str().unwrap_or_default().to_string();
        for date in series_dates(&s, from, to)? {
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM plans WHERE series_id=?1 AND occurrence_date=?2",
                params![sid, date],
                |r| r.get(0),
            )?;
            if exists > 0 {
                continue;
            }
            let time_of_day = s["time_of_day"].as_str();
            let (scheduled_start, scheduled_end, date_only) = match time_of_day {
                Some(t) => {
                    let start = local_to_instant(&date, t, &tz)?;
                    let end = s["duration_minutes"].as_i64().map(|m| {
                        let st = crate::util::parse_instant(&start).unwrap();
                        (st + chrono::Duration::minutes(m)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                    });
                    (Some(start), end, 0i64)
                }
                None => (None, None, 1i64),
            };
            conn.execute(
                "INSERT INTO plans
                   (id, series_id, occurrence_date, title, kind, activity_template_id, assessment_kind,
                    scheduled_start, scheduled_end, local_date, date_only, timezone,
                    target_duration_seconds, status, note, determination_id, protocol_id,
                    reminder_offset_minutes, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'upcoming',?14,?15,?16,?17,?18,?18)",
                params![
                    new_id(), sid, date, s["title"].as_str(), s["kind"].as_str(),
                    s["activity_template_id"].as_str(), s["assessment_kind"].as_str(),
                    scheduled_start, scheduled_end, date, date_only, tz,
                    s["target_duration_seconds"].as_i64(), s["note"].as_str(),
                    s["determination_id"].as_str(), s["protocol_id"].as_str(),
                    s["reminder_offset_minutes"].as_i64(), now
                ],
            )?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- status & listing

/// Effective status: explicit statuses win; otherwise derive due/expired from
/// today's date. An unresolved past plan is information, not failure.
fn effective_status(stored: &str, plan_date: &str, today: &str) -> String {
    match stored {
        "upcoming" | "due" | "expired_unresolved" => {
            if plan_date < today {
                "expired_unresolved".into()
            } else if plan_date == today {
                "due".into()
            } else {
                "upcoming".into()
            }
        }
        other => other.to_string(),
    }
}

pub fn list_plans(conn: &Connection, from: &str, to: &str) -> Result<Vec<Value>> {
    ensure_occurrences(conn, from, to)?;
    let today = crate::repo_daily::today(conn)?;
    let mut rows = query_json(
        conn,
        "SELECT p.*, t.name AS template_name, t.glyph AS template_glyph, t.color AS template_color,
                d.title AS determination_title
         FROM plans p
         LEFT JOIN activity_templates t ON t.id = p.activity_template_id
         LEFT JOIN determinations d ON d.id = p.determination_id
         WHERE p.deleted_at IS NULL AND p.local_date >= ?1 AND p.local_date <= ?2
         ORDER BY p.local_date, COALESCE(p.scheduled_start, p.local_date)",
        [from, to],
    )?;
    for p in &mut rows {
        let stored = p["status"].as_str().unwrap_or("upcoming").to_string();
        let date = p["local_date"].as_str().unwrap_or("").to_string();
        p["effective_status"] = Value::String(effective_status(&stored, &date, &today));
        let pid = p["id"].as_str().unwrap_or_default().to_string();
        p["links"] = serde_json::json!(query_json(
            conn,
            "SELECT l.*, e.note AS event_note, e.duration_seconds AS event_duration
             FROM plan_links l LEFT JOIN activity_events e ON e.id = l.activity_event_id
             WHERE l.plan_id=?1",
            [&pid],
        )?);
    }
    Ok(rows)
}

pub fn get_plan(conn: &Connection, id: &str) -> Result<Value> {
    let today = crate::repo_daily::today(conn)?;
    let mut p = query_one(
        conn,
        "SELECT p.*, t.name AS template_name, t.glyph AS template_glyph, t.color AS template_color
         FROM plans p LEFT JOIN activity_templates t ON t.id = p.activity_template_id
         WHERE p.id=?1",
        [id],
    )?
    .ok_or_else(|| Error::not_found("plan"))?;
    let stored = p["status"].as_str().unwrap_or("upcoming").to_string();
    let date = p["local_date"].as_str().unwrap_or("").to_string();
    p["effective_status"] = Value::String(effective_status(&stored, &date, &today));
    p["links"] = serde_json::json!(query_json(conn, "SELECT * FROM plan_links WHERE plan_id=?1", [id])?);
    Ok(p)
}

/// Explicit user statuses. Skipping/cancelling is neutral information.
pub fn set_plan_status(conn: &Connection, id: &str, status: &str) -> Result<Value> {
    if !["upcoming", "skipped", "cancelled", "completed_unlinked"].contains(&status) {
        return Err(Error::invalid(format!("cannot set plan status '{status}' directly")));
    }
    let prior = snapshot(conn, "plans", id)?.ok_or_else(|| Error::not_found("plan"))?;
    conn.execute(
        "UPDATE plans SET status=?2, updated_at=?3 WHERE id=?1",
        params![id, status, now_rfc3339()],
    )?;
    audit(conn, "plan", id, "lifecycle", Some(&prior), None)?;
    get_plan(conn, id)
}

pub fn delete_plan(conn: &Connection, id: &str) -> Result<()> {
    let prior = snapshot(conn, "plans", id)?.ok_or_else(|| Error::not_found("plan"))?;
    conn.execute(
        "UPDATE plans SET deleted_at=?2, updated_at=?2 WHERE id=?1",
        params![id, now_rfc3339()],
    )?;
    audit(conn, "plan", id, "delete", Some(&prior), None)?;
    Ok(())
}

/// Explicit link between a plan and a completed activity event.
pub fn link_event_to_plan(conn: &Connection, plan_id: &str, event_id: &str) -> Result<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM plan_links WHERE plan_id=?1 AND activity_event_id=?2",
        params![plan_id, event_id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        conn.execute(
            "INSERT INTO plan_links (id, plan_id, activity_event_id, created_at) VALUES (?1,?2,?3,?4)",
            params![new_id(), plan_id, event_id, now_rfc3339()],
        )?;
    }
    conn.execute(
        "UPDATE plans SET status='completed_linked', updated_at=?2 WHERE id=?1",
        params![plan_id, now_rfc3339()],
    )?;
    conn.execute(
        "UPDATE activity_events SET plan_id=?2, updated_at=?3 WHERE id=?1",
        params![event_id, plan_id, now_rfc3339()],
    )?;
    Ok(())
}

pub fn link_session_to_plan(conn: &Connection, plan_id: &str, session_id: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO plan_links (id, plan_id, assessment_session_id, created_at) VALUES (?1,?2,?3,?4)",
        params![new_id(), plan_id, session_id, now_rfc3339()],
    )?;
    conn.execute(
        "UPDATE plans SET status='completed_linked', updated_at=?2 WHERE id=?1",
        params![plan_id, now_rfc3339()],
    )?;
    Ok(())
}

pub fn unlink_plan(conn: &Connection, plan_id: &str, event_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM plan_links WHERE plan_id=?1 AND activity_event_id=?2",
        params![plan_id, event_id],
    )?;
    conn.execute(
        "UPDATE activity_events SET plan_id=NULL, updated_at=?2 WHERE id=?1 AND plan_id=?3",
        params![event_id, now_rfc3339(), plan_id],
    )?;
    let remaining: i64 =
        conn.query_row("SELECT COUNT(*) FROM plan_links WHERE plan_id=?1", [plan_id], |r| r.get(0))?;
    if remaining == 0 {
        conn.execute(
            "UPDATE plans SET status='upcoming', updated_at=?2 WHERE id=?1 AND status='completed_linked'",
            params![plan_id, now_rfc3339()],
        )?;
    }
    Ok(())
}

/// Suggest link candidates for an event: same template or same date, unlinked plans.
pub fn suggest_plans_for_event(conn: &Connection, event_id: &str) -> Result<Vec<Value>> {
    let ev = crate::repo_daily::get_event(conn, event_id)?;
    let date = ev["local_date"].as_str().unwrap_or_default().to_string();
    let template = ev["template_id"].as_str().unwrap_or_default().to_string();
    let lo = add_days(&date, -1)?;
    let hi = add_days(&date, 1)?;
    query_json(
        conn,
        "SELECT p.* FROM plans p
         WHERE p.deleted_at IS NULL AND p.status IN ('upcoming','due')
           AND p.local_date >= ?1 AND p.local_date <= ?2
           AND (p.activity_template_id = ?3 OR p.local_date = ?4)
           AND p.id NOT IN (SELECT plan_id FROM plan_links)
         ORDER BY (p.local_date = ?4) DESC, p.local_date",
        params![lo, hi, template, date],
    )
}
