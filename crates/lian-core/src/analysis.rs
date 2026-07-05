//! Transparent association analysis.
//!
//! Observational only: output carries counts, inspectable day-level points,
//! exclusions, missingness, caveats, and one of the approved evidence labels.
//! Nothing here produces causal language.

use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::jsonq::query_json;
use crate::util::{add_days, new_id, now_rfc3339, parse_date};
use crate::{settings, Error, Result};

pub const ANALYSIS_VERSION: &str = "analysis-1.0";

#[derive(Deserialize, Clone)]
pub struct ExposureDef {
    /// 'activity_duration' | 'activity_count' | 'sleep_duration' | 'checkin_dimension'
    pub kind: String,
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub dimension_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct OutcomeDef {
    /// 'checkin_dimension' | 'assessment_metric' | 'sleep_duration'
    pub kind: String,
    #[serde(default)]
    pub dimension_id: Option<String>,
    #[serde(default)]
    pub assessment_kind: Option<String>,
    /// key inside derived_metrics, e.g. 'lapse_rate', 'median_rt_ms', 'commission_error_rate'
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Deserialize)]
pub struct AssociationSpec {
    pub exposure: ExposureDef,
    pub outcome: OutcomeDef,
    /// Outcome is measured `lag_days` after the exposure day.
    #[serde(default)]
    pub lag_days: i64,
    pub from: String,
    pub to: String,
    #[serde(default = "default_true")]
    pub exclude_invalid: bool,
    #[serde(default = "default_true")]
    pub exclude_familiarization: bool,
    /// Context kinds that exclude a day (e.g. ["illness", "travel"]).
    #[serde(default)]
    pub exclude_context_kinds: Vec<String>,
    #[serde(default)]
    pub protocol_id: Option<String>,
    #[serde(default)]
    pub persist: bool,
}

fn default_true() -> bool {
    true
}

/// exposure value per local date; None where genuinely unknown.
fn exposure_series(conn: &Connection, def: &ExposureDef, from: &str, to: &str) -> Result<BTreeMap<String, f64>> {
    let mut out = BTreeMap::new();
    match def.kind.as_str() {
        "activity_duration" | "activity_count" => {
            let template = def
                .template_id
                .as_deref()
                .ok_or_else(|| Error::invalid("exposure needs template_id"))?;
            let rows = query_json(
                conn,
                "SELECT local_date, COUNT(*) AS n, SUM(duration_seconds) AS total
                 FROM activity_events
                 WHERE deleted_at IS NULL AND status='completed' AND template_id=?1
                   AND local_date >= ?2 AND local_date <= ?3
                 GROUP BY local_date",
                rusqlite::params![template, from, to],
            )?;
            for r in rows {
                let d = r["local_date"].as_str().unwrap_or_default().to_string();
                let v = if def.kind == "activity_count" {
                    r["n"].as_f64().unwrap_or(0.0)
                } else {
                    r["total"].as_f64().unwrap_or(0.0) / 60.0 // minutes
                };
                out.insert(d, v);
            }
            // Days without a log stay absent: unknown, not zero. A day is a known
            // zero only when the user logged nothing but did log a check-in that
            // day? No — we cannot infer practice absence. Missing stays missing.
        }
        "sleep_duration" => {
            let rows = query_json(
                conn,
                "SELECT local_date, sleep_duration_minutes FROM daily_checkins
                 WHERE deleted_at IS NULL AND sleep_duration_minutes IS NOT NULL
                   AND local_date >= ?1 AND local_date <= ?2
                 ORDER BY logged_at",
                [from, to],
            )?;
            for r in rows {
                if let (Some(d), Some(v)) = (r["local_date"].as_str(), r["sleep_duration_minutes"].as_f64()) {
                    out.insert(d.to_string(), v);
                }
            }
        }
        "checkin_dimension" => {
            let dim = def
                .dimension_id
                .as_deref()
                .ok_or_else(|| Error::invalid("exposure needs dimension_id"))?;
            for (d, v) in dimension_series(conn, dim, from, to)? {
                out.insert(d, v);
            }
        }
        other => return Err(Error::invalid(format!("unknown exposure kind '{other}'"))),
    }
    Ok(out)
}

fn dimension_series(conn: &Connection, dimension_id: &str, from: &str, to: &str) -> Result<Vec<(String, f64)>> {
    let rows = query_json(
        conn,
        "SELECT c.local_date, r.value FROM checkin_ratings r
         JOIN daily_checkins c ON c.id = r.checkin_id
         WHERE c.deleted_at IS NULL AND r.dimension_id = ?1
           AND c.local_date >= ?2 AND c.local_date <= ?3
         ORDER BY c.logged_at",
        rusqlite::params![dimension_id, from, to],
    )?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some((r["local_date"].as_str()?.to_string(), r["value"].as_f64()?))
        })
        .collect())
}

struct OutcomeSeries {
    values: BTreeMap<String, f64>,
    excluded: i64, // sessions excluded by validity/familiarization
}

fn outcome_series(conn: &Connection, def: &OutcomeDef, from: &str, to: &str,
                  exclude_invalid: bool, exclude_familiarization: bool) -> Result<OutcomeSeries> {
    let mut values = BTreeMap::new();
    let mut excluded = 0i64;
    match def.kind.as_str() {
        "checkin_dimension" => {
            let dim = def
                .dimension_id
                .as_deref()
                .ok_or_else(|| Error::invalid("outcome needs dimension_id"))?;
            for (d, v) in dimension_series(conn, dim, from, to)? {
                values.insert(d, v); // later entries win = latest check-in of the day
            }
        }
        "sleep_duration" => {
            let rows = query_json(
                conn,
                "SELECT local_date, sleep_duration_minutes FROM daily_checkins
                 WHERE deleted_at IS NULL AND sleep_duration_minutes IS NOT NULL
                   AND local_date >= ?1 AND local_date <= ?2 ORDER BY logged_at",
                [from, to],
            )?;
            for r in rows {
                if let (Some(d), Some(v)) = (r["local_date"].as_str(), r["sleep_duration_minutes"].as_f64()) {
                    values.insert(d.to_string(), v);
                }
            }
        }
        "assessment_metric" => {
            let kind = def
                .assessment_kind
                .as_deref()
                .ok_or_else(|| Error::invalid("outcome needs assessment_kind"))?;
            let metric = def
                .metric
                .as_deref()
                .ok_or_else(|| Error::invalid("outcome needs metric key"))?;
            let rows = query_json(
                conn,
                "SELECT local_date, derived_metrics, validity_state, is_familiarization
                 FROM assessment_sessions
                 WHERE kind=?1 AND status='completed' AND local_date >= ?2 AND local_date <= ?3
                 ORDER BY started_at",
                rusqlite::params![kind, from, to],
            )?;
            for r in rows {
                let invalid = r["validity_state"].as_str() == Some("invalid");
                let familiar = r["is_familiarization"].as_i64().unwrap_or(0) == 1;
                if (exclude_invalid && invalid) || (exclude_familiarization && familiar) {
                    excluded += 1;
                    continue;
                }
                if let (Some(d), Some(v)) = (r["local_date"].as_str(), r["derived_metrics"][metric].as_f64()) {
                    values.insert(d.to_string(), v);
                }
            }
        }
        other => return Err(Error::invalid(format!("unknown outcome kind '{other}'"))),
    }
    Ok(OutcomeSeries { values, excluded })
}

fn spearman(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 3 {
        return None;
    }
    fn ranks(xs: &[f64]) -> Vec<f64> {
        let mut idx: Vec<usize> = (0..xs.len()).collect();
        idx.sort_by(|&a, &b| xs[a].partial_cmp(&xs[b]).unwrap_or(std::cmp::Ordering::Equal));
        let mut r = vec![0.0; xs.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && xs[idx[j + 1]] == xs[idx[i]] {
                j += 1;
            }
            let avg = (i + j) as f64 / 2.0 + 1.0;
            for k in i..=j {
                r[idx[k]] = avg;
            }
            i = j + 1;
        }
        r
    }
    let xs: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = pairs.iter().map(|p| p.1).collect();
    let rx = ranks(&xs);
    let ry = ranks(&ys);
    let n = pairs.len() as f64;
    let mx = rx.iter().sum::<f64>() / n;
    let my = ry.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for i in 0..pairs.len() {
        num += (rx[i] - mx) * (ry[i] - my);
        dx += (rx[i] - mx).powi(2);
        dy += (ry[i] - my).powi(2);
    }
    if dx == 0.0 || dy == 0.0 {
        return None;
    }
    Some(num / (dx * dy).sqrt())
}

fn summary(xs: &[f64]) -> Value {
    if xs.is_empty() {
        return json!(null);
    }
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    let mean = s.iter().sum::<f64>() / n as f64;
    let median = if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 };
    json!({ "n": n, "mean": mean, "median": median, "min": s[0], "max": s[n - 1] })
}

pub fn run_association(conn: &Connection, spec: AssociationSpec) -> Result<Value> {
    parse_date(&spec.from)?;
    parse_date(&spec.to)?;
    if spec.lag_days < 0 || spec.lag_days > 14 {
        return Err(Error::invalid("lag must be between 0 and 14 days"));
    }
    let exposure = exposure_series(conn, &spec.exposure, &spec.from, &spec.to)?;
    // Outcome window shifts by lag.
    let out_from = add_days(&spec.from, spec.lag_days)?;
    let out_to = add_days(&spec.to, spec.lag_days)?;
    let outcome = outcome_series(conn, &spec.outcome, &out_from, &out_to,
                                 spec.exclude_invalid, spec.exclude_familiarization)?;

    // Days excluded by context filters.
    let mut context_excluded_days: Vec<String> = Vec::new();
    if !spec.exclude_context_kinds.is_empty() {
        let ctx = crate::repo_daily::list_context_events(conn, &spec.from, &out_to)?;
        for c in ctx {
            let kind = c["kind"].as_str().unwrap_or_default().to_string();
            if !spec.exclude_context_kinds.contains(&kind) {
                continue;
            }
            let s = c["start_date"].as_str().unwrap_or(&spec.from).to_string();
            let e = c["end_date"].as_str().map(|x| x.to_string()).unwrap_or_else(|| out_to.clone());
            let mut d = s;
            while d <= e {
                context_excluded_days.push(d.clone());
                d = add_days(&d, 1)?;
            }
        }
    }

    // Pair per day: exposure(day) -> outcome(day + lag).
    let mut points: Vec<Value> = Vec::new();
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    let mut missing = 0i64;
    let mut excluded = outcome.excluded;
    let mut day = spec.from.clone();
    while day <= spec.to {
        let outcome_day = add_days(&day, spec.lag_days)?;
        if context_excluded_days.contains(&day) || context_excluded_days.contains(&outcome_day) {
            excluded += 1;
            day = add_days(&day, 1)?;
            continue;
        }
        match (exposure.get(&day), outcome.values.get(&outcome_day)) {
            (Some(x), Some(y)) => {
                pairs.push((*x, *y));
                points.push(json!({
                    "exposure_date": day, "outcome_date": outcome_day,
                    "exposure": x, "outcome": y,
                }));
            }
            (None, None) => missing += 2,
            _ => missing += 1,
        }
        day = add_days(&day, 1)?;
    }

    let min_pairs = settings::get(conn, "association_min_pairs")?.as_i64().unwrap_or(14);
    let rho = spearman(&pairs);

    // Split-half consistency: same correlation sign in both halves of the period.
    let consistent = if pairs.len() >= (min_pairs as usize) && rho.is_some() {
        let mid = pairs.len() / 2;
        match (spearman(&pairs[..mid]), spearman(&pairs[mid..])) {
            (Some(a), Some(b)) => (a > 0.0) == (b > 0.0) && a.abs() > 0.05 && b.abs() > 0.05,
            _ => false,
        }
    } else {
        false
    };

    // Baseline period caps the evidence label.
    let settings_all = settings::get_all(conn)?;
    let in_baseline = match settings_all["baseline_start"].as_str() {
        Some(start) => {
            let weeks = settings_all["baseline_weeks"].as_i64().unwrap_or(5);
            let baseline_end = add_days(start, weeks * 7)?;
            spec.to <= baseline_end
        }
        None => false,
    };

    let evidence_label = if (pairs.len() as i64) < min_pairs {
        "insufficient_data"
    } else if in_baseline {
        "descriptive"
    } else if rho.map(|r| r.abs() >= 0.3).unwrap_or(false) && consistent {
        "observational_signal"
    } else {
        "descriptive"
    };

    let mut caveats: Vec<String> = vec![
        "Observational data: an association is not evidence of causation.".into(),
        "Possible explanations include reverse causation, common causes, time trends, and selection effects.".into(),
    ];
    if (pairs.len() as i64) < min_pairs {
        caveats.push(format!(
            "Only {} paired observations; at least {} are needed for a signal to be considered.",
            pairs.len(), min_pairs
        ));
    }
    if missing > 0 {
        caveats.push(format!("{missing} day-values in this window are unknown (not recorded)."));
    }
    if in_baseline {
        caveats.push("This window falls inside the baseline period; results stay descriptive.".into());
    }
    if spec.outcome.kind == "assessment_metric" {
        caveats.push("Repeated tests carry familiarization/practice effects; early sessions are excluded by default.".into());
    }
    if excluded > 0 {
        caveats.push(format!("{excluded} observations excluded (invalid, familiarization, or context filters)."));
    }
    if spec.protocol_id.is_some() {
        caveats.push(
            "Linked to a research protocol: the formal conclusion is set explicitly on the protocol, \
             using only the protocol evidence labels."
                .into(),
        );
    }

    let groups = group_comparison(&pairs);
    let generated_at = now_rfc3339();
    let result = json!({
        "id": new_id(),
        "kind": if spec.protocol_id.is_some() { "protocol_result" } else { "association" },
        "generated_at": generated_at,
        "analysis_version": ANALYSIS_VERSION,
        "exposure_definition": {
            "kind": spec.exposure.kind, "template_id": spec.exposure.template_id,
            "dimension_id": spec.exposure.dimension_id, "label": spec.exposure.label,
            "version": ANALYSIS_VERSION,
        },
        "outcome_definition": {
            "kind": spec.outcome.kind, "dimension_id": spec.outcome.dimension_id,
            "assessment_kind": spec.outcome.assessment_kind, "metric": spec.outcome.metric,
            "label": spec.outcome.label, "version": ANALYSIS_VERSION,
        },
        "time_window": { "from": spec.from, "to": spec.to, "lag_days": spec.lag_days },
        "included_count": pairs.len(),
        "excluded_count": excluded,
        "missing_count": missing,
        "evidence_label": evidence_label,
        "caveats": caveats,
        "source_data_scope": {
            "from": spec.from, "to": spec.to, "lag_days": spec.lag_days,
            "exclude_invalid": spec.exclude_invalid,
            "exclude_familiarization": spec.exclude_familiarization,
            "exclude_context_kinds": spec.exclude_context_kinds,
        },
        "values_json": {
            "points": points,
            "spearman_rho": rho,
            "split_half_consistent": consistent,
            "min_pairs_threshold": min_pairs,
            "exposure_summary": summary(&pairs.iter().map(|p| p.0).collect::<Vec<_>>()),
            "outcome_summary": summary(&pairs.iter().map(|p| p.1).collect::<Vec<_>>()),
            "groups": groups,
        },
        "protocol_id": spec.protocol_id,
        "is_stale": 0,
    });

    if spec.persist {
        persist_result(conn, &result)?;
    }
    Ok(result)
}

/// Median-split comparison so the association is inspectable without statistics.
fn group_comparison(pairs: &[(f64, f64)]) -> Value {
    if pairs.len() < 4 {
        return json!(null);
    }
    let mut xs: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = xs[xs.len() / 2];
    let low: Vec<f64> = pairs.iter().filter(|p| p.0 <= median).map(|p| p.1).collect();
    let high: Vec<f64> = pairs.iter().filter(|p| p.0 > median).map(|p| p.1).collect();
    json!({
        "split": "exposure median",
        "split_value": median,
        "low_exposure_outcome": summary(&low),
        "high_exposure_outcome": summary(&high),
    })
}

fn persist_result(conn: &Connection, r: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO analysis_results
           (id, kind, generated_at, analysis_version, exposure_definition, outcome_definition,
            time_window, included_count, excluded_count, missing_count, evidence_label, caveats,
            source_data_scope, values_json, protocol_id, is_stale, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,0,?3)",
        rusqlite::params![
            r["id"].as_str(), r["kind"].as_str(), r["generated_at"].as_str(),
            r["analysis_version"].as_str(),
            r["exposure_definition"].to_string(), r["outcome_definition"].to_string(),
            r["time_window"].to_string(), r["included_count"].as_i64(), r["excluded_count"].as_i64(),
            r["missing_count"].as_i64(), r["evidence_label"].as_str(), r["caveats"].to_string(),
            r["source_data_scope"].to_string(), r["values_json"].to_string(),
            r["protocol_id"].as_str(),
        ],
    )?;
    Ok(())
}

pub fn list_results(conn: &Connection, limit: i64) -> Result<Vec<Value>> {
    query_json(
        conn,
        "SELECT * FROM analysis_results ORDER BY generated_at DESC LIMIT ?1",
        [limit],
    )
}

/// Explicit human promotion of a repeated observational signal to a candidate
/// hypothesis. Never automatic.
pub fn promote_to_candidate(conn: &Connection, result_id: &str, note: Option<&str>) -> Result<Value> {
    let source = crate::jsonq::query_one(
        conn, "SELECT * FROM analysis_results WHERE id=?1", [result_id])?
        .ok_or_else(|| Error::not_found("analysis result"))?;
    if source["evidence_label"].as_str() != Some("observational_signal") {
        return Err(Error::invalid(
            "only an observational signal can be promoted to a candidate hypothesis",
        ));
    }
    let mut promoted = source.clone();
    let new_id_str = new_id();
    promoted["id"] = json!(new_id_str);
    promoted["evidence_label"] = json!("candidate_hypothesis");
    promoted["generated_at"] = json!(now_rfc3339());
    let mut caveats: Vec<String> =
        serde_json::from_value(promoted["caveats"].clone()).unwrap_or_default();
    caveats.push("Promoted to candidate hypothesis by explicit user confirmation.".into());
    if let Some(n) = note {
        caveats.push(format!("User note: {n}"));
    }
    promoted["caveats"] = json!(caveats);
    let mut scope = promoted["source_data_scope"].clone();
    scope["promoted_from_result_id"] = json!(result_id);
    promoted["source_data_scope"] = scope;
    persist_result(conn, &promoted)?;
    Ok(promoted)
}
