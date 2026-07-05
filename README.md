# LIAN — Fable Implementation Handoff

**Package version:** 1.2  
**Status:** Repository-ready product and contract baseline  
**Scope:** Documentation and contracts only. No code, visual specification, or component library is included.

## Purpose

LIAN is a private, local-first desktop application for recording practice and life data over time, completing repeatable assessments, planning activities and personal determinations, and reviewing evidence-aware personal patterns.

It is not a generic habit tracker, a clinical device, a productivity system, a social platform, or an app that claims to prove causation from noisy personal data.

## Intended implementation baseline

- Desktop-first application
- Tauri application shell
- Svelte + TypeScript user interface
- Rust application core
- SQLite source-of-truth database
- Local-first by default
- No account, cloud service, or mandatory network dependency in v1

Fable may choose the visual identity, information architecture, navigation, layouts, components, interaction patterns, and implementation details that do not conflict with this package.

## Repository-ready root files

This package can be unzipped directly as the initial `lian` repository root. It intentionally contains no application scaffold or runtime code. The root hygiene files are included so implementation can begin cleanly:

- `.gitignore` — ignores local databases, backups, exports, build output, logs, secrets, and machine-specific files.
- `.gitattributes` — normalizes text handling and marks binary assets correctly.
- `.editorconfig` — establishes portable formatting defaults.
- `AGENTS.md` — concise operating rules for implementation agents.
- `CONTRIBUTING.md` — change discipline for human or agent contributors.
- `SECURITY.md` — handling and reporting rules for privacy-sensitive local data.
- `LICENSE` — private, all-rights-reserved default; change it deliberately before open-sourcing.

## Document authority

When documents appear to conflict, use this order:

1. `PRODUCT_CHARTER.md`
2. `SCOPE_AND_NON_GOALS.md`
3. `DOMAIN_AND_DATA_CONTRACTS.md`
4. Files under `contracts/`
5. `ASSESSMENTS_AND_VALIDITY_RULES.md`
6. `RESEARCH_AND_ANALYSIS_RULES.md`
7. `ACCEPTANCE_CRITERIA.md`
8. Other product documents

A conflict that cannot be resolved from this order must be recorded in `references/OPEN_QUESTIONS.md` rather than silently decided.

## Package map

| Area | Primary files |
|---|---|
| Product purpose and boundaries | `PRODUCT_CHARTER.md`, `SCOPE_AND_NON_GOALS.md` |
| Required behavior | `CORE_USER_FLOWS.md`, `ACCEPTANCE_CRITERIA.md` |
| Domain and data shape | `DOMAIN_AND_DATA_CONTRACTS.md`, `contracts/` |
| Daily tracking, Five Precepts, and determinations | `DAILY_TRACKING_AND_PRACTICE_MODEL.md`, `contracts/five-precepts.contract.md`, `contracts/determination.contract.md` |
| Calendar, planning, reminders | `CALENDAR_REMINDERS_AND_PLANNING.md`, `contracts/planned-activity.contract.md` |
| Assessments | `ASSESSMENTS_AND_VALIDITY_RULES.md`, `references/` |
| Research and review | `RESEARCH_AND_ANALYSIS_RULES.md`, `contracts/research-protocol.contract.md`, `contracts/analysis-result.contract.md` |
| Privacy, data ownership, exports | `LOCAL_FIRST_PRIVACY_AND_EXPORT.md` |
| Fable delivery boundary | `FABLE_IMPLEMENTATION_HANDOFF.md` |

## Product principles

- Logging must be quick enough to survive normal life.
- Missing data is **unknown**, never automatically “no”, “failure”, or “non-adherence”.
- The app must preserve raw observations, source, timing, and protocol versioning.
- Review language must distinguish observation, association, candidate hypothesis, and stronger protocol result.
- Five Precepts tracking is private self-reflection, not moral scoring or gamification.
- Determinations are user-defined voluntary commitments, distinct from calendar plans and the Five Precepts; they must be reviewable without shame, streak pressure, or automatic failure.
- Planning is distinct from completed activity; the app must never rewrite a plan as a completion.
- The app must remain useful before advanced analysis exists.

## Initial delivery sequence

1. Local data foundation, migration, backup, restore, and export.
2. Daily tracking, calendar/planning, Five Precepts reflection, determinations, reminders, and timeline.
3. PVT, Go/No-Go, assessment validity handling, and raw trial storage.
4. Weekly review, monthly research review, and simple transparent associations.
5. Formal research protocols and any later extensions.

See `FABLE_IMPLEMENTATION_HANDOFF.md` for delivery expectations.
