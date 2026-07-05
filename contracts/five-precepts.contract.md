# Five Precepts Daily Record Contract

## Purpose

Represents private daily reflection on the Five Precepts. This is not an activity record, score, streak system, or behavior-policing mechanism.

## Canonical precepts

1. `non_harming_life`
2. `not_taking_unoffered`
3. `responsible_sexual_conduct`
4. `truthful_harmless_speech`
5. `clarity_regarding_intoxicants`

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `local_date` | Date of reflection in configured timezone |
| `logged_at` | Timestamp of entry |
| `entries` | One status per canonical precept |
| `created_at` / `updated_at` | Audit timestamps |

## Allowed entry status

- `observed`
- `not_observed`
- `uncertain`
- `not_reviewed`

## Optional fields

- per-precept private note
- overall private reflection note
- context tag links

## Invariants

- No status may be inferred from absence of a record.
- The application must not create an aggregate morality score.
- The application must not display streaks, badges, rankings, or punitive messaging.
- Private notes must not appear in generic notifications.
- Historical terminology may be displayed in user-chosen wording, but canonical IDs remain stable.
