//! End-to-end tests over the API dispatch surface — the same calls the
//! desktop shell forwards, exercised against a real SQLite database.

use lian_core::{api::dispatch, db};
use serde_json::{json, Value};

fn setup() -> rusqlite::Connection {
    let conn = db::open_in_memory().unwrap();
    dispatch(&conn, "settings.set", json!({ "timezone": "Europe/Zagreb" })).unwrap();
    conn
}

fn call(conn: &rusqlite::Connection, method: &str, p: Value) -> Value {
    dispatch(conn, method, p).unwrap_or_else(|e| panic!("{method} failed: {e}"))
}

fn first_template_id(conn: &rusqlite::Connection, name: &str) -> String {
    let templates = call(conn, "templates.list", json!({}));
    templates
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn quick_log_edit_and_delete_flow() {
    let conn = setup();
    let med = first_template_id(&conn, "Meditation");

    // Fast log: template + duration, time defaults handled by caller.
    let ev = call(&conn, "events.save", json!({
        "template_id": med,
        "occurred_at": "2026-07-01T07:30:00+02:00",
        "duration_seconds": 1800,
        "source": "manual",
    }));
    assert_eq!(ev["local_date"], "2026-07-01");
    assert_eq!(ev["time_known"], 1);
    assert_eq!(ev["status"], "completed");
    let id = ev["id"].as_str().unwrap().to_string();

    // Enrich later without recreating.
    let ev2 = call(&conn, "events.save", json!({
        "id": id, "template_id": med,
        "occurred_at": "2026-07-01T07:30:00+02:00",
        "duration_seconds": 2100,
        "subtype": "seated", "perceived_quality": 4, "note": "settled quickly",
    }));
    assert_eq!(ev2["duration_seconds"], 2100);
    assert_eq!(ev2["note"], "settled quickly");

    // Edit is audited with prior values.
    let audit = call(&conn, "events.audit", json!({ "id": id }));
    assert_eq!(audit.as_array().unwrap().len(), 1);
    assert_eq!(audit[0]["prior_values"]["duration_seconds"], 1800);

    // Date-only backfill: unknown time stays explicit.
    let back = call(&conn, "events.save", json!({
        "template_id": med, "local_date": "2026-06-28",
    }));
    assert_eq!(back["time_known"], 0);
    assert!(back["occurred_at"].is_null());
    assert!(back["duration_seconds"].is_null(), "omitted duration must stay unknown");

    // Delete (soft) then verify it leaves list; audit retained.
    call(&conn, "events.delete", json!({ "id": id }));
    let listed = call(&conn, "events.list", json!({ "from": "2026-07-01", "to": "2026-07-01" }));
    assert!(listed.as_array().unwrap().is_empty());
}

#[test]
fn checkin_precepts_and_missing_data() {
    let conn = setup();
    let dims = call(&conn, "dimensions.list", json!({}));
    let calm = dims.as_array().unwrap().iter().find(|d| d["key"] == "calm").unwrap()["id"]
        .as_str().unwrap().to_string();

    let ci = call(&conn, "checkins.save", json!({
        "local_date": "2026-07-01",
        "ratings": { calm.clone(): 4 },
        "sleep_duration_minutes": 440,
        "sleep_quality": 3,
    }));
    assert_eq!(ci["ratings"][0]["value"], 4);
    // Omitted dimensions are simply absent — unknown, not zero.
    assert_eq!(ci["ratings"].as_array().unwrap().len(), 1);

    // Precepts: partial review allowed; absent record is Null, not a status.
    let none = call(&conn, "precepts.get", json!({ "local_date": "2026-07-01" }));
    assert!(none.is_null());
    let rec = call(&conn, "precepts.save", json!({
        "local_date": "2026-07-01",
        "entries": [
            { "precept_key": "non_harming_life", "status": "observed" },
            { "precept_key": "truthful_harmless_speech", "status": "uncertain", "note": "one sharp remark" },
        ],
    }));
    assert_eq!(rec["entries"].as_array().unwrap().len(), 2);
    // Invalid status refused.
    assert!(dispatch(&conn, "precepts.save", json!({
        "local_date": "2026-07-01",
        "entries": [{ "precept_key": "non_harming_life", "status": "failed" }],
    })).is_err());
}

#[test]
fn determination_lifecycle_preserves_history() {
    let conn = setup();
    let d = call(&conn, "determinations.save", json!({
        "title": "Sit every morning before breakfast",
        "started_on": "2026-07-01",
        "review_cadence": "weekly",
    }));
    let id = d["id"].as_str().unwrap().to_string();

    // Revision keeps prior wording.
    let d2 = call(&conn, "determinations.save", json!({
        "id": id, "title": "Sit each morning, even briefly",
        "started_on": "2026-07-01", "review_cadence": "weekly",
    }));
    assert_eq!(d2["revisions"][0]["prior_title"], "Sit every morning before breakfast");

    // Review requires a cadence and uses only the allowed statuses.
    let r = call(&conn, "determinations.review", json!({
        "determination_id": id, "local_date": "2026-07-07", "status": "kept",
    }));
    assert_eq!(r["reviews"][0]["status"], "kept");
    assert!(dispatch(&conn, "determinations.review", json!({
        "determination_id": id, "local_date": "2026-07-08", "status": "failed",
    })).is_err());

    // Pause and resume.
    let paused = call(&conn, "determinations.set_lifecycle", json!({ "id": id, "state": "paused" }));
    assert_eq!(paused["lifecycle_state"], "paused");

    // Supersede: old wording retained, chain linked both ways.
    let new = call(&conn, "determinations.supersede", json!({
        "id": id,
        "replacement": { "title": "Sit twice daily", "started_on": "2026-08-01" },
    }));
    assert_eq!(new["predecessor_id"], id.as_str());
    let old = call(&conn, "determinations.get", json!({ "id": id }));
    assert_eq!(old["lifecycle_state"], "superseded");
    assert_eq!(old["superseded_by_id"], new["id"]);

    // A determination without a cadence refuses review entries.
    let bare = call(&conn, "determinations.save", json!({
        "title": "No news before noon", "started_on": "2026-07-01",
    }));
    assert!(dispatch(&conn, "determinations.review", json!({
        "determination_id": bare["id"], "local_date": "2026-07-02", "status": "kept",
    })).is_err());
}

#[test]
fn plans_recurrence_and_linking() {
    let conn = setup();
    let taiji = first_template_id(&conn, "Taiji");

    // Weekly series Mon/Wed/Fri.
    let series = call(&conn, "series.save", json!({
        "title": "Morning form", "kind": "activity", "activity_template_id": taiji,
        "frequency": "weekly", "weekdays": [0, 2, 4], "time_of_day": "07:00",
        "duration_minutes": 30, "starts_on": "2026-06-01",
    }));
    let sid = series["id"].as_str().unwrap().to_string();

    // June 2026: materialized occurrences appear on listing.
    let plans = call(&conn, "plans.list", json!({ "from": "2026-06-01", "to": "2026-06-14" }));
    let occurrences: Vec<&Value> = plans.as_array().unwrap().iter()
        .filter(|p| p["series_id"] == sid.as_str()).collect();
    assert_eq!(occurrences.len(), 6, "two weeks of Mon/Wed/Fri");
    // Listing again does not duplicate.
    let again = call(&conn, "plans.list", json!({ "from": "2026-06-01", "to": "2026-06-14" }));
    assert_eq!(again.as_array().unwrap().len(), plans.as_array().unwrap().len());

    // Link a completed event to one occurrence explicitly.
    let plan_id = occurrences[0]["id"].as_str().unwrap().to_string();
    let plan_date = occurrences[0]["local_date"].as_str().unwrap().to_string();
    let ev = call(&conn, "events.save", json!({
        "template_id": taiji,
        "occurred_at": format!("{plan_date}T07:05:00+02:00"),
        "duration_seconds": 1740,
    }));
    call(&conn, "plans.link_event", json!({ "plan_id": plan_id, "event_id": ev["id"] }));
    let linked = call(&conn, "plans.get", json!({ "id": plan_id }));
    assert_eq!(linked["status"], "completed_linked");
    assert_eq!(linked["links"].as_array().unwrap().len(), 1);

    // Editing the series regenerates the future but never the linked/past rows.
    call(&conn, "series.save", json!({
        "id": sid, "title": "Morning form (longer)", "kind": "activity",
        "activity_template_id": taiji, "frequency": "weekly", "weekdays": [0],
        "time_of_day": "07:00", "duration_minutes": 45, "starts_on": "2026-06-01",
    }));
    let after = call(&conn, "plans.get", json!({ "id": plan_id }));
    assert_eq!(after["title"], "Morning form", "historical occurrence unchanged");
    assert_eq!(after["status"], "completed_linked");

    // Skipping a plan is neutral information.
    let skipped = call(&conn, "plans.set_status", json!({
        "id": occurrences[1]["id"], "status": "skipped",
    }));
    assert_eq!(skipped["status"], "skipped");
}

#[test]
fn pvt_session_full_cycle() {
    let conn = setup();
    let started = call(&conn, "assessments.start", json!({
        "kind": "pvt_v1", "input_method": "keyboard_spacebar",
        "pre_test": { "caffeine": false, "seated": true },
    }));
    let session_id = started["session"]["id"].as_str().unwrap().to_string();
    let intervals = started["sequence"]["intervals_ms"].as_array().unwrap();
    assert!(intervals.len() >= 25);
    assert_eq!(started["session"]["protocol_version"], "pvt-1.0");

    // Simulate raw trials: mostly normal, one lapse, one false start, one omission.
    let mut trials = Vec::new();
    let mut clock = 0i64;
    for (i, isi) in intervals.iter().enumerate() {
        clock += isi.as_i64().unwrap();
        let (rt, resp): (Option<i64>, Option<i64>) = match i {
            2 => (Some(620), Some(clock + 620)), // lapse
            5 => (Some(40), Some(clock + 40)),   // false start
            8 => (None, None),                    // omission
            _ => (Some(280 + (i as i64 % 60)), Some(clock + 280 + (i as i64 % 60))),
        };
        trials.push(json!({
            "trial_index": i, "stimulus_kind": "stimulus",
            "planned_interval_ms": isi, "onset_ms": clock,
            "response_ms": resp, "reaction_time_ms": rt,
        }));
        clock += rt.unwrap_or(3000);
    }
    let done = call(&conn, "assessments.finalize", json!({
        "session_id": session_id,
        "trials": trials,
        "context": { "elapsed_ms": 300000, "visibility_lost_count": 0 },
    }));
    assert_eq!(done["status"], "completed");
    assert_eq!(done["validity_state"], "valid");
    assert_eq!(done["derived_metrics"]["lapse_count"], 1);
    assert_eq!(done["derived_metrics"]["false_start_count"], 1);
    assert_eq!(done["derived_metrics"]["omission_count"], 1);
    assert_eq!(done["trial_count"].as_i64().unwrap(), intervals.len() as i64);

    // Raw trials retrievable.
    let full = call(&conn, "assessments.get", json!({ "id": session_id }));
    assert_eq!(full["trials"].as_array().unwrap().len(), intervals.len());
    assert_eq!(full["trials"][2]["is_lapse"], 1);
    assert_eq!(full["trials"][5]["is_false_start"], 1);

    // Familiarization tagging after the fact.
    let tagged = call(&conn, "assessments.update", json!({
        "session_id": session_id, "is_familiarization": true,
    }));
    assert_eq!(tagged["is_familiarization"], 1);
}

#[test]
fn gng_session_and_interruption_flags() {
    let conn = setup();
    let started = call(&conn, "assessments.start", json!({ "kind": "go_no_go_v1" }));
    let session_id = started["session"]["id"].as_str().unwrap().to_string();
    let seq = started["sequence"]["trials"].as_array().unwrap();
    assert_eq!(seq.len(), 160);
    let nogo = seq.iter().filter(|t| t["stimulus"] == "no_go").count();
    assert_eq!(nogo, 40);

    let mut trials = Vec::new();
    for (i, t) in seq.iter().enumerate() {
        let is_go = t["stimulus"] == "go";
        let responded = if is_go { i % 30 != 0 } else { i % 10 == 0 }; // some omissions/commissions
        trials.push(json!({
            "trial_index": i, "stimulus_kind": t["stimulus"],
            "onset_ms": (i as i64) * 1500,
            "response_ms": if responded { Some((i as i64) * 1500 + 350) } else { None },
            "reaction_time_ms": if responded && is_go { Some(350) } else { None },
        }));
    }
    let done = call(&conn, "assessments.finalize", json!({
        "session_id": session_id, "trials": trials,
        "context": { "visibility_lost_count": 1 },
    }));
    assert_eq!(done["status"], "completed");
    assert_eq!(done["validity_state"], "caution");
    assert!(done["validity_reasons"].as_array().unwrap()
        .iter().any(|r| r == "window_lost_visibility"));
    assert!(done["derived_metrics"]["commission_error_rate"].as_f64().unwrap() > 0.0);
}

#[test]
fn association_analysis_transparency() {
    let conn = setup();
    let taiji = first_template_id(&conn, "Taiji");
    let dims = call(&conn, "dimensions.list", json!({}));
    let tension = dims.as_array().unwrap().iter().find(|d| d["key"] == "body_tension").unwrap()["id"]
        .as_str().unwrap().to_string();

    // 30 days: taiji on even days; tension lower the day after practice.
    for day in 1..=30 {
        let date = format!("2026-06-{day:02}");
        if day % 2 == 0 {
            call(&conn, "events.save", json!({
                "template_id": taiji,
                "occurred_at": format!("{date}T07:00:00+02:00"),
                "duration_seconds": 2400,
            }));
        }
        let prev_practiced = day > 1 && (day - 1) % 2 == 0;
        call(&conn, "checkins.save", json!({
            "local_date": date,
            "ratings": { tension.clone(): if prev_practiced { 2 } else { 4 } },
        }));
    }

    let result = call(&conn, "analysis.run", json!({
        "exposure": { "kind": "activity_duration", "template_id": taiji, "label": "Taiji minutes" },
        "outcome": { "kind": "checkin_dimension", "dimension_id": tension, "label": "Body tension" },
        "lag_days": 1,
        "from": "2026-06-01", "to": "2026-06-29",
        "persist": true,
    }));
    // Transparency requirements.
    assert!(result["included_count"].as_i64().unwrap() > 0);
    assert!(result["values_json"]["points"].as_array().unwrap().len() > 0);
    assert!(result["caveats"].as_array().unwrap().len() >= 2);
    assert_eq!(result["analysis_version"], "analysis-1.0");
    let label = result["evidence_label"].as_str().unwrap();
    assert!(["descriptive", "insufficient_data", "observational_signal"].contains(&label));
    // Practice days missing = unknown: exposure has ~14 known days only where logged.
    assert!(result["missing_count"].as_i64().unwrap() > 0, "unlogged days must count as missing");

    // Promotion requires an observational signal; here it is deterministic
    // enough to be a signal (alternating pattern, strong rank correlation).
    if label == "observational_signal" {
        let promoted = call(&conn, "analysis.promote", json!({
            "result_id": result["id"], "note": "seen two months in a row",
        }));
        assert_eq!(promoted["evidence_label"], "candidate_hypothesis");
    }

    let persisted = call(&conn, "analysis.results", json!({}));
    assert!(!persisted.as_array().unwrap().is_empty());
}

#[test]
fn protocol_version_discipline() {
    let conn = setup();
    let p = call(&conn, "protocols.save", json!({
        "title": "Taiji and next-morning alertness",
        "question": "Does 30+ min Taiji predict fewer PVT lapses next morning?",
        "hypothesis": "Days with >=30 min Taiji are followed by lower lapse rate",
        "primary_outcome_definition": { "kind": "assessment_metric", "assessment_kind": "pvt_v1", "metric": "lapse_rate", "version": "analysis-1.0" },
        "intervention_definition": "Taiji practice 30+ minutes, mornings",
        "analysis_plan": "Compare lapse rate on days following practice vs not, lag 1",
        "start_date": "2026-08-01", "end_date": "2026-09-15",
    }));
    let id = p["id"].as_str().unwrap().to_string();
    assert_eq!(p["version"], 1);
    assert_eq!(p["status"], "draft");

    call(&conn, "protocols.set_status", json!({ "id": id, "status": "active" }));
    call(&conn, "protocols.lock_results", json!({ "id": id }));

    // Changing the outcome after results were viewed forks version 2.
    let amended = call(&conn, "protocols.save", json!({
        "id": id,
        "title": "Taiji and next-morning alertness",
        "question": "Does 30+ min Taiji predict fewer PVT lapses next morning?",
        "hypothesis": "Days with >=45 min Taiji are followed by lower lapse rate",
        "primary_outcome_definition": { "kind": "assessment_metric", "assessment_kind": "pvt_v1", "metric": "lapse_rate", "version": "analysis-1.0" },
        "intervention_definition": "Taiji practice 45+ minutes, mornings",
        "analysis_plan": "Compare lapse rate on days following practice vs not, lag 1",
    }));
    assert_eq!(amended["version"], 2);
    assert_eq!(amended["predecessor_id"], id.as_str());
    let old = call(&conn, "protocols.get", json!({ "id": id }));
    assert_eq!(old["status"], "superseded");

    // Conclusions accept only approved labels; null results are storable.
    assert!(dispatch(&conn, "protocols.conclude", json!({
        "id": amended["id"], "conclusion": "it works",
    })).is_err());
    let concluded = call(&conn, "protocols.conclude", json!({
        "id": amended["id"], "conclusion": "protocol_result_not_supported",
        "note": "no consistent difference",
    }));
    assert_eq!(concluded["conclusion"], "protocol_result_not_supported");
}

#[test]
fn reminders_respect_safeguards() {
    let conn = setup();
    // Enable evening check-in at the current local minute so it is due now.
    let tz: String = "Europe/Zagreb".into();
    let now_hhmm = lian_core::util::now_local_hhmm(&tz).unwrap();
    let rules = call(&conn, "reminders.rules", json!({}));
    let rule = rules.as_array().unwrap().iter().find(|r| r["kind"] == "evening_checkin").unwrap();
    call(&conn, "reminders.save_rule", json!({
        "id": rule["id"], "kind": "evening_checkin", "label": "Evening check-in",
        "time_of_day": now_hhmm, "enabled": true,
    }));
    // Disable quiet hours around now for the test.
    call(&conn, "settings.set", json!({ "quiet_hours_start": "03:00", "quiet_hours_end": "03:01" }));

    let due = lian_core::reminders::due_notifications(&conn).unwrap();
    assert_eq!(due.len(), 1, "reminder should be due");
    lian_core::reminders::record_fired(&conn, &due[0]).unwrap();
    let again = lian_core::reminders::due_notifications(&conn).unwrap();
    assert!(again.is_empty(), "same reminder must not fire twice");

    // Global pause silences everything.
    call(&conn, "reminders.set_pause", json!({ "paused": true }));
    conn.execute("DELETE FROM notification_log", []).unwrap();
    assert!(lian_core::reminders::due_notifications(&conn).unwrap().is_empty());
    call(&conn, "reminders.set_pause", json!({ "paused": false }));

    // Quiet hours silence.
    call(&conn, "settings.set", json!({ "quiet_hours_start": "00:00", "quiet_hours_end": "23:59" }));
    assert!(lian_core::reminders::due_notifications(&conn).unwrap().is_empty());
}

#[test]
fn backup_export_restore_cycle() {
    let conn = setup();
    let med = first_template_id(&conn, "Meditation");
    call(&conn, "events.save", json!({
        "template_id": med, "occurred_at": "2026-07-01T07:30:00+02:00", "duration_seconds": 900,
    }));

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().to_string_lossy().to_string();

    let backup = call(&conn, "backup.create", json!({ "dest_dir": dest }));
    assert_eq!(backup["ok"], true);
    let backup_path = backup["path"].as_str().unwrap().to_string();
    assert!(std::path::Path::new(&backup_path).exists());
    assert!(backup["manifest"]["checksum_sha256"].as_str().unwrap().len() == 64);

    let verify = call(&conn, "backup.verify", json!({ "path": backup_path }));
    assert_eq!(verify["ok"], true);
    assert_eq!(verify["manifest_found_and_matches"], true);

    // The backup itself opens and contains the event.
    let restored = rusqlite::Connection::open(&backup_path).unwrap();
    let n: i64 = restored.query_row("SELECT COUNT(*) FROM activity_events", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);

    // CSV export with manifest and raw trials table.
    let export = call(&conn, "export.csv", json!({ "dest_dir": dest }));
    assert_eq!(export["ok"], true);
    let export_dir = export["path"].as_str().unwrap();
    for f in ["activity_events.csv", "assessment_trials.csv", "precept_records.csv",
              "export-manifest.json", "lian-data.sqlite3"] {
        assert!(std::path::Path::new(export_dir).join(f).exists(), "missing export file {f}");
    }
    let listed = call(&conn, "backup.list", json!({}));
    assert!(!listed.as_array().unwrap().is_empty());
}

#[test]
fn weekly_and_today_views() {
    let conn = setup();
    let med = first_template_id(&conn, "Meditation");
    let today = lian_core::repo_daily::today(&conn).unwrap();
    call(&conn, "events.save", json!({
        "template_id": med, "local_date": today, "duration_seconds": 1200,
    }));

    let view = call(&conn, "view.today", json!({}));
    assert_eq!(view["today"], today.as_str());
    assert_eq!(view["events_today"].as_array().unwrap().len(), 1);
    assert_eq!(view["yesterday_status"]["has_checkin"], false);

    let weekly = call(&conn, "view.weekly", json!({ "date": today }));
    assert_eq!(weekly["volume"][0]["session_count"], 1);
    // Coverage marks unknown days as unknown, not zero/failure.
    let coverage = weekly["coverage"].as_array().unwrap();
    assert_eq!(coverage.len(), 7);
    let today_cov = coverage.iter().find(|c| c["date"] == today.as_str()).unwrap();
    assert_eq!(today_cov["activity"], "recorded");
    assert_eq!(today_cov["checkin"], "unknown");

    call(&conn, "review.save_reflection", json!({
        "week_start": today, "note": "steady week",
    }));
    let weekly2 = call(&conn, "view.weekly", json!({ "date": today }));
    assert_eq!(weekly2["reflection"]["note"], "steady week");

    let monthly = call(&conn, "view.monthly", json!({
        "from": "2026-06-01", "to": today,
    }));
    assert!(monthly["weeks"].as_array().unwrap().len() >= 1);
}
