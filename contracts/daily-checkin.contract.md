# Daily Check-in Contract

## Purpose

Represents a user-entered subjective state record for one local calendar date.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `local_date` | Date in configured user timezone |
| `logged_at` | Timestamp of entry |
| `timezone` | IANA timezone identifier or equivalent stable representation |
| `ratings` | Map of configured dimension IDs to ordinal values |
| `source` | Normally `manual` |
| `created_at` / `updated_at` | Audit timestamps |

## Optional fields

- sleep start/end timestamps
- sleep duration
- sleep quality
- note
- context tags
- linked life-event IDs

## Invariants

- One day may have multiple check-ins; a daily summary view may choose a current/latest representation but must preserve raw entries.
- Rating scales are versioned/configured and retain labels/anchors.
- Omitted ratings are unknown, not zero.
- No opaque “wellness score” may be derived as the only displayed representation.
