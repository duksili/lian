# Assessment Session Contract

## Purpose

Represents a single attempt at a versioned assessment protocol.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `assessment_kind` | `pvt_v1`, `go_no_go_v1`, `physical_weekly_v1`, or future registered type |
| `protocol_version` | Immutable protocol identifier/version |
| `status` | `planned`, `in_progress`, `completed`, `aborted`, `invalid` |
| `started_at` / `ended_at` | Offset-aware timestamps where applicable |
| `timezone` | User/device timezone |
| `input_method` | Keyboard/input type where relevant |
| `device_metadata` | Platform and relevant display/input metadata |
| `validity_state` | `valid`, `caution`, `invalid`, or `unreviewed` |
| `created_at` / `updated_at` | Audit timestamps |

## Optional fields

- linked plan ID
- linked protocol ID
- pre-test condition values
- self-reported interruption
- validity reason list
- derived metrics and metrics version
- notes
- assessment trial IDs/attempt IDs

## Invariants

- Raw trials/attempts must be retained for trial-based tests.
- Derived metrics must be reproducible from raw data and versioned logic.
- Completed does not imply valid.
- Invalid sessions remain visible/exportable and are excluded from default primary analysis.
- Session device and protocol changes must be inspectable.
