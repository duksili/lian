//! Development utility: seed a LIAN database with a few weeks of plausible
//! data through the real repository APIs (never raw SQL), so runtime smoke
//! tests and screenshots exercise genuine application state.
//!
//!   cargo run -p lian-core --example seed_demo -- /path/to/lian.sqlite3

use lian_core::api::dispatch;
use lian_core::rusqlite;
use serde_json::json;

fn main() {
    let path = std::env::args().nth(1).expect("usage: seed_demo <db-path>");
    let conn = lian_core::db::open(std::path::Path::new(&path)).expect("open db");
    let call = |m: &str, p: serde_json::Value| dispatch(&conn, m, p).unwrap_or_else(|e| panic!("{m}: {e}"));

    let tz = "Europe/Zagreb";
    call("settings.set", json!({
        "timezone": tz, "onboarding_complete": true,
        "quiet_hours_start": "21:30", "quiet_hours_end": "07:30",
    }));
    let today = lian_core::repo_daily::today(&conn).unwrap();
    let day = |off: i64| lian_core::util::add_days(&today, off).unwrap();
    // Baseline started five weeks ago and has just completed.
    call("settings.set", json!({ "baseline_start": day(-38), "baseline_weeks": 5 }));

    let templates = call("templates.list", json!({}));
    let tid = |name: &str| templates.as_array().unwrap().iter()
        .find(|t| t["name"] == name).unwrap()["id"].as_str().unwrap().to_string();
    let (med, taiji, walk) = (tid("Meditation"), tid("Taiji"), tid("Walking"));
    let dims = call("dimensions.list", json!({}));
    let dim = |key: &str| dims.as_array().unwrap().iter()
        .find(|d| d["key"] == key).unwrap()["id"].as_str().unwrap().to_string();
    let (calm, energy, focus, tension) = (dim("calm"), dim("energy"), dim("focus"), dim("body_tension"));

    // ~5.5 weeks of practice, check-ins, precepts. Some days deliberately unlogged.
    for off in -38..=0i64 {
        let date = day(off);
        let n = off.rem_euclid(7);
        if n == 3 && off % 2 == 0 {
            continue; // unknown days stay unknown
        }
        if n != 2 && n != 5 {
            call("events.save", json!({
                "template_id": med, "occurred_at": format!("{date}T06:50:00+02:00"),
                "duration_seconds": 1500 + (off.rem_euclid(4)) * 300, "subtype": "seated",
                "perceived_quality": 3 + (off.rem_euclid(3)),
            }));
        }
        if n == 0 || n == 2 || n == 4 {
            call("events.save", json!({
                "template_id": taiji, "occurred_at": format!("{date}T07:30:00+02:00"),
                "duration_seconds": 1800 + (off.rem_euclid(3)) * 600,
                "subtype": if n == 0 { "form" } else { "standing" },
                "intensity": 2 + (off.rem_euclid(2)), "perceived_quality": 3 + (off.rem_euclid(3)),
            }));
        }
        if n == 5 {
            call("events.save", json!({
                "template_id": walk, "occurred_at": format!("{date}T17:30:00+02:00"),
                "duration_seconds": 3600, "subtype": "hike",
            }));
        }
        if n != 6 {
            let taiji_day = n == 0 || n == 2 || n == 4;
            call("checkins.save", json!({
                "local_date": date,
                "ratings": {
                    calm.clone(): if taiji_day { 4 } else { 3 },
                    energy.clone(): 3 + (off.rem_euclid(2)),
                    focus.clone(): if taiji_day { 4 } else { 3 },
                    tension.clone(): if taiji_day { 2 } else { 3 },
                },
                "sleep_duration_minutes": 400 + (off.rem_euclid(5)) * 15,
                "sleep_quality": 3 + (off.rem_euclid(2)),
            }));
        }
        if n == 0 || n == 1 || n == 4 {
            call("precepts.save", json!({
                "local_date": date,
                "entries": [
                    { "precept_key": "non_harming_life", "status": "observed" },
                    { "precept_key": "not_taking_unoffered", "status": "observed" },
                    { "precept_key": "responsible_sexual_conduct", "status": "observed" },
                    { "precept_key": "truthful_harmless_speech",
                      "status": if n == 1 { "uncertain" } else { "observed" } },
                    { "precept_key": "clarity_regarding_intoxicants", "status": "observed" },
                ],
            }));
        }
    }

    // Context, determination, plans.
    call("context.save", json!({
        "kind": "illness", "label": "Head cold", "start_date": day(-16), "end_date": day(-13),
    }));
    call("context.save", json!({
        "kind": "workload", "label": "Release week", "start_date": day(-6), "end_date": day(-2),
    }));
    let det = call("determinations.save", json!({
        "title": "Stand ten minutes before breakfast, every day this season",
        "description": "Zhan zhuang before anything else; short is fine.",
        "started_on": day(-30), "review_cadence": "weekly",
    }));
    call("determinations.review", json!({
        "determination_id": det["id"], "local_date": day(-7), "status": "kept",
    }));
    call("series.save", json!({
        "title": "Morning form", "kind": "activity", "activity_template_id": taiji,
        "frequency": "weekly", "weekdays": [0, 2, 4], "time_of_day": "07:30",
        "duration_minutes": 40, "starts_on": day(-14), "reminder_offset_minutes": 15,
    }));
    call("plans.save", json!({
        "title": "Evening restorative walk", "kind": "activity", "activity_template_id": walk,
        "local_date": day(1), "time_of_day": "18:00",
    }));

    // Assessment history: PVT 3x/week, GNG 2x/week; first two are familiarization.
    call("assessments.save_schedule", json!({
        "kind": "pvt_v1", "enabled": true, "weekdays": [0, 2, 4],
        "window_start": "07:00", "window_end": "11:00",
    }));
    let mut pvt_count = 0;
    for off in -35..=0i64 {
        let n = off.rem_euclid(7);
        if !(n == 0 || n == 2 || n == 4) {
            continue;
        }
        let started = call("assessments.start", json!({
            "kind": "pvt_v1", "input_method": "keyboard_spacebar",
            "is_familiarization": pvt_count < 2,
        }));
        let sid = started["session"]["id"].as_str().unwrap().to_string();
        let intervals = started["sequence"]["intervals_ms"].as_array().unwrap().clone();
        let mut trials = vec![];
        let mut clock = 0i64;
        let base_rt = 320 - (pvt_count as i64 * 2).min(40) + (off.rem_euclid(3)) * 15;
        let mut i = 0usize;
        loop {
            let isi = intervals[i].as_i64().unwrap();
            if clock + isi + 3000 > 300_000 { break; }
            clock += isi;
            let rt = base_rt + ((i as i64 * 37) % 90);
            trials.push(json!({
                "trial_index": i, "stimulus_kind": "stimulus", "planned_interval_ms": isi,
                "onset_ms": clock, "response_ms": clock + rt, "reaction_time_ms": rt,
            }));
            clock += rt + 550;
            i += 1;
        }
        call("assessments.finalize", json!({
            "session_id": sid, "trials": trials,
            "context": { "elapsed_ms": clock, "visibility_lost_count": if off == -9 { 1 } else { 0 } },
        }));
        // Backdate for a believable history.
        conn.execute(
            "UPDATE assessment_sessions SET local_date=?2, started_at=?3, created_at=?3 WHERE id=?1",
            rusqlite::params![sid, day(off), format!("{}T08:05:00+02:00", day(off))],
        ).unwrap();
        pvt_count += 1;
    }

    println!("seeded demo data into {path} (today = {today})");
}
