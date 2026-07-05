# Determination Contract

## Purpose

Represents a voluntary, user-defined personal commitment or resolution held over time. A determination is not a scheduled plan, completed activity, Five Precepts record, score, streak, badge, or behavior-policing mechanism.

A determination may describe a practice emphasis, restraint, conduct intention, or other personally meaningful commitment. It can be open-ended or time-bounded and may be reviewed privately on a chosen cadence.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `title` | User-visible wording of the determination |
| `started_on` | Local date on which it became active |
| `lifecycle_state` | Current lifecycle state |
| `created_at` / `updated_at` | Audit timestamps |

## Optional fields

- `description` or private rationale
- `ends_on`
- `review_cadence` and review reminder rule ID
- review rule / user-defined observable criteria
- links to related planned activities, activity events, check-ins, context events, and notes
- private category/tags
- predecessor/superseded-by determination IDs
- `completed_at`, `paused_at`, `discontinued_at`, or equivalent lifecycle timestamps

## Lifecycle states

- `active`
- `paused`
- `completed`
- `discontinued`
- `superseded`

## Optional review entries

A review entry exists only when the determination has an explicit review rule. It contains:

- `id`
- `determination_id`
- `local_date`
- `logged_at`
- `status`
- optional private note and context links

Allowed review statuses:

- `kept`
- `not_kept`
- `uncertain`
- `not_reviewed`

## Invariants

- A calendar plan, activity event, or reminder may be linked to a determination but must not silently set its review status.
- Absence of a review entry never means `not_kept`.
- Editing a determination’s current wording must preserve prior wording or create a revision/supersession record.
- Completing, pausing, discontinuing, or superseding a determination must be explicit.
- The app must not calculate aggregate moral scores, streaks, badges, rankings, or punitive messaging from determination data.
- Determination titles, notes, and review details must not appear in generic system notifications by default.
- Determination data is private and user-controlled.
