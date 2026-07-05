# Scope and Non-Goals

## v1 scope

### Tracking

- Meditation, Taiji, yoga/mobility, walking, sport, strength/cardio, recovery/rest, and custom activity templates.
- Duration, timing, type, intensity, subjective quality, optional before/after body state, optional private note.
- Daily ratings for a small user-selected set of state dimensions.
- Sleep timing, duration, quality, and optional contextual factors.
- Context/life events such as illness, injury, travel, unusual workload, emotional stress, late caffeine, alcohol, or environmental disruption.
- Five Precepts daily reflection.
- User-defined personal determinations, with lifecycle, private review, optional reminder support, and optional links to calendar plans.
- One-off and recurring plans in a calendar.

### Assessments

- PVT v1.
- Go/No-Go v1.
- A simple weekly physical assessment flow (single-leg stance and five-times sit-to-stand) recorded as an assessment session.
- Raw trial or attempt-level storage where the assessment produces trials.

### Review and research

- Daily timeline.
- Weekly summary and missing-data review.
- Monthly trend and transparent association review.
- Candidate hypotheses.
- Structured protocols with explicit hypothesis, schedule, outcome, adherence, and conclusion status.

### Foundations

- SQLite source-of-truth data.
- Backup, restore, CSV export, and analysis-friendly export.
- Local notifications, system tray behavior, and quiet hours.
- Correction, deletion, and audit metadata.

## Explicit non-goals for v1

- Cloud account, mandatory sign-in, social features, sharing, leaderboards, streak pressure, or public profiles.
- Clinical diagnostics, medical interpretation, treatment recommendations, or medical claims.
- “Brain age,” opaque wellness scores, gamified moral scores, or pseudo-scientific composite scores.
- AI-generated conclusions that are not traceable to raw observations and defined methods.
- Mobile app, cloud sync, wearable API integration, voice transcription, or external calendar sync.
- Full causal-inference platform, generalized machine-learning system, or automated intervention prescription.
- Broad cognitive battery. PVT and Go/No-Go are enough for the initial cognitive layer.
- External data collection that makes daily use depend on internet access.
- Automatic inference that a missed log means no practice, non-observance, or a broken determination.

## Deferred but anticipated

The architecture should allow, but not implement prematurely:

- wearable imports with source provenance;
- a lightweight mobile logger;
- environmental-data imports;
- optional additional assessments;
- advanced analysis exported to Python/R/DuckDB;
- encrypted backup transport or optional sync;
- calendar import/export.
