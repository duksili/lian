# Research Protocol Contract

## Purpose

Represents a predefined personal research plan created after baseline and/or candidate-hypothesis review.

## Required fields

| Field | Type / semantics |
|---|---|
| `id` | Stable opaque identifier |
| `title` | Human-readable protocol name |
| `version` | Immutable version label |
| `question` | Plain-language research question |
| `hypothesis` | Predefined expected relationship or outcome |
| `status` | Lifecycle status |
| `start_date` / `end_date` | Planned dates |
| `primary_outcome_definition` | Versioned outcome metric |
| `intervention_definition` | What changes or is scheduled |
| `analysis_plan` | Predefined comparison and lag logic |
| `created_at` / `updated_at` | Audit timestamps |

## Optional fields

- secondary outcomes
- planned schedule / planned activity references
- adherence requirements
- carryover/washout assumptions
- required context variables
- exclusion rules
- stop/pause criteria
- notes
- result IDs

## Lifecycle status

- `draft`
- `planned`
- `active`
- `paused`
- `completed`
- `cancelled`
- `superseded`

## Invariants

- Primary outcome and analysis plan cannot be silently rewritten after results are viewed; amendment creates a new version.
- Protocols must retain negative/null/inconclusive outcomes.
- Protocol results cannot claim universal causality.
- A protocol may be paused for illness, travel, injury, or other predeclared/contextual reasons.
