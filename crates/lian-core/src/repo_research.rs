//! Research protocols: predefined hypothesis/outcome/analysis plans with
//! version discipline — once results have been viewed, changing the outcome
//! or analysis plan requires a new protocol version.

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::jsonq::{query_json, query_one, snapshot};
use crate::repo_daily::audit;
use crate::util::{new_id, now_rfc3339};
use crate::{Error, Result};

const STATUSES: &[&str] = &["draft", "planned", "active", "paused", "completed", "cancelled", "superseded"];
const CONCLUSIONS: &[&str] = &[
    "protocol_result_inconclusive",
    "protocol_result_supported",
    "protocol_result_not_supported",
];

#[derive(Deserialize)]
pub struct ProtocolInput {
    pub id: Option<String>,
    pub title: String,
    pub question: String,
    pub hypothesis: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    pub primary_outcome_definition: Value,
    pub intervention_definition: String,
    pub analysis_plan: String,
    #[serde(default)]
    pub secondary_outcomes: Option<String>,
    #[serde(default)]
    pub adherence_requirements: Option<String>,
    #[serde(default)]
    pub context_variables: Option<String>,
    #[serde(default)]
    pub stop_criteria: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

pub fn save_protocol(conn: &Connection, input: ProtocolInput) -> Result<Value> {
    for (field, v) in [("title", &input.title), ("question", &input.question), ("hypothesis", &input.hypothesis)] {
        if v.trim().is_empty() {
            return Err(Error::invalid(format!("protocol {field} is required")));
        }
    }
    let now = now_rfc3339();
    let id = match &input.id.clone() {
        Some(id) => {
            let prior = snapshot(conn, "research_protocols", id)?
                .ok_or_else(|| Error::not_found("protocol"))?;
            let locked = prior["results_locked"].as_i64().unwrap_or(0) == 1;
            let outcome_changed = prior["primary_outcome_definition"] != input.primary_outcome_definition
                || prior["analysis_plan"].as_str() != Some(input.analysis_plan.as_str())
                || prior["hypothesis"].as_str() != Some(input.hypothesis.as_str());
            if locked && outcome_changed {
                // Amendment after results were viewed -> new version, old superseded.
                return amend_protocol(conn, id, input);
            }
            conn.execute(
                "UPDATE research_protocols SET title=?2, question=?3, hypothesis=?4, start_date=?5, end_date=?6,
                   primary_outcome_definition=?7, intervention_definition=?8, analysis_plan=?9,
                   secondary_outcomes=?10, adherence_requirements=?11, context_variables=?12,
                   stop_criteria=?13, notes=?14, updated_at=?15
                 WHERE id=?1",
                params![
                    id, input.title.trim(), input.question, input.hypothesis, input.start_date, input.end_date,
                    input.primary_outcome_definition.to_string(), input.intervention_definition,
                    input.analysis_plan, input.secondary_outcomes, input.adherence_requirements,
                    input.context_variables, input.stop_criteria, input.notes, now
                ],
            )?;
            audit(conn, "research_protocol", id, "update", Some(&prior), None)?;
            id.clone()
        }
        None => {
            let id = new_id();
            conn.execute(
                "INSERT INTO research_protocols
                   (id, title, version, question, hypothesis, status, start_date, end_date,
                    primary_outcome_definition, intervention_definition, analysis_plan,
                    secondary_outcomes, adherence_requirements, context_variables, stop_criteria,
                    notes, created_at, updated_at)
                 VALUES (?1,?2,1,?3,?4,'draft',?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15)",
                params![
                    id, input.title.trim(), input.question, input.hypothesis, input.start_date, input.end_date,
                    input.primary_outcome_definition.to_string(), input.intervention_definition,
                    input.analysis_plan, input.secondary_outcomes, input.adherence_requirements,
                    input.context_variables, input.stop_criteria, input.notes, now
                ],
            )?;
            id
        }
    };
    get_protocol(conn, &id)
}

fn amend_protocol(conn: &Connection, old_id: &str, input: ProtocolInput) -> Result<Value> {
    let old = snapshot(conn, "research_protocols", old_id)?.ok_or_else(|| Error::not_found("protocol"))?;
    let old_version = old["version"].as_i64().unwrap_or(1);
    let now = now_rfc3339();
    let id = new_id();
    conn.execute(
        "INSERT INTO research_protocols
           (id, title, version, question, hypothesis, status, start_date, end_date,
            primary_outcome_definition, intervention_definition, analysis_plan,
            secondary_outcomes, adherence_requirements, context_variables, stop_criteria,
            notes, predecessor_id, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,'draft',?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?17)",
        params![
            id, input.title.trim(), old_version + 1, input.question, input.hypothesis,
            input.start_date, input.end_date, input.primary_outcome_definition.to_string(),
            input.intervention_definition, input.analysis_plan, input.secondary_outcomes,
            input.adherence_requirements, input.context_variables, input.stop_criteria,
            input.notes, old_id, now
        ],
    )?;
    conn.execute(
        "UPDATE research_protocols SET status='superseded', updated_at=?2 WHERE id=?1",
        params![old_id, now],
    )?;
    audit(conn, "research_protocol", old_id, "lifecycle", Some(&old), Some("amended after results viewed"))?;
    get_protocol(conn, &id)
}

pub fn set_protocol_status(conn: &Connection, id: &str, status: &str) -> Result<Value> {
    if !STATUSES.contains(&status) {
        return Err(Error::invalid(format!("unknown protocol status '{status}'")));
    }
    let prior = snapshot(conn, "research_protocols", id)?.ok_or_else(|| Error::not_found("protocol"))?;
    conn.execute(
        "UPDATE research_protocols SET status=?2, updated_at=?3 WHERE id=?1",
        params![id, status, now_rfc3339()],
    )?;
    audit(conn, "research_protocol", id, "lifecycle", Some(&prior), None)?;
    get_protocol(conn, id)
}

/// Conclude with one of the approved protocol evidence labels; null and
/// negative results are first-class outcomes.
pub fn conclude_protocol(conn: &Connection, id: &str, conclusion: &str, note: Option<&str>) -> Result<Value> {
    if !CONCLUSIONS.contains(&conclusion) {
        return Err(Error::invalid("conclusion must be one of the protocol evidence labels"));
    }
    let prior = snapshot(conn, "research_protocols", id)?.ok_or_else(|| Error::not_found("protocol"))?;
    conn.execute(
        "UPDATE research_protocols SET status='completed', conclusion=?2, conclusion_note=?3, updated_at=?4
         WHERE id=?1",
        params![id, conclusion, note, now_rfc3339()],
    )?;
    audit(conn, "research_protocol", id, "lifecycle", Some(&prior), Some("concluded"))?;
    get_protocol(conn, id)
}

/// Mark that results were viewed: from here on, outcome/plan edits fork a new version.
pub fn lock_protocol_results(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE research_protocols SET results_locked=1, updated_at=?2 WHERE id=?1",
        params![id, now_rfc3339()],
    )?;
    Ok(())
}

pub fn get_protocol(conn: &Connection, id: &str) -> Result<Value> {
    let mut p = query_one(conn, "SELECT * FROM research_protocols WHERE id=?1", [id])?
        .ok_or_else(|| Error::not_found("protocol"))?;
    p["results"] = json!(query_json(
        conn,
        "SELECT * FROM analysis_results WHERE protocol_id=?1 ORDER BY generated_at DESC",
        [id],
    )?);
    Ok(p)
}

pub fn list_protocols(conn: &Connection) -> Result<Vec<Value>> {
    query_json(
        conn,
        "SELECT * FROM research_protocols ORDER BY
           CASE status WHEN 'active' THEN 0 WHEN 'planned' THEN 1 WHEN 'draft' THEN 2 WHEN 'paused' THEN 3 ELSE 4 END,
           created_at DESC",
        [],
    )
}
