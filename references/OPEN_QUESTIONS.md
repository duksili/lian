# Open Questions

Use this file for true unresolved design choices, implementation conflicts, or decisions requiring user input. Do not use it as a vague backlog.

## Current open questions

1. **Assessment schedule defaults:** Should first-run defaults be PVT three mornings weekly and Go/No-Go twice weekly, or should all schedules begin disabled until the user opts in?
2. **Physical assessment UX:** Should the weekly physical check be guided with a timer/voice prompts, or should it begin as manual result entry with instructions?
3. **Data encryption at rest:** Is OS-level account protection sufficient for v1, or should application-level encrypted database support be planned early?
4. **Backups:** Should automatic scheduled backup be a v1 hard requirement or a v1.1 feature after manual backup/restore is proven?
5. **Assessment window behavior:** Should a due assessment disappear after its window, become “late but available,” or remain available with a validity deviation flag?

## Decisions taken in the v1 implementation (2026-07-05, Fable)

1. **Assessment schedule defaults**
   - Decision: schedules exist as pre-configured rows (PVT Mon/Wed/Fri 07:00–11:00, Go/No-Go Tue/Thu, physical Sat) but **start disabled**. Onboarding offers one-click opt-in; ad-hoc runs are always available.
   - Why: a repeated test is a personal commitment; defaulting it on creates exactly the schedule pressure the charter avoids.

2. **Physical assessment UX**
   - Decision: **manual result entry** with in-app instructions and safety language (protocol `physical-1.0`). Attempts are stored as structured raw trials, so guided timers can layer on later without schema change.

3. **Data encryption at rest**
   - Decision: **OS-level account protection is the v1 boundary.** The database lives in the per-user XDG data directory. Application-level SQLCipher can be added later through the single `db::open` chokepoint; not implemented in v1.

4. **Backups**
   - Decision: manual backup/restore (manifest + SHA-256 + pre-restore safety copy) is the v1 requirement; **automatic scheduled backup deferred to v1.1**. Settings surfaces a missing destination and last-backup time so the gap is visible, not silent.

5. **Assessment window behavior**
   - Decision: a due assessment stays **available outside its window**; the deviation is recorded as validity reason `taken_outside_configured_window` (session marked `caution`, never blocked, never hidden).

6. **Assessment input method** (2026-07-05, R1 remediation LIAN-03)
   - Decision: v1 fixes the response input to the **spacebar** as an explicit protocol contract; both runners record `keyboard_spacebar` as session provenance, and validity evaluation compares the session's recorded method against the configured `assessment_input_method` setting, producing `input_method_differs_from_configured` on mismatch. A configurable key mapping is a later, deliberate change.

7. **PVT duration semantics** (2026-07-05, R1 remediation LIAN-01)
   - Decision: the runner consumes the seeded interval pool against a monotonic 5-minute deadline and stops as soon as the next response window cannot fit, so a completed session ends at most ~14 s before the deadline and never after it. Validity: ended > 20 s early → `incomplete_duration` (invalid); overrun > 15 s → `duration_overrun` (caution); > 60 s → invalid. `pvt-1.0` was never released with the prior nominal-schedule behavior, so the version identifier is retained.

8. **Recurrence model** (added for transparency; not in the original list)
   - Series definitions live in `plan_series`; occurrences are materialized into `plans` idempotently per `(series_id, occurrence_date)`. Editing or ending a series regenerates only future occurrences that are unlinked and untouched; everything else is immutable history.

## Decision record format

For future entries:

- Question:
- Why it matters:
- Options:
- Recommended option:
- Decision owner:
- Date decided:
- Result:


## Determinations

The v1 contract treats a determination as a voluntary user-defined commitment distinct from plans and the Five Precepts. The implementation must preserve the distinction. Any desired specialized Buddhist terminology or additional canonical determination categories should be added only through a later explicit product decision.
