-- LIAN schema v1.
-- Conventions:
--   * ids are UUIDv4 strings
--   * instants are RFC3339 text with UTC offset (offset-aware)
--   * local dates are 'YYYY-MM-DD' text derived from the configured timezone
--   * JSON columns hold serde-serialized structures
--   * missing/unknown is NULL or an explicit status value, never 0/'' by convention

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL, -- JSON
  updated_at TEXT NOT NULL
);

CREATE TABLE audit_log (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL, -- 'update' | 'delete' | 'restore' | 'lifecycle'
  changed_at TEXT NOT NULL,
  prior_values TEXT, -- JSON snapshot of fields before the change
  reason TEXT
);
CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);

CREATE TABLE activity_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL, -- 'meditation'|'taiji'|'yoga_mobility'|'walking'|'strength_sport'|'recovery'|'custom'
  glyph TEXT,             -- short decorative marker chosen by the app
  color TEXT,             -- token name from the app palette, not a hex requirement
  subtypes TEXT NOT NULL DEFAULT '[]', -- JSON array of subtype strings
  default_duration_seconds INTEGER,
  supports_intensity INTEGER NOT NULL DEFAULT 0,
  supports_body_state INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_archived INTEGER NOT NULL DEFAULT 0,
  is_builtin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE activity_events (
  id TEXT PRIMARY KEY,
  template_id TEXT NOT NULL REFERENCES activity_templates(id),
  status TEXT NOT NULL DEFAULT 'completed', -- 'completed' | 'cancelled'
  occurred_at TEXT,            -- NULL when only the date is known
  local_date TEXT NOT NULL,    -- always present; the day this belongs to
  time_known INTEGER NOT NULL DEFAULT 1,
  timezone TEXT NOT NULL,
  logged_at TEXT NOT NULL,
  duration_seconds INTEGER,    -- NULL = unknown duration (never coerced to 0)
  subtype TEXT,
  intensity INTEGER,           -- 1..5
  perceived_quality INTEGER,   -- 1..5
  body_state_before TEXT,
  body_state_after TEXT,
  location TEXT,
  note TEXT,
  context_tags TEXT NOT NULL DEFAULT '[]', -- JSON array
  plan_id TEXT,                -- explicit link, never inferred
  source TEXT NOT NULL DEFAULT 'manual', -- 'manual'|'timer'|'import'|'system'|'derived'
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
CREATE INDEX idx_events_date ON activity_events(local_date);
CREATE INDEX idx_events_template ON activity_events(template_id);
CREATE INDEX idx_events_plan ON activity_events(plan_id);

CREATE TABLE checkin_dimensions (
  id TEXT PRIMARY KEY,
  key TEXT NOT NULL UNIQUE, -- 'calm', 'energy', ...
  label TEXT NOT NULL,
  anchor_low TEXT NOT NULL,
  anchor_high TEXT NOT NULL,
  scale_min INTEGER NOT NULL DEFAULT 1,
  scale_max INTEGER NOT NULL DEFAULT 5,
  scale_version INTEGER NOT NULL DEFAULT 1,
  is_enabled INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE daily_checkins (
  id TEXT PRIMARY KEY,
  local_date TEXT NOT NULL,
  logged_at TEXT NOT NULL,
  timezone TEXT NOT NULL,
  note TEXT,
  sleep_start TEXT,             -- RFC3339
  sleep_end TEXT,
  sleep_duration_minutes INTEGER,
  sleep_quality INTEGER,        -- 1..5
  awakenings INTEGER,
  context_tags TEXT NOT NULL DEFAULT '[]',
  source TEXT NOT NULL DEFAULT 'manual',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
CREATE INDEX idx_checkins_date ON daily_checkins(local_date);

CREATE TABLE checkin_ratings (
  id TEXT PRIMARY KEY,
  checkin_id TEXT NOT NULL REFERENCES daily_checkins(id) ON DELETE CASCADE,
  dimension_id TEXT NOT NULL REFERENCES checkin_dimensions(id),
  value INTEGER NOT NULL,
  scale_version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_ratings_checkin ON checkin_ratings(checkin_id);

-- Five Precepts: one record per local date, upserted; entries per canonical precept.
CREATE TABLE precept_records (
  id TEXT PRIMARY KEY,
  local_date TEXT NOT NULL UNIQUE,
  logged_at TEXT NOT NULL,
  timezone TEXT NOT NULL,
  overall_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE precept_entries (
  id TEXT PRIMARY KEY,
  record_id TEXT NOT NULL REFERENCES precept_records(id) ON DELETE CASCADE,
  precept_key TEXT NOT NULL, -- canonical: non_harming_life | not_taking_unoffered | responsible_sexual_conduct | truthful_harmless_speech | clarity_regarding_intoxicants
  status TEXT NOT NULL,      -- 'observed'|'not_observed'|'uncertain'|'not_reviewed'
  note TEXT,
  UNIQUE(record_id, precept_key)
);

CREATE TABLE context_events (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL, -- 'illness'|'injury'|'travel'|'workload'|'emotional_stress'|'routine_change'|'practice_change'|'sleep_disruption'|'caffeine'|'alcohol'|'custom'
  label TEXT NOT NULL,
  started_at TEXT NOT NULL,      -- RFC3339 or date-anchored midnight
  ended_at TEXT,                 -- NULL = ongoing or point event
  start_date TEXT NOT NULL,      -- local date
  end_date TEXT,
  tags TEXT NOT NULL DEFAULT '[]',
  note TEXT,
  source TEXT NOT NULL DEFAULT 'manual',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
CREATE INDEX idx_context_dates ON context_events(start_date, end_date);

CREATE TABLE determinations (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  started_on TEXT NOT NULL,     -- local date
  ends_on TEXT,                 -- NULL = open-ended
  lifecycle_state TEXT NOT NULL DEFAULT 'active', -- 'active'|'paused'|'completed'|'discontinued'|'superseded'
  review_cadence TEXT,          -- NULL|'daily'|'weekly'|'monthly'
  category TEXT,
  predecessor_id TEXT,
  superseded_by_id TEXT,
  completed_at TEXT,
  paused_at TEXT,
  discontinued_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE determination_revisions (
  id TEXT PRIMARY KEY,
  determination_id TEXT NOT NULL REFERENCES determinations(id) ON DELETE CASCADE,
  revised_at TEXT NOT NULL,
  prior_title TEXT NOT NULL,
  prior_description TEXT
);

CREATE TABLE determination_reviews (
  id TEXT PRIMARY KEY,
  determination_id TEXT NOT NULL REFERENCES determinations(id) ON DELETE CASCADE,
  local_date TEXT NOT NULL,
  logged_at TEXT NOT NULL,
  status TEXT NOT NULL, -- 'kept'|'not_kept'|'uncertain'|'not_reviewed'
  note TEXT,
  UNIQUE(determination_id, local_date)
);

CREATE TABLE determination_links (
  id TEXT PRIMARY KEY,
  determination_id TEXT NOT NULL REFERENCES determinations(id) ON DELETE CASCADE,
  linked_type TEXT NOT NULL, -- 'plan'|'activity_event'|'checkin'|'context_event'|'note'
  linked_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(determination_id, linked_type, linked_id)
);

-- Recurring plans: a series defines recurrence; occurrences are materialized
-- into `plans` rows so past occurrences are immutable history.
CREATE TABLE plan_series (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  kind TEXT NOT NULL, -- 'activity'|'assessment'|'recovery'|'commitment'|'custom'
  activity_template_id TEXT,
  assessment_kind TEXT,
  frequency TEXT NOT NULL,  -- 'daily'|'weekly'|'monthly'
  interval INTEGER NOT NULL DEFAULT 1,
  weekdays TEXT NOT NULL DEFAULT '[]', -- JSON array of 0..6 (Mon=0) for weekly
  month_day INTEGER,        -- for monthly
  time_of_day TEXT,         -- 'HH:MM' local, NULL = date-only
  duration_minutes INTEGER,
  timezone TEXT NOT NULL,
  starts_on TEXT NOT NULL,  -- local date
  until TEXT,               -- local date, NULL = open
  target_duration_seconds INTEGER,
  determination_id TEXT,
  protocol_id TEXT,
  note TEXT,
  reminder_offset_minutes INTEGER, -- NULL = no reminder
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE plans (
  id TEXT PRIMARY KEY,
  series_id TEXT REFERENCES plan_series(id),
  occurrence_date TEXT,     -- local date the series occurrence belongs to
  title TEXT NOT NULL,
  kind TEXT NOT NULL,
  activity_template_id TEXT,
  assessment_kind TEXT,
  scheduled_start TEXT,     -- RFC3339; NULL when date-only
  scheduled_end TEXT,
  local_date TEXT NOT NULL,
  date_only INTEGER NOT NULL DEFAULT 0,
  timezone TEXT NOT NULL,
  target_duration_seconds INTEGER,
  status TEXT NOT NULL DEFAULT 'upcoming', -- 'upcoming'|'due'|'completed_linked'|'completed_unlinked'|'skipped'|'cancelled'|'expired_unresolved'
  note TEXT,
  determination_id TEXT,
  protocol_id TEXT,
  reminder_offset_minutes INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT,
  UNIQUE(series_id, occurrence_date)
);
CREATE INDEX idx_plans_date ON plans(local_date);

CREATE TABLE plan_links (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
  activity_event_id TEXT,
  assessment_session_id TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX idx_plan_links_plan ON plan_links(plan_id);

CREATE TABLE reminder_rules (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL, -- 'evening_checkin'|'weekly_review'|'monthly_review'|'assessment_window'|'determination_review'|'recovery'|'plan'
  label TEXT NOT NULL,
  time_of_day TEXT,        -- 'HH:MM' local
  weekdays TEXT NOT NULL DEFAULT '[]', -- JSON array 0..6 (Mon=0); [] = every day (or per-kind semantics)
  target_id TEXT,          -- determination id / assessment kind, etc.
  enabled INTEGER NOT NULL DEFAULT 1,
  snoozed_until TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE notification_log (
  id TEXT PRIMARY KEY,
  rule_id TEXT,
  plan_id TEXT,
  kind TEXT NOT NULL,
  dedupe_key TEXT NOT NULL, -- prevents re-firing the same logical reminder
  fired_at TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_notiflog_dedupe ON notification_log(dedupe_key);

CREATE TABLE assessment_sessions (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL, -- 'pvt_v1'|'go_no_go_v1'|'physical_weekly_v1'
  protocol_version TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'in_progress', -- 'planned'|'in_progress'|'completed'|'aborted'|'invalid'
  started_at TEXT,
  ended_at TEXT,
  timezone TEXT NOT NULL,
  input_method TEXT,
  device_metadata TEXT NOT NULL DEFAULT '{}', -- JSON
  pre_test TEXT NOT NULL DEFAULT '{}',        -- JSON: protocol-defined condition fields
  self_reported_interruption TEXT,
  visibility_lost_count INTEGER NOT NULL DEFAULT 0,
  validity_state TEXT NOT NULL DEFAULT 'unreviewed', -- 'valid'|'caution'|'invalid'|'unreviewed'
  validity_reasons TEXT NOT NULL DEFAULT '[]',       -- JSON array of reason codes
  is_familiarization INTEGER NOT NULL DEFAULT 0,
  session_seed TEXT,
  derived_metrics TEXT,     -- JSON, reproducible from raw trials
  metrics_version TEXT,
  plan_id TEXT,
  protocol_id TEXT,
  note TEXT,
  local_date TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_sessions_kind_date ON assessment_sessions(kind, local_date);

CREATE TABLE assessment_trials (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES assessment_sessions(id) ON DELETE CASCADE,
  trial_index INTEGER NOT NULL,
  stimulus_kind TEXT,           -- pvt: 'stimulus'; gng: 'go'|'no_go'; physical: 'single_leg_stance'|'sit_to_stand'
  planned_interval_ms INTEGER,  -- pvt ISI
  onset_ms INTEGER,             -- ms from session start
  response_ms INTEGER,          -- ms from session start, NULL = no response
  reaction_time_ms INTEGER,
  is_false_start INTEGER NOT NULL DEFAULT 0,
  is_lapse INTEGER NOT NULL DEFAULT 0,
  is_omission INTEGER NOT NULL DEFAULT 0,
  is_commission_error INTEGER NOT NULL DEFAULT 0,
  is_correct INTEGER,           -- NULL where correctness does not apply
  visibility_lost INTEGER NOT NULL DEFAULT 0,
  payload TEXT NOT NULL DEFAULT '{}', -- kind-specific raw detail (e.g. stance side, touchdowns)
  UNIQUE(session_id, trial_index)
);

CREATE TABLE assessment_schedules (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL DEFAULT 0,
  weekdays TEXT NOT NULL DEFAULT '[]',   -- JSON array 0..6 Mon=0
  window_start TEXT NOT NULL DEFAULT '07:00',
  window_end TEXT NOT NULL DEFAULT '11:00',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE research_protocols (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  question TEXT NOT NULL,
  hypothesis TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft', -- 'draft'|'planned'|'active'|'paused'|'completed'|'cancelled'|'superseded'
  start_date TEXT,
  end_date TEXT,
  primary_outcome_definition TEXT NOT NULL, -- JSON versioned definition
  intervention_definition TEXT NOT NULL,
  analysis_plan TEXT NOT NULL,
  secondary_outcomes TEXT,
  adherence_requirements TEXT,
  context_variables TEXT,
  stop_criteria TEXT,
  notes TEXT,
  results_locked INTEGER NOT NULL DEFAULT 0, -- set once results viewed; edits then require a new version
  predecessor_id TEXT,
  conclusion TEXT, -- one of the protocol evidence labels once concluded
  conclusion_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE analysis_results (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL, -- 'descriptive'|'association'|'protocol_result'
  generated_at TEXT NOT NULL,
  analysis_version TEXT NOT NULL,
  exposure_definition TEXT NOT NULL,  -- JSON
  outcome_definition TEXT NOT NULL,   -- JSON
  time_window TEXT NOT NULL,          -- JSON {lag_days, ...}
  included_count INTEGER NOT NULL,
  excluded_count INTEGER NOT NULL,
  missing_count INTEGER NOT NULL,
  evidence_label TEXT NOT NULL,
  caveats TEXT NOT NULL DEFAULT '[]',
  source_data_scope TEXT NOT NULL,    -- JSON filter definition for reproduction
  values_json TEXT NOT NULL DEFAULT '{}', -- points, group stats
  protocol_id TEXT,
  is_stale INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE weekly_reflections (
  id TEXT PRIMARY KEY,
  week_start TEXT NOT NULL UNIQUE, -- Monday local date
  note TEXT,
  logged_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE backups_log (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  path TEXT NOT NULL,
  app_version TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  checksum_sha256 TEXT NOT NULL,
  size_bytes INTEGER NOT NULL,
  ok INTEGER NOT NULL,
  message TEXT
);
