//! Assessment protocol engines: seeded sequence generation, derived-metric
//! calculation (versioned), and validity evaluation. Pure logic — raw trials
//! in, metrics out — so it is fully unit-testable and reproducible.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PVT_KIND: &str = "pvt_v1";
pub const GNG_KIND: &str = "go_no_go_v1";
pub const PHYSICAL_KIND: &str = "physical_weekly_v1";

pub const PVT_PROTOCOL_VERSION: &str = "pvt-1.0";
pub const GNG_PROTOCOL_VERSION: &str = "gng-1.0";
pub const PHYSICAL_PROTOCOL_VERSION: &str = "physical-1.0";
pub const METRICS_VERSION: &str = "metrics-1.0";

// PVT v1 fixed parameters (see references/PVT_PROTOCOL_V1.md).
pub const PVT_DURATION_MS: i64 = 5 * 60 * 1000;
pub const PVT_ISI_MIN_MS: i64 = 2000;
pub const PVT_ISI_MAX_MS: i64 = 10000;
pub const PVT_FALSE_START_MS: i64 = 100;
pub const PVT_LAPSE_MS: i64 = 500;
pub const PVT_TIMEOUT_MS: i64 = 3000; // no-response window per stimulus

// Go/No-Go v1 fixed parameters (see references/GO_NO_GO_PROTOCOL_V1.md).
pub const GNG_TRIALS: usize = 160;
pub const GNG_NO_GO_COUNT: usize = 40; // 25%
pub const GNG_STIMULUS_MS: i64 = 600; // stimulus visible / response window
pub const GNG_ISI_MIN_MS: i64 = 900;
pub const GNG_ISI_MAX_MS: i64 = 1500;

/// Generate the PVT interstimulus schedule for a session seed.
/// Returns planned intervals (ms) that fit inside the fixed duration.
pub fn pvt_schedule(seed: u64) -> Vec<i64> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut intervals = Vec::new();
    let mut elapsed: i64 = 0;
    loop {
        let isi = rng.gen_range(PVT_ISI_MIN_MS..=PVT_ISI_MAX_MS);
        // A stimulus must have room for its response window inside the test.
        if elapsed + isi + PVT_TIMEOUT_MS > PVT_DURATION_MS {
            break;
        }
        elapsed += isi;
        intervals.push(isi);
        elapsed += 0; // response time consumes real time in practice; schedule is nominal
    }
    intervals
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum GngStimulus {
    Go,
    NoGo,
}

/// Reproducible Go/No-Go trial sequence for a session seed:
/// exactly 160 trials, 120 Go / 40 No-Go, shuffled, with per-trial ISI.
pub fn gng_sequence(seed: u64) -> Vec<(GngStimulus, i64)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut kinds: Vec<GngStimulus> = Vec::with_capacity(GNG_TRIALS);
    for i in 0..GNG_TRIALS {
        kinds.push(if i < GNG_NO_GO_COUNT { GngStimulus::NoGo } else { GngStimulus::Go });
    }
    // Fisher–Yates with the seeded RNG so the order is retained/reproducible.
    for i in (1..kinds.len()).rev() {
        let j = rng.gen_range(0..=i);
        kinds.swap(i, j);
    }
    kinds
        .into_iter()
        .map(|k| (k, rng.gen_range(GNG_ISI_MIN_MS..=GNG_ISI_MAX_MS)))
        .collect()
}

/// Raw trial as submitted by the UI at session end. Field meanings follow the
/// assessment_trials schema; this struct is the raw contract for both tests.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RawTrial {
    pub trial_index: i64,
    #[serde(default)]
    pub stimulus_kind: Option<String>,
    #[serde(default)]
    pub planned_interval_ms: Option<i64>,
    #[serde(default)]
    pub onset_ms: Option<i64>,
    #[serde(default)]
    pub response_ms: Option<i64>,
    #[serde(default)]
    pub reaction_time_ms: Option<i64>,
    #[serde(default)]
    pub is_false_start: bool,
    #[serde(default)]
    pub visibility_lost: bool,
    #[serde(default)]
    pub payload: Option<Value>,
}

/// Classified PVT trial flags derived from raw data (derivation is versioned).
pub struct PvtDerived {
    pub is_lapse: bool,
    pub is_omission: bool,
    pub is_false_start: bool,
}

pub fn classify_pvt_trial(t: &RawTrial) -> PvtDerived {
    let rt = t.reaction_time_ms;
    let is_false_start = t.is_false_start || matches!(rt, Some(r) if r < PVT_FALSE_START_MS);
    let is_omission = !is_false_start && t.response_ms.is_none();
    let is_lapse = !is_false_start && matches!(rt, Some(r) if r >= PVT_LAPSE_MS);
    PvtDerived { is_lapse, is_omission, is_false_start }
}

fn median(sorted: &[i64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len();
    Some(if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) as f64 / 2.0
    })
}

fn mean(xs: &[i64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    Some(xs.iter().sum::<i64>() as f64 / xs.len() as f64)
}

fn std_dev(xs: &[i64]) -> Option<f64> {
    if xs.len() < 2 {
        return None;
    }
    let m = mean(xs)?;
    let var = xs.iter().map(|x| (*x as f64 - m).powi(2)).sum::<f64>() / (xs.len() - 1) as f64;
    Some(var.sqrt())
}

/// PVT derived metrics v1: reproducible from raw trials alone.
pub fn pvt_metrics(trials: &[RawTrial]) -> Value {
    let mut valid_rts: Vec<i64> = Vec::new();
    let mut false_starts = 0i64;
    let mut lapses = 0i64;
    let mut omissions = 0i64;
    for t in trials {
        let c = classify_pvt_trial(t);
        if c.is_false_start {
            false_starts += 1;
        } else if c.is_omission {
            omissions += 1;
        } else if let Some(rt) = t.reaction_time_ms {
            valid_rts.push(rt);
            if c.is_lapse {
                lapses += 1;
            }
        }
    }
    valid_rts.sort_unstable();
    let responded = valid_rts.len() as i64;
    json!({
        "metrics_version": METRICS_VERSION,
        "trial_count": trials.len(),
        "valid_trial_count": responded,
        "median_rt_ms": median(&valid_rts),
        "mean_rt_ms": mean(&valid_rts),
        "rt_sd_ms": std_dev(&valid_rts),
        "lapse_count": lapses,
        "lapse_rate": if responded + omissions > 0 {
            Some((lapses + omissions) as f64 / (responded + omissions) as f64)
        } else { None },
        "false_start_count": false_starts,
        "omission_count": omissions,
    })
}

/// Go/No-Go derived metrics v1.
pub fn gng_metrics(trials: &[RawTrial]) -> Value {
    let mut go_rts: Vec<i64> = Vec::new();
    let mut go_total = 0i64;
    let mut nogo_total = 0i64;
    let mut commissions = 0i64;
    let mut omissions = 0i64;
    for t in trials {
        match t.stimulus_kind.as_deref() {
            Some("go") => {
                go_total += 1;
                match t.reaction_time_ms {
                    Some(rt) if rt >= 0 => go_rts.push(rt),
                    _ => omissions += 1,
                }
            }
            Some("no_go") => {
                nogo_total += 1;
                if t.response_ms.is_some() {
                    commissions += 1;
                }
            }
            _ => {}
        }
    }
    go_rts.sort_unstable();
    json!({
        "metrics_version": METRICS_VERSION,
        "trial_count": trials.len(),
        "valid_trial_count": go_rts.len() as i64 + (nogo_total - commissions) + commissions,
        "go_trial_count": go_total,
        "no_go_trial_count": nogo_total,
        "commission_error_count": commissions,
        "commission_error_rate": if nogo_total > 0 { Some(commissions as f64 / nogo_total as f64) } else { None },
        "omission_count": omissions,
        "omission_rate": if go_total > 0 { Some(omissions as f64 / go_total as f64) } else { None },
        "go_rt_median_ms": median(&go_rts),
        "go_rt_mean_ms": mean(&go_rts),
        "go_rt_sd_ms": std_dev(&go_rts),
    })
}

/// Physical weekly summary: passthrough of structured attempts (no scoring).
pub fn physical_metrics(trials: &[RawTrial]) -> Value {
    let mut stance: Vec<Value> = Vec::new();
    let mut sit_to_stand: Vec<Value> = Vec::new();
    for t in trials {
        let p = t.payload.clone().unwrap_or(json!({}));
        match t.stimulus_kind.as_deref() {
            Some("single_leg_stance") => stance.push(p),
            Some("sit_to_stand") => sit_to_stand.push(p),
            _ => {}
        }
    }
    json!({
        "metrics_version": METRICS_VERSION,
        "single_leg_stance_attempts": stance,
        "sit_to_stand_attempts": sit_to_stand,
    })
}

/// Context the validity evaluation needs beyond the raw trials.
#[derive(Deserialize, Default)]
pub struct SessionContext {
    #[serde(default)]
    pub visibility_lost_count: i64,
    #[serde(default)]
    pub self_reported_interruption: Option<String>,
    #[serde(default)]
    pub input_method: Option<String>,
    #[serde(default)]
    pub configured_input_method: Option<String>,
    #[serde(default)]
    pub elapsed_ms: Option<i64>,
    #[serde(default)]
    pub outside_window: bool,
    #[serde(default)]
    pub aborted: bool,
}

/// Evaluate validity per the documented warning rules. Returns
/// (validity_state, reasons). Completed does not imply valid; invalid data
/// stays visible and exportable.
pub fn evaluate_validity(kind: &str, trials: &[RawTrial], ctx: &SessionContext) -> (String, Vec<String>) {
    let mut reasons: Vec<String> = Vec::new();
    let mut invalid = false;

    if ctx.aborted {
        reasons.push("session_aborted_early".into());
        invalid = true;
    }
    if ctx.visibility_lost_count > 0 {
        reasons.push("window_lost_visibility".into());
    }
    if ctx.self_reported_interruption.is_some() {
        reasons.push("self_reported_interruption".into());
    }
    if let (Some(used), Some(cfg)) = (&ctx.input_method, &ctx.configured_input_method) {
        if used != cfg {
            reasons.push("input_method_differs_from_configured".into());
        }
    }
    if ctx.outside_window {
        reasons.push("taken_outside_configured_window".into());
    }

    match kind {
        PVT_KIND => {
            let false_starts = trials.iter().filter(|t| classify_pvt_trial(t).is_false_start).count();
            if trials.len() >= 10 && false_starts * 5 >= trials.len() {
                reasons.push("excessive_false_starts".into()); // >= 20% of trials
            }
            if let Some(elapsed) = ctx.elapsed_ms {
                if elapsed < PVT_DURATION_MS - 5_000 {
                    reasons.push("incomplete_duration".into());
                    invalid = true;
                }
            }
        }
        GNG_KIND => {
            if trials.len() < GNG_TRIALS {
                reasons.push("incomplete_trial_count".into());
                if trials.len() < GNG_TRIALS / 2 {
                    invalid = true;
                }
            }
        }
        _ => {}
    }

    let state = if invalid {
        "invalid"
    } else if reasons.is_empty() {
        "valid"
    } else {
        "caution"
    };
    (state.into(), reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pvt_schedule_is_reproducible_and_bounded() {
        let a = pvt_schedule(42);
        let b = pvt_schedule(42);
        assert_eq!(a, b);
        assert!(a.iter().all(|i| (PVT_ISI_MIN_MS..=PVT_ISI_MAX_MS).contains(i)));
        let total: i64 = a.iter().sum();
        assert!(total + PVT_TIMEOUT_MS <= PVT_DURATION_MS);
        assert!(a.len() >= 25, "5 min at 2-10s ISI should give >= 25 stimuli, got {}", a.len());
    }

    #[test]
    fn gng_sequence_composition() {
        let seq = gng_sequence(7);
        assert_eq!(seq.len(), GNG_TRIALS);
        let nogo = seq.iter().filter(|(k, _)| *k == GngStimulus::NoGo).count();
        assert_eq!(nogo, GNG_NO_GO_COUNT);
        assert_eq!(gng_sequence(7).iter().map(|(k, _)| *k).collect::<Vec<_>>(),
                   seq.iter().map(|(k, _)| *k).collect::<Vec<_>>());
        // Different seeds should give different orders (overwhelmingly likely).
        assert_ne!(gng_sequence(8).iter().map(|(k, _)| *k).collect::<Vec<_>>(),
                   seq.iter().map(|(k, _)| *k).collect::<Vec<_>>());
    }

    fn rt_trial(idx: i64, rt: Option<i64>) -> RawTrial {
        RawTrial {
            trial_index: idx,
            stimulus_kind: Some("stimulus".into()),
            reaction_time_ms: rt,
            response_ms: rt.map(|r| 1000 + r),
            onset_ms: Some(1000),
            ..Default::default()
        }
    }

    #[test]
    fn pvt_metrics_classification() {
        let trials = vec![
            rt_trial(0, Some(250)),
            rt_trial(1, Some(310)),
            rt_trial(2, Some(520)),  // lapse
            rt_trial(3, Some(80)),   // false start (<100ms)
            rt_trial(4, None),       // omission
        ];
        let m = pvt_metrics(&trials);
        assert_eq!(m["valid_trial_count"], 3);
        assert_eq!(m["lapse_count"], 1);
        assert_eq!(m["false_start_count"], 1);
        assert_eq!(m["omission_count"], 1);
        assert_eq!(m["median_rt_ms"], serde_json::json!(310.0));
    }

    #[test]
    fn gng_metrics_rates() {
        let mut trials = Vec::new();
        for i in 0..120 {
            trials.push(RawTrial {
                trial_index: i,
                stimulus_kind: Some("go".into()),
                reaction_time_ms: if i < 110 { Some(400) } else { None },
                response_ms: if i < 110 { Some(1400) } else { None },
                ..Default::default()
            });
        }
        for i in 120..160 {
            trials.push(RawTrial {
                trial_index: i,
                stimulus_kind: Some("no_go".into()),
                response_ms: if i < 130 { Some(1200) } else { None }, // 10 commissions
                ..Default::default()
            });
        }
        let m = gng_metrics(&trials);
        assert_eq!(m["commission_error_count"], 10);
        assert_eq!(m["commission_error_rate"], serde_json::json!(0.25));
        assert_eq!(m["omission_count"], 10);
        assert_eq!(m["go_rt_median_ms"], serde_json::json!(400.0));
    }

    #[test]
    fn validity_rules() {
        // Complete clean PVT -> valid.
        let trials: Vec<RawTrial> = (0..30).map(|i| rt_trial(i, Some(300))).collect();
        let (state, reasons) = evaluate_validity(PVT_KIND, &trials, &SessionContext {
            elapsed_ms: Some(PVT_DURATION_MS),
            ..Default::default()
        });
        assert_eq!(state, "valid");
        assert!(reasons.is_empty());

        // Visibility loss -> caution, not invalid.
        let (state, reasons) = evaluate_validity(PVT_KIND, &trials, &SessionContext {
            elapsed_ms: Some(PVT_DURATION_MS),
            visibility_lost_count: 2,
            ..Default::default()
        });
        assert_eq!(state, "caution");
        assert!(reasons.contains(&"window_lost_visibility".to_string()));

        // Cut short -> invalid but retained.
        let (state, _) = evaluate_validity(PVT_KIND, &trials, &SessionContext {
            elapsed_ms: Some(60_000),
            ..Default::default()
        });
        assert_eq!(state, "invalid");

        // GNG with under half the trials -> invalid.
        let few: Vec<RawTrial> = (0..40).map(|i| RawTrial {
            trial_index: i, stimulus_kind: Some("go".into()), ..Default::default()
        }).collect();
        let (state, reasons) = evaluate_validity(GNG_KIND, &few, &SessionContext::default());
        assert_eq!(state, "invalid");
        assert!(reasons.contains(&"incomplete_trial_count".to_string()));
    }
}
