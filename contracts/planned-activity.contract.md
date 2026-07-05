# Planned Activity Contract

## Purpose

Represents an intention, scheduled activity, assessment, recovery period, or custom commitment. It does not prove completion.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `title` | User-visible plan title |
| `kind` | `activity`, `assessment`, `recovery`, `commitment`, or `custom` |
| `scheduled_start` | Offset-aware timestamp or date-only planned representation |
| `timezone` | Timezone used for occurrence and recurrence |
| `status` | Plan lifecycle state |
| `created_at` / `updated_at` | Audit timestamps |

## Optional fields

- `scheduled_end`
- `activity_template_id`
- `assessment_kind`
- `target_duration_seconds`
- recurrence rule and recurrence anchor
- reminder rule IDs
- protocol ID
- note
- context/tags
- planned intensity or quality target

## Plan status

- `upcoming`
- `due`
- `completed_linked`
- `completed_unlinked`
- `skipped`
- `cancelled`
- `expired_unresolved`

## Invariants

- Editing a future recurring plan does not change historical plan occurrences.
- Completion requires explicit link to an activity event or assessment session.
- The user may mark a plan skipped/cancelled without creating a negative score.
- A plan may exist without an activity template, allowing generic commitments.
