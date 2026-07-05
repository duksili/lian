//! Aggregated views: Today, weekly review, monthly review. These are computed
//! reads — nothing here writes derived state back into source records, and
//! missing days are reported as unknown, never as zero.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::jsonq::query_json;
use crate::util::{add_days, week_start};
use crate::{settings, Result};

/// Everything the Today surface needs in one round trip.
pub fn today_view(conn: &Connection) -> Result<Value> {
    let today = crate::repo_daily::today(conn)?;
    let yesterday = add_days(&today, -1)?;

    let plans = crate::repo_plans::list_plans(conn, &today, &today)?;
    let events_today = crate::repo_daily::list_events(conn, &today, &today)?;
    let recent_events = crate::repo_daily::list_events(conn, &add_days(&today, -2)?, &today)?;
    let checkins_today = crate::repo_daily::list_checkins(conn, &today, &today)?;
    let precepts_today = crate::repo_daily::get_precepts(conn, &today)?;
    let due_assessments = crate::repo_assess::due_today(conn)?;
    let determinations = crate::repo_determinations::list_determinations(conn, false)?;
    let due_reviews = crate::repo_determinations::due_for_review(conn, &today)?;
    let context = crate::repo_daily::list_context_events(conn, &today, &today)?;

    // Yesterday coverage: shown as "unreviewed", never as failure.
    let y_checkins = crate::repo_daily::list_checkins(conn, &yesterday, &yesterday)?;
    let y_events = crate::repo_daily::list_events(conn, &yesterday, &yesterday)?;
    let y_precepts = crate::repo_daily::get_precepts(conn, &yesterday)?;

    let settings_all = settings::get_all(conn)?;
    let baseline = baseline_status(&settings_all, &today)?;

    Ok(json!({
        "today": today,
        "yesterday": yesterday,
        "plans": plans,
        "events_today": events_today,
        "recent_events": recent_events,
        "checkins_today": checkins_today,
        "precepts_today": precepts_today,
        "due_assessments": due_assessments,
        "active_determinations": determinations,
        "determinations_due_review": due_reviews,
        "context_today": context,
        "yesterday_status": {
            "has_checkin": !y_checkins.is_empty(),
            "has_events": !y_events.is_empty(),
            "has_precepts": !y_precepts.is_null(),
        },
        "baseline": baseline,
    }))
}

fn baseline_status(settings_all: &Value, today: &str) -> Result<Value> {
    match settings_all["baseline_start"].as_str() {
        None => Ok(json!({ "state": "not_started" })),
        Some(start) => {
            let weeks = settings_all["baseline_weeks"].as_i64().unwrap_or(5);
            let end = add_days(start, weeks * 7)?;
            let state = if today < start {
                "not_started"
            } else if today <= end.as_str() {
                "in_baseline"
            } else {
                "complete"
            };
            let day_number = (crate::util::parse_date(today)? - crate::util::parse_date(start)?).num_days() + 1;
            Ok(json!({
                "state": state, "start": start, "end": end,
                "weeks": weeks, "day_number": day_number.max(0),
            }))
        }
    }
}

/// Weekly review: practice volume, plan vs actual, assessments + validity,
/// missing-data map, precepts and determinations (private), context, reflection.
pub fn weekly_review(conn: &Connection, any_date_in_week: &str) -> Result<Value> {
    let start = week_start(any_date_in_week)?;
    let end = add_days(&start, 6)?;
    let today = crate::repo_daily::today(conn)?;

    let events = crate::repo_daily::list_events(conn, &start, &end)?;
    let plans = crate::repo_plans::list_plans(conn, &start, &end)?;
    let checkins = crate::repo_daily::list_checkins(conn, &start, &end)?;
    let precepts = crate::repo_daily::list_precepts(conn, &start, &end)?;
    let context = crate::repo_daily::list_context_events(conn, &start, &end)?;
    let determinations = crate::repo_determinations::list_determinations(conn, false)?;

    let sessions = query_json(
        conn,
        "SELECT * FROM assessment_sessions WHERE local_date >= ?1 AND local_date <= ?2 ORDER BY started_at",
        [start.as_str(), end.as_str()],
    )?;

    // Practice volume per template.
    let volume = query_json(
        conn,
        "SELECT t.id AS template_id, t.name, t.glyph, t.color,
                COUNT(e.id) AS session_count,
                SUM(e.duration_seconds) AS total_seconds,
                SUM(CASE WHEN e.duration_seconds IS NULL THEN 1 ELSE 0 END) AS unknown_duration_count
         FROM activity_events e JOIN activity_templates t ON t.id = e.template_id
         WHERE e.deleted_at IS NULL AND e.status='completed' AND e.local_date >= ?1 AND e.local_date <= ?2
         GROUP BY t.id ORDER BY total_seconds DESC",
        [start.as_str(), end.as_str()],
    )?;

    // Data coverage per day. 'future' days are simply not yet.
    let mut coverage = Vec::new();
    let mut d = start.clone();
    while d <= end {
        let has_checkin = checkins.iter().any(|c| c["local_date"].as_str() == Some(d.as_str()));
        let has_events = events.iter().any(|e| e["local_date"].as_str() == Some(d.as_str()));
        let has_precepts = precepts.iter().any(|p| p["local_date"].as_str() == Some(d.as_str()));
        coverage.push(json!({
            "date": d,
            "is_future": d > today,
            "checkin": if d > today { "future" } else if has_checkin { "recorded" } else { "unknown" },
            "activity": if d > today { "future" } else if has_events { "recorded" } else { "unknown" },
            "precepts": if d > today { "future" } else if has_precepts { "recorded" } else { "unknown" },
        }));
        d = add_days(&d, 1)?;
    }

    let reflection = crate::jsonq::query_one(
        conn,
        "SELECT * FROM weekly_reflections WHERE week_start=?1",
        [start.as_str()],
    )?;

    Ok(json!({
        "week_start": start,
        "week_end": end,
        "events": events,
        "plans": plans,
        "checkins": checkins,
        "precepts": precepts,
        "context": context,
        "sessions": sessions,
        "volume": volume,
        "coverage": coverage,
        "determinations": determinations,
        "reflection": reflection,
    }))
}

pub fn save_weekly_reflection(conn: &Connection, week_start_date: &str, note: &str) -> Result<Value> {
    let start = week_start(week_start_date)?;
    let now = crate::util::now_rfc3339();
    conn.execute(
        "INSERT INTO weekly_reflections (id, week_start, note, logged_at, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?4,?4)
         ON CONFLICT(week_start) DO UPDATE SET note=?3, logged_at=?4, updated_at=?4",
        rusqlite::params![crate::util::new_id(), start, note, now],
    )?;
    Ok(crate::jsonq::query_one(conn, "SELECT * FROM weekly_reflections WHERE week_start=?1", [start.as_str()])?
        .unwrap_or(Value::Null))
}

/// Monthly review: per-week descriptive trends over a rolling window plus
/// check-in dimension trends and assessment metric trends. Purely descriptive.
pub fn monthly_review(conn: &Connection, from: &str, to: &str) -> Result<Value> {
    // Weekly practice volume buckets.
    let mut weeks: Vec<Value> = Vec::new();
    let mut w = week_start(from)?;
    let today = crate::repo_daily::today(conn)?;
    while w.as_str() <= to {
        let w_end = add_days(&w, 6)?;
        let volume = query_json(
            conn,
            "SELECT t.id AS template_id, t.name, t.color, t.glyph,
                    COUNT(e.id) AS session_count, SUM(e.duration_seconds) AS total_seconds
             FROM activity_events e JOIN activity_templates t ON t.id = e.template_id
             WHERE e.deleted_at IS NULL AND e.status='completed' AND e.local_date >= ?1 AND e.local_date <= ?2
             GROUP BY t.id",
            [w.as_str(), w_end.as_str()],
        )?;
        let checkin_days: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT local_date) FROM daily_checkins
             WHERE deleted_at IS NULL AND local_date >= ?1 AND local_date <= ?2",
            [w.as_str(), w_end.as_str()],
            |r| r.get(0),
        )?;
        weeks.push(json!({
            "week_start": w, "week_end": w_end, "is_current": w_end >= today,
            "volume": volume, "checkin_days": checkin_days,
        }));
        w = add_days(&w, 7)?;
    }

    // Dimension day-series for trend sparklines.
    let dims = query_json(
        conn,
        "SELECT * FROM checkin_dimensions WHERE is_enabled=1 ORDER BY sort_order",
        [],
    )?;
    let mut dim_series = Vec::new();
    for dim in &dims {
        let id = dim["id"].as_str().unwrap_or_default();
        let rows = query_json(
            conn,
            "SELECT c.local_date AS date, r.value FROM checkin_ratings r
             JOIN daily_checkins c ON c.id = r.checkin_id
             WHERE c.deleted_at IS NULL AND r.dimension_id=?1 AND c.local_date >= ?2 AND c.local_date <= ?3
             ORDER BY c.local_date, c.logged_at",
            rusqlite::params![id, from, to],
        )?;
        dim_series.push(json!({ "dimension": dim, "points": rows }));
    }

    // Assessment metric series (valid, non-familiarization by default).
    let metric_specs = [
        ("pvt_v1", "median_rt_ms", "PVT median RT (ms)"),
        ("pvt_v1", "lapse_rate", "PVT lapse rate"),
        ("go_no_go_v1", "commission_error_rate", "Go/No-Go commission rate"),
        ("go_no_go_v1", "go_rt_median_ms", "Go RT median (ms)"),
    ];
    let mut assessment_series = Vec::new();
    for (kind, metric, label) in metric_specs {
        let rows = query_json(
            conn,
            "SELECT local_date AS date, derived_metrics, validity_state, is_familiarization
             FROM assessment_sessions
             WHERE kind=?1 AND status='completed' AND local_date >= ?2 AND local_date <= ?3
             ORDER BY started_at",
            rusqlite::params![kind, from, to],
        )?;
        let points: Vec<Value> = rows
            .iter()
            .filter_map(|r| {
                let v = r["derived_metrics"][metric].as_f64()?;
                Some(json!({
                    "date": r["date"],
                    "value": v,
                    "validity_state": r["validity_state"],
                    "is_familiarization": r["is_familiarization"],
                }))
            })
            .collect();
        assessment_series.push(json!({ "kind": kind, "metric": metric, "label": label, "points": points }));
    }

    let context = crate::repo_daily::list_context_events(conn, from, to)?;

    Ok(json!({
        "from": from,
        "to": to,
        "weeks": weeks,
        "dimension_series": dim_series,
        "assessment_series": assessment_series,
        "context": context,
    }))
}
