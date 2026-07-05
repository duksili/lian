//! Assessment session persistence: start/finalize/abort, raw trial storage,
//! validity metadata, familiarization tagging, and schedules.

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::assessments::{self, RawTrial, SessionContext};
use crate::jsonq::{query_json, query_one, snapshot};
use crate::repo_daily::audit;
use crate::util::{new_id, now_rfc3339, now_local_hhmm, in_window, weekday_index};
use crate::{settings, Error, Result};

pub const KINDS: &[&str] = &[assessments::PVT_KIND, assessments::GNG_KIND, assessments::PHYSICAL_KIND];

fn protocol_version_for(kind: &str) -> Result<&'static str> {
    match kind {
        k if k == assessments::PVT_KIND => Ok(assessments::PVT_PROTOCOL_VERSION),
        k if k == assessments::GNG_KIND => Ok(assessments::GNG_PROTOCOL_VERSION),
        k if k == assessments::PHYSICAL_KIND => Ok(assessments::PHYSICAL_PROTOCOL_VERSION),
        other => Err(Error::invalid(format!("unknown assessment kind '{other}'"))),
    }
}

#[derive(Deserialize)]
pub struct StartSessionInput {
    pub kind: String,
    #[serde(default)]
    pub input_method: Option<String>,
    #[serde(default)]
    pub device_metadata: Option<Value>,
    #[serde(default)]
    pub pre_test: Option<Value>,
    #[serde(default)]
    pub is_familiarization: bool,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub protocol_id: Option<String>,
}

/// Create an in-progress session and return it together with the seeded
/// protocol sequence the UI must follow.
pub fn start_session(conn: &Connection, input: StartSessionInput) -> Result<Value> {
    protocol_version_for(&input.kind)?;
    let tz = settings::timezone(conn)?;
    let now = now_rfc3339();
    let today = crate::repo_daily::today(conn)?;
    let id = new_id();
    let seed: u64 = rand::random();

    conn.execute(
        "INSERT INTO assessment_sessions
           (id, kind, protocol_version, status, started_at, timezone, input_method, device_metadata,
            pre_test, is_familiarization, session_seed, plan_id, protocol_id, local_date, created_at, updated_at)
         VALUES (?1,?2,?3,'in_progress',?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14)",
        params![
            id, input.kind, protocol_version_for(&input.kind)?, now, tz,
            input.input_method,
            serde_json::to_string(&input.device_metadata.unwrap_or(json!({})))?,
            serde_json::to_string(&input.pre_test.unwrap_or(json!({})))?,
            input.is_familiarization as i64, seed.to_string(),
            input.plan_id, input.protocol_id, today, now
        ],
    )?;

    let sequence: Value = match input.kind.as_str() {
        k if k == assessments::PVT_KIND => json!({
            "kind": "pvt_v1",
            "duration_ms": assessments::PVT_DURATION_MS,
            "timeout_ms": assessments::PVT_TIMEOUT_MS,
            "false_start_ms": assessments::PVT_FALSE_START_MS,
            "lapse_ms": assessments::PVT_LAPSE_MS,
            "intervals_ms": assessments::pvt_schedule(seed),
        }),
        k if k == assessments::GNG_KIND => json!({
            "kind": "go_no_go_v1",
            "stimulus_ms": assessments::GNG_STIMULUS_MS,
            "trials": assessments::gng_sequence(seed).iter().map(|(kind, isi)| json!({
                "stimulus": match kind { assessments::GngStimulus::Go => "go", assessments::GngStimulus::NoGo => "no_go" },
                "isi_ms": isi,
            })).collect::<Vec<_>>(),
        }),
        _ => json!({ "kind": "physical_weekly_v1" }),
    };

    let session = get_session(conn, &id)?;
    Ok(json!({ "session": session, "sequence": sequence }))
}

#[derive(Deserialize)]
pub struct FinalizeInput {
    pub session_id: String,
    pub trials: Vec<RawTrial>,
    #[serde(default)]
    pub context: SessionContext,
    #[serde(default)]
    pub note: Option<String>,
}

/// Persist raw trials, derive versioned metrics, evaluate validity, and mark
/// the session completed (or aborted-with-data). Raw data is always retained.
pub fn finalize_session(conn: &Connection, input: FinalizeInput) -> Result<Value> {
    let session = query_one(conn, "SELECT * FROM assessment_sessions WHERE id=?1", [&input.session_id])?
        .ok_or_else(|| Error::not_found("assessment session"))?;
    let kind = session["kind"].as_str().unwrap_or_default().to_string();
    let now = now_rfc3339();

    // Deviation-from-window flag for validity context.
    let mut ctx = input.context;
    ctx.configured_input_method = settings::get_string(conn, "assessment_input_method")?;
    if !ctx.outside_window {
        ctx.outside_window = !is_inside_window_now(conn, &kind)?;
    }

    for t in &input.trials {
        let derived = if kind == assessments::PVT_KIND {
            let c = assessments::classify_pvt_trial(t);
            (c.is_false_start, c.is_lapse, c.is_omission, false, Value::Null)
        } else if kind == assessments::GNG_KIND {
            let is_go = t.stimulus_kind.as_deref() == Some("go");
            let responded = t.response_ms.is_some();
            let commission = !is_go && responded;
            let omission = is_go && !responded;
            let correct = (is_go && responded) || (!is_go && !responded);
            (false, false, omission, commission, Value::Bool(correct))
        } else {
            (false, false, false, false, Value::Null)
        };
        conn.execute(
            "INSERT INTO assessment_trials
               (id, session_id, trial_index, stimulus_kind, planned_interval_ms, onset_ms, response_ms,
                reaction_time_ms, is_false_start, is_lapse, is_omission, is_commission_error, is_correct,
                visibility_lost, payload)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                new_id(), input.session_id, t.trial_index, t.stimulus_kind, t.planned_interval_ms,
                t.onset_ms, t.response_ms, t.reaction_time_ms,
                derived.0 as i64, derived.1 as i64, derived.2 as i64, derived.3 as i64,
                match derived.4 { Value::Bool(b) => Some(b as i64), _ => None },
                t.visibility_lost as i64,
                serde_json::to_string(&t.payload.clone().unwrap_or(json!({})))?
            ],
        )?;
    }

    let metrics = match kind.as_str() {
        k if k == assessments::PVT_KIND => assessments::pvt_metrics(&input.trials),
        k if k == assessments::GNG_KIND => assessments::gng_metrics(&input.trials),
        _ => assessments::physical_metrics(&input.trials),
    };
    let (validity_state, reasons) = assessments::evaluate_validity(&kind, &input.trials, &ctx);
    let status = if ctx.aborted { "aborted" } else { "completed" };

    conn.execute(
        "UPDATE assessment_sessions SET status=?2, ended_at=?3, derived_metrics=?4, metrics_version=?5,
           validity_state=?6, validity_reasons=?7, visibility_lost_count=?8,
           self_reported_interruption=?9, note=?10, updated_at=?3
         WHERE id=?1",
        params![
            input.session_id, status, now, serde_json::to_string(&metrics)?,
            assessments::METRICS_VERSION, validity_state, serde_json::to_string(&reasons)?,
            ctx.visibility_lost_count, ctx.self_reported_interruption, input.note
        ],
    )?;

    if let Some(plan_id) = session["plan_id"].as_str() {
        crate::repo_plans::link_session_to_plan(conn, plan_id, &input.session_id)?;
    }

    get_session(conn, &input.session_id)
}

/// Abandon before any usable data: the session row is kept as 'aborted' with
/// whatever partial information exists (interrupted sessions remain visible).
pub fn abort_session(conn: &Connection, session_id: &str, reason: Option<&str>) -> Result<Value> {
    let prior = snapshot(conn, "assessment_sessions", session_id)?
        .ok_or_else(|| Error::not_found("assessment session"))?;
    conn.execute(
        "UPDATE assessment_sessions SET status='aborted', ended_at=?2, validity_state='invalid',
           validity_reasons=?3, note=COALESCE(?4, note), updated_at=?2
         WHERE id=?1",
        params![session_id, now_rfc3339(), json!(["session_aborted_early"]).to_string(), reason],
    )?;
    audit(conn, "assessment_session", session_id, "lifecycle", Some(&prior), reason)?;
    get_session(conn, session_id)
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Value> {
    let mut s = query_one(conn, "SELECT * FROM assessment_sessions WHERE id=?1", [id])?
        .ok_or_else(|| Error::not_found("assessment session"))?;
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM assessment_trials WHERE session_id=?1", [id], |r| r.get(0))?;
    s["trial_count"] = json!(n);
    Ok(s)
}

pub fn get_session_with_trials(conn: &Connection, id: &str) -> Result<Value> {
    let mut s = get_session(conn, id)?;
    s["trials"] = json!(query_json(
        conn,
        "SELECT * FROM assessment_trials WHERE session_id=?1 ORDER BY trial_index",
        [id],
    )?);
    Ok(s)
}

pub fn list_sessions(conn: &Connection, kind: Option<&str>, limit: i64) -> Result<Vec<Value>> {
    let mut rows = match kind {
        Some(k) => query_json(
            conn,
            "SELECT * FROM assessment_sessions WHERE kind=?1 ORDER BY created_at DESC LIMIT ?2",
            params![k, limit],
        )?,
        None => query_json(
            conn,
            "SELECT * FROM assessment_sessions ORDER BY created_at DESC LIMIT ?1",
            params![limit],
        )?,
    };
    for s in &mut rows {
        let id = s["id"].as_str().unwrap_or_default().to_string();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM assessment_trials WHERE session_id=?1", [&id], |r| r.get(0))?;
        s["trial_count"] = json!(n);
    }
    Ok(rows)
}

#[derive(Deserialize)]
pub struct SessionPatch {
    pub session_id: String,
    #[serde(default)]
    pub is_familiarization: Option<bool>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub self_reported_interruption: Option<String>,
    #[serde(default)]
    pub validity_state: Option<String>,
    #[serde(default)]
    pub pre_test: Option<Value>,
}

/// Post-hoc user adjustments: familiarization tag, note, self-reported
/// interruption, or manual validity override (auditable).
pub fn update_session(conn: &Connection, patch: SessionPatch) -> Result<Value> {
    let prior = snapshot(conn, "assessment_sessions", &patch.session_id)?
        .ok_or_else(|| Error::not_found("assessment session"))?;
    if let Some(v) = &patch.validity_state {
        if !["valid", "caution", "invalid", "unreviewed"].contains(&v.as_str()) {
            return Err(Error::invalid("invalid validity state"));
        }
    }
    let now = now_rfc3339();
    conn.execute(
        "UPDATE assessment_sessions SET
           is_familiarization = COALESCE(?2, is_familiarization),
           note = COALESCE(?3, note),
           self_reported_interruption = COALESCE(?4, self_reported_interruption),
           validity_state = COALESCE(?5, validity_state),
           pre_test = COALESCE(?6, pre_test),
           updated_at = ?7
         WHERE id=?1",
        params![
            patch.session_id, patch.is_familiarization.map(|b| b as i64), patch.note,
            patch.self_reported_interruption, patch.validity_state,
            patch.pre_test.map(|v| v.to_string()), now
        ],
    )?;
    audit(conn, "assessment_session", &patch.session_id, "update", Some(&prior), None)?;
    get_session(conn, &patch.session_id)
}

// ---------------------------------------------------------------- schedules

pub fn list_schedules(conn: &Connection) -> Result<Vec<Value>> {
    query_json(conn, "SELECT * FROM assessment_schedules ORDER BY kind", [])
}

#[derive(Deserialize)]
pub struct ScheduleInput {
    pub kind: String,
    pub enabled: bool,
    pub weekdays: Vec<i64>,
    pub window_start: String,
    pub window_end: String,
}

pub fn save_schedule(conn: &Connection, input: ScheduleInput) -> Result<Value> {
    crate::util::parse_hhmm(&input.window_start)?;
    crate::util::parse_hhmm(&input.window_end)?;
    conn.execute(
        "UPDATE assessment_schedules SET enabled=?2, weekdays=?3, window_start=?4, window_end=?5, updated_at=?6
         WHERE kind=?1",
        params![
            input.kind, input.enabled as i64, serde_json::to_string(&input.weekdays)?,
            input.window_start, input.window_end, now_rfc3339()
        ],
    )?;
    query_one(conn, "SELECT * FROM assessment_schedules WHERE kind=?1", [&input.kind])?
        .ok_or_else(|| Error::not_found("assessment schedule"))
}

fn is_inside_window_now(conn: &Connection, kind: &str) -> Result<bool> {
    let sched = query_one(conn, "SELECT * FROM assessment_schedules WHERE kind=?1 AND enabled=1", [kind])?;
    match sched {
        None => Ok(true), // no configured window -> no deviation to record
        Some(s) => {
            let tz = settings::timezone(conn)?;
            let now_hhmm = now_local_hhmm(&tz)?;
            let ws = s["window_start"].as_str().unwrap_or("00:00");
            let we = s["window_end"].as_str().unwrap_or("23:59");
            in_window(&now_hhmm, ws, we)
        }
    }
}

/// Assessments due today: enabled schedule matches today's weekday and no
/// completed session of that kind exists today. Late-but-available: a due
/// assessment stays startable outside its window; the deviation is recorded
/// as a validity flag rather than blocking (OPEN_QUESTIONS #5 decision).
pub fn due_today(conn: &Connection) -> Result<Vec<Value>> {
    let today = crate::repo_daily::today(conn)?;
    let wd = weekday_index(&today)? as i64;
    let schedules = list_schedules(conn)?;
    let mut due = Vec::new();
    for s in schedules {
        if s["enabled"].as_i64().unwrap_or(0) == 0 {
            continue;
        }
        let weekdays: Vec<i64> = serde_json::from_value(s["weekdays"].clone()).unwrap_or_default();
        if !weekdays.is_empty() && !weekdays.contains(&wd) {
            continue;
        }
        let kind = s["kind"].as_str().unwrap_or_default().to_string();
        let done: i64 = conn.query_row(
            "SELECT COUNT(*) FROM assessment_sessions WHERE kind=?1 AND local_date=?2 AND status IN ('completed')",
            params![kind, today],
            |r| r.get(0),
        )?;
        if done == 0 {
            let mut item = s.clone();
            item["inside_window_now"] = json!(is_inside_window_now(conn, &kind)?);
            due.push(item);
        }
    }
    Ok(due)
}
