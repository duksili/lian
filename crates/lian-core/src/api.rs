//! Typed API dispatch. The desktop shell exposes exactly one command that
//! forwards `(method, payload)` here — no SQL or ad-hoc queries ever cross
//! the boundary, and the whole surface is testable without Tauri.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::{
    analysis, backup, jsonq, reminders, repo_assess, repo_daily, repo_determinations,
    repo_plans, repo_research, review, settings, Error, Result,
};

fn s<'a>(p: &'a Value, key: &str) -> Result<&'a str> {
    p[key]
        .as_str()
        .ok_or_else(|| Error::invalid(format!("missing field '{key}'")))
}

fn b(p: &Value, key: &str, default: bool) -> bool {
    p[key].as_bool().unwrap_or(default)
}

fn parse<T: serde::de::DeserializeOwned>(p: Value) -> Result<T> {
    serde_json::from_value(p).map_err(|e| Error::invalid(format!("invalid payload: {e}")))
}

pub fn dispatch(conn: &Connection, method: &str, p: Value) -> Result<Value> {
    match method {
        // -------- settings / meta
        "settings.get" => settings::get_all(conn),
        "settings.set" => settings::set_many(conn, &p),
        "meta.status" => Ok(json!({
            "app_version": crate::APP_VERSION,
            "schema_version": crate::db::schema_version(conn)?,
            "today": repo_daily::today(conn)?,
        })),

        // -------- templates
        "templates.list" => Ok(json!(repo_daily::list_templates(conn, b(&p, "include_archived", false))?)),
        "templates.save" => repo_daily::save_template(conn, parse(p)?),
        "templates.set_archived" => {
            repo_daily::set_template_archived(conn, s(&p, "id")?, b(&p, "archived", true))?;
            Ok(json!({ "ok": true }))
        }
        "templates.reorder" => {
            repo_daily::reorder_templates(conn, parse(p["ordered_ids"].clone())?)?;
            Ok(json!({ "ok": true }))
        }

        // -------- activity events
        "events.save" => repo_daily::save_event(conn, parse(p)?),
        "events.get" => repo_daily::get_event(conn, s(&p, "id")?),
        "events.list" => Ok(json!(repo_daily::list_events(conn, s(&p, "from")?, s(&p, "to")?)?)),
        "events.set_status" => {
            repo_daily::set_event_status(conn, s(&p, "id")?, s(&p, "status")?)?;
            Ok(json!({ "ok": true }))
        }
        "events.delete" => {
            repo_daily::delete_event(conn, s(&p, "id")?, b(&p, "hard", false), p["reason"].as_str())?;
            Ok(json!({ "ok": true }))
        }
        "events.audit" => Ok(json!(repo_daily::audit_for(conn, "activity_event", s(&p, "id")?)?)),

        // -------- check-ins
        "checkins.save" => repo_daily::save_checkin(conn, parse(p)?),
        "checkins.list" => Ok(json!(repo_daily::list_checkins(conn, s(&p, "from")?, s(&p, "to")?)?)),
        "checkins.delete" => {
            repo_daily::delete_checkin(conn, s(&p, "id")?, b(&p, "hard", false))?;
            Ok(json!({ "ok": true }))
        }
        "dimensions.list" => Ok(json!(repo_daily::list_dimensions(conn)?)),
        "dimensions.configure" => {
            repo_daily::configure_dimensions(conn, parse(p["enabled_ids"].clone())?)?;
            Ok(json!({ "ok": true }))
        }

        // -------- Five Precepts
        "precepts.save" => repo_daily::save_precepts(conn, parse(p)?),
        "precepts.get" => repo_daily::get_precepts(conn, s(&p, "local_date")?),
        "precepts.list" => Ok(json!(repo_daily::list_precepts(conn, s(&p, "from")?, s(&p, "to")?)?)),

        // -------- context events
        "context.save" => repo_daily::save_context_event(conn, parse(p)?),
        "context.list" => Ok(json!(repo_daily::list_context_events(conn, s(&p, "from")?, s(&p, "to")?)?)),
        "context.delete" => {
            repo_daily::delete_context_event(conn, s(&p, "id")?)?;
            Ok(json!({ "ok": true }))
        }

        // -------- determinations
        "determinations.save" => repo_determinations::save_determination(conn, parse(p)?),
        "determinations.get" => repo_determinations::get_determination(conn, s(&p, "id")?),
        "determinations.list" => Ok(json!(repo_determinations::list_determinations(conn, b(&p, "include_closed", false))?)),
        "determinations.set_lifecycle" => repo_determinations::set_lifecycle(conn, s(&p, "id")?, s(&p, "state")?),
        "determinations.supersede" => {
            let id = s(&p, "id")?.to_string();
            repo_determinations::supersede(conn, &id, parse(p["replacement"].clone())?)
        }
        "determinations.review" => repo_determinations::save_review(conn, parse(p)?),
        "determinations.add_link" => {
            repo_determinations::add_link(conn, parse(p)?)?;
            Ok(json!({ "ok": true }))
        }
        "determinations.remove_link" => {
            repo_determinations::remove_link(conn, s(&p, "link_id")?)?;
            Ok(json!({ "ok": true }))
        }

        // -------- plans
        "plans.save" => repo_plans::save_plan(conn, parse(p)?),
        "plans.get" => repo_plans::get_plan(conn, s(&p, "id")?),
        "plans.list" => Ok(json!(repo_plans::list_plans(conn, s(&p, "from")?, s(&p, "to")?)?)),
        "plans.set_status" => repo_plans::set_plan_status(conn, s(&p, "id")?, s(&p, "status")?),
        "plans.delete" => {
            repo_plans::delete_plan(conn, s(&p, "id")?)?;
            Ok(json!({ "ok": true }))
        }
        "plans.link_event" => {
            repo_plans::link_event_to_plan(conn, s(&p, "plan_id")?, s(&p, "event_id")?)?;
            Ok(json!({ "ok": true }))
        }
        "plans.unlink_event" => {
            repo_plans::unlink_plan(conn, s(&p, "plan_id")?, s(&p, "event_id")?)?;
            Ok(json!({ "ok": true }))
        }
        "plans.suggest_for_event" => Ok(json!(repo_plans::suggest_plans_for_event(conn, s(&p, "event_id")?)?)),
        "series.save" => repo_plans::save_series(conn, parse(p)?),
        "series.list" => Ok(json!(repo_plans::list_series(conn)?)),
        "series.end" => {
            repo_plans::end_series(conn, s(&p, "id")?)?;
            Ok(json!({ "ok": true }))
        }

        // -------- reminders
        "reminders.rules" => Ok(json!(reminders::list_rules(conn)?)),
        "reminders.save_rule" => reminders::save_rule(conn, parse(p)?),
        "reminders.set_enabled" => {
            reminders::set_rule_enabled(conn, s(&p, "id")?, b(&p, "enabled", true))?;
            Ok(json!({ "ok": true }))
        }
        "reminders.snooze" => {
            reminders::snooze_rule(conn, s(&p, "id")?, p["minutes"].as_i64().unwrap_or(30))?;
            Ok(json!({ "ok": true }))
        }
        "reminders.delete_rule" => {
            reminders::delete_rule(conn, s(&p, "id")?)?;
            Ok(json!({ "ok": true }))
        }
        "reminders.set_pause" => {
            reminders::set_pause(conn, b(&p, "paused", false), p["until"].as_str().map(String::from))?;
            Ok(json!({ "ok": true }))
        }

        // -------- assessments
        "assessments.start" => repo_assess::start_session(conn, parse(p)?),
        "assessments.finalize" => repo_assess::finalize_session(conn, parse(p)?),
        "assessments.abort" => repo_assess::abort_session(conn, s(&p, "session_id")?, p["reason"].as_str()),
        "assessments.get" => repo_assess::get_session_with_trials(conn, s(&p, "id")?),
        "assessments.list" => Ok(json!(repo_assess::list_sessions(
            conn, p["kind"].as_str(), p["limit"].as_i64().unwrap_or(100))?)),
        "assessments.update" => repo_assess::update_session(conn, parse(p)?),
        "assessments.schedules" => Ok(json!(repo_assess::list_schedules(conn)?)),
        "assessments.save_schedule" => repo_assess::save_schedule(conn, parse(p)?),
        "assessments.due_today" => Ok(json!(repo_assess::due_today(conn)?)),

        // -------- research
        "protocols.save" => repo_research::save_protocol(conn, parse(p)?),
        "protocols.get" => repo_research::get_protocol(conn, s(&p, "id")?),
        "protocols.list" => Ok(json!(repo_research::list_protocols(conn)?)),
        "protocols.set_status" => repo_research::set_protocol_status(conn, s(&p, "id")?, s(&p, "status")?),
        "protocols.conclude" => repo_research::conclude_protocol(conn, s(&p, "id")?, s(&p, "conclusion")?, p["note"].as_str()),
        "protocols.lock_results" => {
            repo_research::lock_protocol_results(conn, s(&p, "id")?)?;
            Ok(json!({ "ok": true }))
        }
        "analysis.run" => analysis::run_association(conn, parse(p)?),
        "analysis.results" => Ok(json!(analysis::list_results(conn, p["limit"].as_i64().unwrap_or(50))?)),
        "analysis.promote" => analysis::promote_to_candidate(conn, s(&p, "result_id")?, p["note"].as_str()),

        // -------- reviews
        "view.today" => review::today_view(conn),
        "view.weekly" => review::weekly_review(conn, s(&p, "date")?),
        "view.monthly" => review::monthly_review(conn, s(&p, "from")?, s(&p, "to")?),
        "review.save_reflection" => review::save_weekly_reflection(conn, s(&p, "week_start")?, s(&p, "note")?),

        // -------- backup / export
        "backup.create" => backup::create_backup(conn, s(&p, "dest_dir")?),
        "backup.list" => Ok(json!(backup::list_backups(conn)?)),
        "backup.verify" => backup::verify_backup(s(&p, "path")?),
        "export.csv" => backup::export_csv(conn, s(&p, "dest_dir")?),

        // -------- audit
        "audit.list" => Ok(json!(repo_daily::audit_for(conn, s(&p, "entity_type")?, s(&p, "entity_id")?)?)),
        "audit.recent" => Ok(json!(jsonq::query_json(
            conn,
            "SELECT * FROM audit_log ORDER BY changed_at DESC LIMIT 100",
            [],
        )?)),

        other => Err(Error::invalid(format!("unknown method '{other}'"))),
    }
}
