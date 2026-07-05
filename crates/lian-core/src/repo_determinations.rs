//! Determinations: voluntary commitments with lifecycle, revision history,
//! private reviews, and explicit (never inferred) links to other records.

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::jsonq::{query_json, query_one, snapshot};
use crate::repo_daily::audit;
use crate::util::{new_id, now_rfc3339, parse_date};
use crate::{Error, Result};

const LIFECYCLES: &[&str] = &["active", "paused", "completed", "discontinued", "superseded"];
const REVIEW_STATUSES: &[&str] = &["kept", "not_kept", "uncertain", "not_reviewed"];
const CADENCES: &[&str] = &["daily", "weekly", "monthly"];

#[derive(Deserialize)]
pub struct DeterminationInput {
    pub id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub started_on: String,
    #[serde(default)]
    pub ends_on: Option<String>,
    #[serde(default)]
    pub review_cadence: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

pub fn save_determination(conn: &Connection, input: DeterminationInput) -> Result<Value> {
    if input.title.trim().is_empty() {
        return Err(Error::invalid("a determination needs wording"));
    }
    parse_date(&input.started_on)?;
    if let Some(e) = &input.ends_on {
        parse_date(e)?;
    }
    if let Some(c) = &input.review_cadence {
        if !CADENCES.contains(&c.as_str()) {
            return Err(Error::invalid("review cadence must be daily, weekly, or monthly"));
        }
    }
    let now = now_rfc3339();
    let id = match &input.id {
        Some(id) => {
            let prior = snapshot(conn, "determinations", id)?
                .ok_or_else(|| Error::not_found("determination"))?;
            // Editing wording preserves prior wording as a revision (contract invariant).
            let prior_title = prior["title"].as_str().unwrap_or_default();
            let prior_desc = prior["description"].as_str();
            if prior_title != input.title.trim()
                || prior_desc != input.description.as_deref()
            {
                conn.execute(
                    "INSERT INTO determination_revisions (id, determination_id, revised_at, prior_title, prior_description)
                     VALUES (?1,?2,?3,?4,?5)",
                    params![new_id(), id, now, prior_title, prior_desc],
                )?;
            }
            conn.execute(
                "UPDATE determinations SET title=?2, description=?3, started_on=?4, ends_on=?5,
                   review_cadence=?6, category=?7, updated_at=?8 WHERE id=?1",
                params![
                    id, input.title.trim(), input.description, input.started_on, input.ends_on,
                    input.review_cadence, input.category, now
                ],
            )?;
            audit(conn, "determination", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO determinations
                   (id, title, description, started_on, ends_on, review_cadence, category, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
                params![
                    id, input.title.trim(), input.description, input.started_on, input.ends_on,
                    input.review_cadence, input.category, now
                ],
            )?;
            id
        }
    };
    get_determination(conn, &id)
}

/// Explicit lifecycle transitions; each is auditable and reversible in wording.
pub fn set_lifecycle(conn: &Connection, id: &str, state: &str) -> Result<Value> {
    if !LIFECYCLES.contains(&state) {
        return Err(Error::invalid(format!("unknown lifecycle state '{state}'")));
    }
    let prior = snapshot(conn, "determinations", id)?.ok_or_else(|| Error::not_found("determination"))?;
    let now = now_rfc3339();
    let stamp_col = match state {
        "completed" => Some("completed_at"),
        "paused" => Some("paused_at"),
        "discontinued" => Some("discontinued_at"),
        _ => None,
    };
    match stamp_col {
        Some(col) => {
            let sql = format!(
                "UPDATE determinations SET lifecycle_state=?2, {col}=?3, updated_at=?3 WHERE id=?1"
            );
            conn.execute(&sql, params![id, state, now])?;
        }
        None => {
            conn.execute(
                "UPDATE determinations SET lifecycle_state=?2, updated_at=?3 WHERE id=?1",
                params![id, state, now],
            )?;
        }
    }
    audit(conn, "determination", id, "lifecycle", Some(&prior), None)?;
    get_determination(conn, id)
}

/// Supersede: old determination keeps its wording and history; the new one
/// carries a predecessor link.
pub fn supersede(conn: &Connection, id: &str, input: DeterminationInput) -> Result<Value> {
    let old = snapshot(conn, "determinations", id)?.ok_or_else(|| Error::not_found("determination"))?;
    let new = save_determination(
        conn,
        DeterminationInput { id: None, ..input },
    )?;
    let new_id_str = new["id"].as_str().unwrap_or_default().to_string();
    let now = now_rfc3339();
    conn.execute(
        "UPDATE determinations SET lifecycle_state='superseded', superseded_by_id=?2, updated_at=?3 WHERE id=?1",
        params![id, new_id_str, now],
    )?;
    conn.execute(
        "UPDATE determinations SET predecessor_id=?2, updated_at=?3 WHERE id=?1",
        params![new_id_str, id, now],
    )?;
    audit(conn, "determination", id, "lifecycle", Some(&old), Some("superseded"))?;
    get_determination(conn, &new_id_str)
}

pub fn get_determination(conn: &Connection, id: &str) -> Result<Value> {
    let mut d = query_one(conn, "SELECT * FROM determinations WHERE id=?1", [id])?
        .ok_or_else(|| Error::not_found("determination"))?;
    d["revisions"] = json!(query_json(
        conn,
        "SELECT * FROM determination_revisions WHERE determination_id=?1 ORDER BY revised_at DESC",
        [id],
    )?);
    d["reviews"] = json!(query_json(
        conn,
        "SELECT * FROM determination_reviews WHERE determination_id=?1 ORDER BY local_date DESC",
        [id],
    )?);
    d["links"] = json!(query_json(
        conn,
        "SELECT * FROM determination_links WHERE determination_id=?1 ORDER BY created_at DESC",
        [id],
    )?);
    Ok(d)
}

pub fn list_determinations(conn: &Connection, include_closed: bool) -> Result<Vec<Value>> {
    let sql = if include_closed {
        "SELECT * FROM determinations ORDER BY
           CASE lifecycle_state WHEN 'active' THEN 0 WHEN 'paused' THEN 1 ELSE 2 END, started_on DESC"
    } else {
        "SELECT * FROM determinations WHERE lifecycle_state IN ('active','paused') ORDER BY started_on DESC"
    };
    let mut rows = query_json(conn, sql, [])?;
    for d in &mut rows {
        let id = d["id"].as_str().unwrap_or_default().to_string();
        d["last_review"] = query_one(
            conn,
            "SELECT * FROM determination_reviews WHERE determination_id=?1 ORDER BY local_date DESC LIMIT 1",
            [&id],
        )?
        .unwrap_or(Value::Null);
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM determination_links WHERE determination_id=?1",
            [&id],
            |r| r.get(0),
        )?;
        d["link_count"] = json!(n);
    }
    Ok(rows)
}

#[derive(Deserialize)]
pub struct ReviewInput {
    pub determination_id: String,
    pub local_date: String,
    pub status: String,
    #[serde(default)]
    pub note: Option<String>,
}

pub fn save_review(conn: &Connection, input: ReviewInput) -> Result<Value> {
    if !REVIEW_STATUSES.contains(&input.status.as_str()) {
        return Err(Error::invalid(format!("invalid review status '{}'", input.status)));
    }
    parse_date(&input.local_date)?;
    // Reviews only exist where the determination has a review cadence/rule.
    let cadence: Option<String> = conn
        .query_row(
            "SELECT review_cadence FROM determinations WHERE id=?1",
            [&input.determination_id],
            |r| r.get(0),
        )
        .map_err(|_| Error::not_found("determination"))?;
    if cadence.is_none() {
        return Err(Error::invalid(
            "this determination has no review rule; add a review cadence first",
        ));
    }
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO determination_reviews (id, determination_id, local_date, logged_at, status, note)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(determination_id, local_date) DO UPDATE SET status=?5, note=?6, logged_at=?4",
        params![new_id(), input.determination_id, input.local_date, now, input.status, input.note],
    )?;
    get_determination(conn, &input.determination_id)
}

#[derive(Deserialize)]
pub struct LinkInput {
    pub determination_id: String,
    pub linked_type: String,
    pub linked_id: String,
}

pub fn add_link(conn: &Connection, input: LinkInput) -> Result<()> {
    if !["plan", "activity_event", "checkin", "context_event", "note"].contains(&input.linked_type.as_str()) {
        return Err(Error::invalid("unknown link type"));
    }
    conn.execute(
        "INSERT OR IGNORE INTO determination_links (id, determination_id, linked_type, linked_id, created_at)
         VALUES (?1,?2,?3,?4,?5)",
        params![new_id(), input.determination_id, input.linked_type, input.linked_id, now_rfc3339()],
    )?;
    Ok(())
}

pub fn remove_link(conn: &Connection, link_id: &str) -> Result<()> {
    conn.execute("DELETE FROM determination_links WHERE id=?1", [link_id])?;
    Ok(())
}

/// Active determinations whose cadence makes them due for private review today.
pub fn due_for_review(conn: &Connection, today: &str) -> Result<Vec<Value>> {
    let rows = query_json(
        conn,
        "SELECT d.* FROM determinations d
         WHERE d.lifecycle_state='active' AND d.review_cadence IS NOT NULL
           AND (d.ends_on IS NULL OR d.ends_on >= ?1) AND d.started_on <= ?1",
        [today],
    )?;
    let mut due = Vec::new();
    for d in rows {
        let id = d["id"].as_str().unwrap_or_default().to_string();
        let cadence = d["review_cadence"].as_str().unwrap_or("weekly");
        let last: Option<String> = conn
            .query_row(
                "SELECT MAX(local_date) FROM determination_reviews WHERE determination_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap_or(None);
        let gap_needed = match cadence {
            "daily" => 1,
            "weekly" => 7,
            _ => 28,
        };
        let is_due = match last {
            None => true,
            Some(l) => {
                let last_d = parse_date(&l)?;
                let today_d = parse_date(today)?;
                (today_d - last_d).num_days() >= gap_needed
            }
        };
        if is_due {
            due.push(d);
        }
    }
    Ok(due)
}
