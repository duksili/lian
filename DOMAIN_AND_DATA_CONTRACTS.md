# Domain and Data Contracts

## Domain vocabulary

| Term | Meaning |
|---|---|
| Activity Event | A completed action or exposure that occurred in time: meditation, Taiji, walking, caffeine, sport, rest, etc. |
| Activity Template | User-configurable definition of an activity type and allowed optional fields. |
| Daily Check-in | A dated set of subjective ratings and optional sleep/context information. |
| Five Precepts Daily Record | A private daily self-reflection record containing one status per precept. |
| Determination | A voluntary, user-defined practice commitment or resolution with explicit scope, lifecycle, and optional review rule. It is distinct from both a calendar plan and Five Precepts reflection. |
| Context Event | A notable condition or life event that can affect interpretation: illness, travel, workload, injury, etc. |
| Planned Activity | A future intended activity or commitment, optionally recurring. |
| Plan–Actual Link | Explicit relationship between a plan and a completed event. |
| Reminder Rule | A configurable reminder associated with an activity, plan, assessment, review, or protocol. |
| Assessment Session | One completed, cancelled, aborted, or invalid assessment attempt under a specific protocol version. |
| Assessment Trial | A single raw trial/attempt within an assessment session. |
| Research Protocol | A structured future-facing study plan. |
| Analysis Result | A versioned, transparent output describing an observation or association. |

## Cross-cutting data rules

### Identity

- Every persisted entity must use a stable UUID or equivalent opaque identifier.
- Identifiers must never encode personal data, date meaning, or mutable status.
- Relations must be explicit rather than inferred from labels.

### Time

- Store `occurred_at` as an offset-aware timestamp whenever time is known.
- Store `logged_at` for manual entry time.
- Store a local date derived using the user’s configured timezone for daily views.
- Store the timezone/offset used at record creation where required for later interpretation.
- A record with only a date must explicitly state that time is unknown.

### Source and provenance

All records must carry a source where applicable:

- `manual`
- `timer`
- `assessment_engine`
- `import`
- `derived`
- `system`

Imported and derived records must retain their source system and source record identifier where available.

### Unknown, missing, and not recorded

- Missing data must not be encoded as `false`, `0`, empty string, or “did not happen”.
- Use explicit absence, `unknown`, or `not_reviewed` based on the contract.
- Analysis must distinguish completed-zero from not-recorded.

### Correction and deletion

- User edits must update `updated_at`.
- Where feasible, prior values and a correction reason should be retained in an audit trail.
- Deletion must be user-controlled and observable; soft deletion is allowed only if permanent deletion is also available.

### Versioning

- Assessment protocols, research protocols, exports, derived metrics, and analysis results must carry a version.
- Contract-breaking schema changes must be migrated deliberately and retain backward-read capability where practical.

## Contract hierarchy

The compact contracts in `contracts/` define minimum field requirements and behavioral invariants. Implementations may add fields but may not weaken required semantics.
