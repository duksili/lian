//! Reminder rules and due-notification computation.
//!
//! The desktop shell polls [`due_notifications`] on a short interval and
//! delivers what it returns. All safeguards live here so they are testable:
//! quiet hours, global pause, snooze, per-rule enable, privacy-minimal text,
//! dedupe (a reminder fires once), and burst suppression (a reminder whose
//! moment passed more than a grace window ago is skipped, not queued).

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::jsonq::query_json;
use crate::util::{in_window, new_id, now_rfc3339, parse_hhmm, weekday_index};
use crate::{settings, Error, Result};

/// Minutes after the scheduled moment during which a reminder may still fire.
/// Anything older is dropped silently — no stacked catch-up notices.
const GRACE_MINUTES: i64 = 45;

pub fn list_rules(conn: &Connection) -> Result<Vec<Value>> {
    query_json(conn, "SELECT * FROM reminder_rules ORDER BY kind, label", [])
}

#[derive(Deserialize)]
pub struct RuleInput {
    pub id: Option<String>,
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub time_of_day: Option<String>,
    #[serde(default)]
    pub weekdays: Vec<i64>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default = "crate::reminders::default_true")]
    pub enabled: bool,
}

pub(crate) fn default_true() -> bool {
    true
}

pub fn save_rule(conn: &Connection, input: RuleInput) -> Result<Value> {
    if let Some(t) = &input.time_of_day {
        parse_hhmm(t)?;
    }
    let now = now_rfc3339();
    let id = match &input.id {
        Some(id) => {
            conn.execute(
                "UPDATE reminder_rules SET kind=?2, label=?3, time_of_day=?4, weekdays=?5, target_id=?6,
                   enabled=?7, updated_at=?8 WHERE id=?1",
                params![
                    id, input.kind, input.label, input.time_of_day,
                    serde_json::to_string(&input.weekdays)?, input.target_id,
                    input.enabled as i64, now
                ],
            )?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO reminder_rules (id, kind, label, time_of_day, weekdays, target_id, enabled, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
                params![
                    id, input.kind, input.label, input.time_of_day,
                    serde_json::to_string(&input.weekdays)?, input.target_id,
                    input.enabled as i64, now
                ],
            )?;
            id
        }
    };
    crate::jsonq::query_one(conn, "SELECT * FROM reminder_rules WHERE id=?1", [&id])?
        .ok_or_else(|| Error::not_found("reminder rule"))
}

pub fn set_rule_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "UPDATE reminder_rules SET enabled=?2, snoozed_until=NULL, updated_at=?3 WHERE id=?1",
        params![id, enabled as i64, now_rfc3339()],
    )?;
    Ok(())
}

pub fn snooze_rule(conn: &Connection, id: &str, minutes: i64) -> Result<()> {
    let until = chrono::Utc::now() + chrono::Duration::minutes(minutes.max(1));
    conn.execute(
        "UPDATE reminder_rules SET snoozed_until=?2, updated_at=?3 WHERE id=?1",
        params![id, until.to_rfc3339_opts(chrono::SecondsFormat::Millis, true), now_rfc3339()],
    )?;
    Ok(())
}

pub fn delete_rule(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM reminder_rules WHERE id=?1", [id])?;
    Ok(())
}

/// True when `occurrence` is within [now - grace, now]; the grace window is
/// how long after its moment a reminder may still fire (no catch-up beyond).
fn in_grace(now: &chrono::DateTime<chrono::Utc>, occurrence_rfc3339: &str) -> bool {
    match crate::util::parse_instant(occurrence_rfc3339) {
        Ok(occ) => {
            let delta = now.signed_duration_since(occ.with_timezone(&chrono::Utc));
            delta >= chrono::Duration::zero() && delta <= chrono::Duration::minutes(GRACE_MINUTES)
        }
        Err(_) => false,
    }
}

fn instant_lte_now(now: &chrono::DateTime<chrono::Utc>, rfc3339: &str) -> bool {
    match crate::util::parse_instant(rfc3339) {
        Ok(t) => t.with_timezone(&chrono::Utc) <= *now,
        Err(_) => true, // malformed timestamps never keep something suppressed
    }
}

/// Compute the notifications that should fire right now. Returns at most one
/// non-critical notification per call (documented default behavior).
pub fn due_notifications(conn: &Connection) -> Result<Vec<Value>> {
    due_notifications_at(conn, chrono::Utc::now())
}

/// Instant-injected variant so pause expiry, grace windows, and cross-midnight
/// behavior are all deterministically testable. All comparisons are between
/// timezone-aware instants; HH:MM strings are only inputs for computing the
/// local occurrence instant of a rule.
pub fn due_notifications_at(conn: &Connection, now: chrono::DateTime<chrono::Utc>) -> Result<Vec<Value>> {
    let s = settings::get_all(conn)?;
    // Global pause: a time-bounded pause clears itself once expired; a pause
    // without an expiry is indefinite until the user resumes.
    if s["notifications_paused"].as_bool().unwrap_or(false) {
        match s["notifications_pause_until"].as_str() {
            Some(until) if instant_lte_now(&now, until) => {
                set_pause(conn, false, None)?;
            }
            _ => return Ok(vec![]),
        }
    }
    let tz = settings::timezone(conn)?;
    let tz_parsed = crate::util::parse_tz(&tz)?;
    let now_local = now.with_timezone(&tz_parsed);
    let now_hhmm = now_local.format("%H:%M").to_string();
    // Quiet hours are mandatory (daily local window, may cross midnight).
    let qs = s["quiet_hours_start"].as_str().unwrap_or("21:30");
    let qe = s["quiet_hours_end"].as_str().unwrap_or("07:30");
    if in_window(&now_hhmm, qs, qe)? {
        return Ok(vec![]);
    }

    let today = now_local.date_naive().format("%Y-%m-%d").to_string();
    let yesterday = crate::util::add_days(&today, -1)?;
    let tomorrow = crate::util::add_days(&today, 1)?;
    let minimal = s["lock_screen_minimal"].as_bool().unwrap_or(true);
    let mut candidates: Vec<Value> = Vec::new();

    // Rule-based reminders. A rule occurs at (date, time_of_day) in the local
    // timezone; we consider yesterday's and today's occurrences so a grace
    // window that crosses midnight still works.
    for rule in list_rules(conn)? {
        if rule["enabled"].as_i64().unwrap_or(0) == 0 {
            continue;
        }
        if let Some(sn) = rule["snoozed_until"].as_str() {
            if !instant_lte_now(&now, sn) {
                continue;
            }
        }
        let kind = rule["kind"].as_str().unwrap_or_default().to_string();
        let time = match rule["time_of_day"].as_str() {
            Some(t) => t.to_string(),
            None => continue,
        };
        let weekdays: Vec<i64> = serde_json::from_value(rule["weekdays"].clone()).unwrap_or_default();

        for date in [&yesterday, &today] {
            // Weekday/monthly filters apply to the occurrence's own date.
            if !weekdays.is_empty() && !weekdays.contains(&(weekday_index(date)? as i64)) {
                continue;
            }
            if kind == "monthly_review" && !date.ends_with("-01") {
                continue;
            }
            let occurrence = match crate::util::local_to_instant(date, &time, &tz) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if !in_grace(&now, &occurrence) {
                continue;
            }
            // Determination review prompts only when something is actually due.
            if kind == "determination_review"
                && crate::repo_determinations::due_for_review(conn, &today)?.is_empty()
            {
                continue;
            }
            // Recovery prompt only when yesterday genuinely has no entries yet.
            if kind == "recovery" {
                let has_any: i64 = conn.query_row(
                    "SELECT (SELECT COUNT(*) FROM activity_events WHERE local_date=?1 AND deleted_at IS NULL)
                          + (SELECT COUNT(*) FROM daily_checkins WHERE local_date=?1 AND deleted_at IS NULL)",
                    [&yesterday],
                    |r| r.get(0),
                )?;
                if has_any > 0 {
                    continue;
                }
            }
            let rule_id = rule["id"].as_str().unwrap_or_default().to_string();
            let dedupe = format!("rule:{rule_id}:{date}");
            // Neutral, factual text. Privacy: determination titles stay out of
            // notifications by default; Five Precepts content never appears.
            let (title, body) = match kind.as_str() {
                "evening_checkin" => ("Evening check-in".to_string(),
                    "A quiet moment to record today, if you wish.".to_string()),
                "weekly_review" => ("Weekly review".to_string(),
                    "This week's record is ready to look over.".to_string()),
                "monthly_review" => ("Monthly review".to_string(),
                    "A month of observations is ready for review.".to_string()),
                "determination_review" => ("Determination review".to_string(),
                    if minimal { "A determination is due for private review.".to_string() }
                    else { rule["label"].as_str().unwrap_or("A determination is due for review.").to_string() }),
                "recovery" => ("Yesterday".to_string(),
                    "Yesterday has entries you may still want to add. No action needed.".to_string()),
                _ => (rule["label"].as_str().unwrap_or("Reminder").to_string(), String::new()),
            };
            candidates.push(json!({
                "dedupe_key": dedupe, "rule_id": rule_id, "plan_id": null,
                "kind": kind, "title": title, "body": body,
            }));
        }
    }

    // Plan reminders: fire `reminder_offset_minutes` before scheduled start.
    // The date range covers yesterday..tomorrow so an offset that reaches
    // back across midnight (e.g. 00:10 plan, 30 min notice) is not missed.
    let plans = crate::repo_plans::list_plans(conn, &yesterday, &tomorrow)?;
    for p in &plans {
        let offset = match p["reminder_offset_minutes"].as_i64() {
            Some(o) => o,
            None => continue,
        };
        let start = match p["scheduled_start"].as_str() {
            Some(st) => st.to_string(),
            None => continue,
        };
        let status = p["effective_status"].as_str().unwrap_or("");
        if !["due", "upcoming"].contains(&status) {
            continue;
        }
        let start_instant = match crate::util::parse_instant(&start) {
            Ok(t) => t.with_timezone(&chrono::Utc),
            Err(_) => continue,
        };
        let fire_at = (start_instant - chrono::Duration::minutes(offset))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        if !in_grace(&now, &fire_at) {
            continue;
        }
        let start_local = start_instant.with_timezone(&tz_parsed).format("%H:%M").to_string();
        let plan_id = p["id"].as_str().unwrap_or_default().to_string();
        let dedupe = format!("plan:{plan_id}");
        let title = p["title"].as_str().unwrap_or("Planned activity").to_string();
        candidates.push(json!({
            "dedupe_key": dedupe, "rule_id": null, "plan_id": plan_id,
            "kind": "plan", "title": if minimal { "Planned activity".into() } else { title },
            "body": format!("Planned for {start_local}."),
        }));
    }

    // Assessment window reminders: once, at window start, for due assessments.
    for due in crate::repo_assess::due_today(conn)? {
        let kind = due["kind"].as_str().unwrap_or_default().to_string();
        let ws = due["window_start"].as_str().unwrap_or("07:00");
        let occurrence = match crate::util::local_to_instant(&today, ws, &tz) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !in_grace(&now, &occurrence) {
            continue;
        }
        let dedupe = format!("assessment:{kind}:{today}");
        let label = match kind.as_str() {
            "pvt_v1" => "PVT",
            "go_no_go_v1" => "Go/No-Go",
            _ => "Weekly physical check",
        };
        candidates.push(json!({
            "dedupe_key": dedupe, "rule_id": null, "plan_id": null,
            "kind": "assessment_window",
            "title": format!("{label} window open"),
            "body": "An assessment is available in its usual window.",
        }));
    }

    // Filter already-fired, keep at most one.
    let mut out = Vec::new();
    for c in candidates {
        let dedupe = c["dedupe_key"].as_str().unwrap_or_default();
        let seen: i64 = conn.query_row(
            "SELECT COUNT(*) FROM notification_log WHERE dedupe_key=?1",
            [dedupe],
            |r| r.get(0),
        )?;
        if seen == 0 {
            out.push(c);
            break; // at most one non-critical notification at a time
        }
    }
    Ok(out)
}

/// Record delivery so the same logical reminder never fires twice.
pub fn record_fired(conn: &Connection, notification: &Value) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO notification_log (id, rule_id, plan_id, kind, dedupe_key, fired_at, title, body)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            new_id(),
            notification["rule_id"].as_str(),
            notification["plan_id"].as_str(),
            notification["kind"].as_str().unwrap_or("unknown"),
            notification["dedupe_key"].as_str().unwrap_or_default(),
            now_rfc3339(),
            notification["title"].as_str().unwrap_or(""),
            notification["body"].as_str().unwrap_or(""),
        ],
    )?;
    Ok(())
}

/// Global pause until an instant (or indefinitely when None).
pub fn set_pause(conn: &Connection, paused: bool, until: Option<String>) -> Result<()> {
    settings::set(conn, "notifications_paused", &json!(paused))?;
    settings::set(conn, "notifications_pause_until", &json!(until))?;
    Ok(())
}
