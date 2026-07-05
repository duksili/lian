//! First-run seed data: built-in activity templates, check-in dimensions,
//! assessment schedules, and default reminder rules. Everything seeded here
//! stays user-editable; seeding is idempotent.

use rusqlite::{params, Connection};

use crate::util::{new_id, now_rfc3339};
use crate::Result;

pub struct TemplateSeed {
    pub name: &'static str,
    pub category: &'static str,
    pub glyph: &'static str,
    pub color: &'static str,
    pub subtypes: &'static [&'static str],
    pub supports_intensity: bool,
    pub supports_body_state: bool,
}

pub const TEMPLATE_SEEDS: &[TemplateSeed] = &[
    TemplateSeed { name: "Meditation", category: "meditation", glyph: "◦", color: "indigo",
        subtypes: &["seated", "walking", "guided", "chanting / recitation", "informal mindfulness", "retreat / extended"],
        supports_intensity: false, supports_body_state: false },
    TemplateSeed { name: "Taiji", category: "taiji", glyph: "☯", color: "teal",
        subtypes: &["form", "standing", "silk-reeling", "applications / partner work", "correction / study", "class", "mixed"],
        supports_intensity: true, supports_body_state: true },
    TemplateSeed { name: "Yoga / mobility", category: "yoga_mobility", glyph: "⌇", color: "moss",
        subtypes: &["yoga", "stretching", "joint mobility", "breath work"],
        supports_intensity: true, supports_body_state: true },
    TemplateSeed { name: "Walking", category: "walking", glyph: "→", color: "ochre",
        subtypes: &["casual", "brisk", "hike", "commute"],
        supports_intensity: true, supports_body_state: false },
    TemplateSeed { name: "Strength / sport / cardio", category: "strength_sport", glyph: "▲", color: "rust",
        subtypes: &["strength", "cardio", "sport", "intervals"],
        supports_intensity: true, supports_body_state: true },
    TemplateSeed { name: "Recovery / rest", category: "recovery", glyph: "◡", color: "slate",
        subtypes: &["nap", "deliberate rest", "sauna / bath", "massage / self-care"],
        supports_intensity: false, supports_body_state: false },
    TemplateSeed { name: "Custom activity", category: "custom", glyph: "·", color: "plum",
        subtypes: &[],
        supports_intensity: true, supports_body_state: false },
];

pub struct DimensionSeed {
    pub key: &'static str,
    pub label: &'static str,
    pub anchor_low: &'static str,
    pub anchor_high: &'static str,
    pub default_enabled: bool,
}

pub const DIMENSION_SEEDS: &[DimensionSeed] = &[
    DimensionSeed { key: "calm", label: "Calm", anchor_low: "agitated", anchor_high: "deeply settled", default_enabled: true },
    DimensionSeed { key: "energy", label: "Energy", anchor_low: "depleted", anchor_high: "abundant", default_enabled: true },
    DimensionSeed { key: "focus", label: "Focus", anchor_low: "scattered", anchor_high: "sharp", default_enabled: true },
    DimensionSeed { key: "body_tension", label: "Body tension", anchor_low: "loose", anchor_high: "very tense", default_enabled: true },
    DimensionSeed { key: "mood", label: "Mood", anchor_low: "low", anchor_high: "bright", default_enabled: false },
    DimensionSeed { key: "recovery", label: "Recovery", anchor_low: "worn down", anchor_high: "restored", default_enabled: false },
    DimensionSeed { key: "sleepiness", label: "Sleepiness", anchor_low: "wide awake", anchor_high: "can barely stay up", default_enabled: false },
    DimensionSeed { key: "pain_stiffness", label: "Pain / stiffness", anchor_low: "none", anchor_high: "severe", default_enabled: false },
];

pub const PRECEPT_KEYS: &[(&str, &str)] = &[
    ("non_harming_life", "Non-harming of life"),
    ("not_taking_unoffered", "Not taking what is not offered"),
    ("responsible_sexual_conduct", "Responsible sexual conduct"),
    ("truthful_harmless_speech", "Truthful, harmless speech"),
    ("clarity_regarding_intoxicants", "Clarity regarding intoxicants"),
];

pub fn ensure_seeded(conn: &Connection) -> Result<()> {
    let now = now_rfc3339();

    let templates: i64 = conn.query_row("SELECT COUNT(*) FROM activity_templates", [], |r| r.get(0))?;
    if templates == 0 {
        for (i, t) in TEMPLATE_SEEDS.iter().enumerate() {
            conn.execute(
                "INSERT INTO activity_templates
                   (id, name, category, glyph, color, subtypes, supports_intensity, supports_body_state,
                    sort_order, is_builtin, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1,?10,?10)",
                params![
                    new_id(), t.name, t.category, t.glyph, t.color,
                    serde_json::to_string(t.subtypes)?,
                    t.supports_intensity as i64, t.supports_body_state as i64,
                    i as i64, now
                ],
            )?;
        }
    }

    let dims: i64 = conn.query_row("SELECT COUNT(*) FROM checkin_dimensions", [], |r| r.get(0))?;
    if dims == 0 {
        for (i, d) in DIMENSION_SEEDS.iter().enumerate() {
            conn.execute(
                "INSERT INTO checkin_dimensions
                   (id, key, label, anchor_low, anchor_high, is_enabled, sort_order, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
                params![new_id(), d.key, d.label, d.anchor_low, d.anchor_high, d.default_enabled as i64, i as i64, now],
            )?;
        }
    }

    // Assessment schedules exist as rows but start disabled: taking a
    // repeated test is an opt-in commitment (see OPEN_QUESTIONS #1 decision).
    let schedules: i64 = conn.query_row("SELECT COUNT(*) FROM assessment_schedules", [], |r| r.get(0))?;
    if schedules == 0 {
        let rows = [
            ("pvt_v1", "[0,2,4]", "07:00", "11:00"),        // Mon/Wed/Fri suggestion, disabled
            ("go_no_go_v1", "[1,3]", "07:00", "11:00"),     // Tue/Thu suggestion, disabled
            ("physical_weekly_v1", "[5]", "08:00", "20:00"), // Saturday suggestion, disabled
        ];
        for (kind, weekdays, ws, we) in rows {
            conn.execute(
                "INSERT INTO assessment_schedules (id, kind, enabled, weekdays, window_start, window_end, created_at, updated_at)
                 VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?6)",
                params![new_id(), kind, weekdays, ws, we, now],
            )?;
        }
    }

    // Default reminder rules, disabled until onboarding/settings enables them.
    let rules: i64 = conn.query_row("SELECT COUNT(*) FROM reminder_rules", [], |r| r.get(0))?;
    if rules == 0 {
        let rows: [(&str, &str, Option<&str>, &str); 5] = [
            ("evening_checkin", "Evening check-in", Some("20:30"), "[]"),
            ("weekly_review", "Weekly review", Some("18:00"), "[6]"), // Sunday
            ("monthly_review", "Monthly review", Some("18:00"), "[]"),
            ("determination_review", "Determination review", Some("19:30"), "[]"),
            ("recovery", "Yesterday recovery prompt", Some("09:30"), "[]"),
        ];
        for (kind, label, time, weekdays) in rows {
            conn.execute(
                "INSERT INTO reminder_rules (id, kind, label, time_of_day, weekdays, enabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
                params![new_id(), kind, label, time, weekdays, now],
            )?;
        }
    }

    Ok(())
}
