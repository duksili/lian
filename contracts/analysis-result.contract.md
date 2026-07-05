# Analysis Result Contract

## Purpose

Represents a transparent, reproducible output from defined data and analysis logic.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `kind` | `descriptive`, `association`, `protocol_result` |
| `generated_at` | Timestamp |
| `analysis_version` | Version of calculation logic |
| `exposure_definition` | Versioned exposure description |
| `outcome_definition` | Versioned outcome description |
| `time_window` | Same-day/lag/custom window |
| `included_count` | Number of included observations |
| `excluded_count` | Number of excluded observations |
| `missing_count` | Relevant missing-data count |
| `evidence_label` | One approved label from research rules |
| `caveats` | Human-readable caution list |
| `source_data_scope` | IDs/query/filter definition sufficient for reproduction |

## Optional fields

- context filters/strata
- metric values / intervals / charts
- linked protocol ID
- inspectable observation IDs
- generated narrative summary

## Invariants

- Narrative must not exceed the evidence label.
- Result must retain an inspectable path to included/excluded data.
- Regeneration after data correction should create a fresh result or clearly mark the old result stale.
- The system must not suppress null or contrary findings.
