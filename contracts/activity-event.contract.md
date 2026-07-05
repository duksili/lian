# Activity Event Contract

## Purpose

Represents a completed action or exposure that occurred in time. Plans are not activity events.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `activity_template_id` | Reference to activity template; custom event may use a preserved archived template |
| `occurred_at` | Offset-aware timestamp, or explicit date-only/unknown-time representation |
| `logged_at` | Timestamp when record was created |
| `duration_seconds` | Non-negative duration; may be absent only for event types without duration |
| `source` | `manual`, `timer`, `import`, `system`, or `derived` |
| `created_at` / `updated_at` | Audit timestamps |
| `status` | `completed` or `cancelled`; cancelled must not be treated as completed |

## Optional structured fields

- `subtype`
- `intensity`
- `perceived_quality`
- `body_state_before`
- `body_state_after`
- `location`
- `note`
- `context_tags`
- `plan_id`
- `metadata`

## Invariants

- `occurred_at` describes reality; `logged_at` describes entry time.
- A duration of zero does not mean missing duration.
- An omitted duration does not mean zero.
- An event is never automatically inferred solely from a plan.
- Archived templates must remain resolvable for historical events.
- A linked plan must be explicit and auditable.
