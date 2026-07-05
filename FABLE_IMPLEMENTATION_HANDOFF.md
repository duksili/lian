# Fable Implementation Handoff

## Assignment

Create LIAN from scratch as an exceptionally polished, local-first desktop application.

This handoff defines product meaning, required behavior, data contracts, scientific/interpretation safeguards, and v1 boundaries. It does not prescribe a visual design system or screen layout.

The visual result matters. Do not treat this as a functional admin tool that happens to have charts; create a calm, coherent, premium-feeling personal desktop product that is a pleasure to return to for years.

## Frozen product constraints

- Desktop-first.
- Tauri + Svelte/TypeScript UI + Rust core + SQLite baseline.
- Local-first; no account or cloud dependency in v1.
- User-owned data with backup, restore, and export.
- Activity plans and completed activity remain separate records.
- Five Precepts are private reflection, not moral scoring.
- Determinations are voluntary user-defined commitments, distinct from plans and Five Precepts; they are private and non-punitive.
- PVT and Go/No-Go retain raw trials and versioned protocols.
- Analysis distinguishes association from causation.
- Reminders are configurable, quiet, privacy-safe, and non-punitive.

## Design freedom

Fable is expected to independently decide:

- information architecture;
- navigation;
- layout;
- visual identity;
- component system;
- typography;
- interaction patterns;
- responsive desktop behavior;
- onboarding presentation;
- charts and data visualizations;
- empty states;
- visual treatment of privacy-sensitive material.

Fable must not weaken the frozen constraints or change domain/data semantics without documenting the conflict in `references/OPEN_QUESTIONS.md`.

## Visual quality bar

Fable has wide design freedom, but the implementation must meet this bar:

- The result should feel intentionally designed at every level: app shell, hierarchy, data entry, calendar, assessments, review, and privacy-sensitive reflection.
- Favor calm clarity over generic wellness clichés, productivity gamification, dense admin-dashboard clutter, or spiritual ornamentation that distracts from use.
- Make the daily loop exceptionally frictionless while making review and research feel trustworthy and deeply inspectable.
- Establish a coherent visual system: type scale, spacing rhythm, elevation/surface treatment, feedback, interaction states, data-viz language, and keyboard/focus behavior.
- Treat the calendar, timeline, assessment sessions, and weekly/monthly review as flagship experiences, not utility pages.
- Use restrained motion only where it improves orientation or feedback; no ornamental animation.
- Make unknown, missing, paused, deferred, invalid, and privacy-sensitive states unmistakable without making the user feel judged.
- Do a dedicated visual refinement pass before declaring delivery complete. Remove default-looking controls, unresolved spacing/alignment issues, placeholder surfaces, and inconsistent behavior.

The exact visual style remains Fable’s responsibility. The requirement is a high-end, coherent finish—not a predefined aesthetic.

## Delivery expectation

### Milestone 1 — Foundations and daily use

Implement:

- application shell and durable local storage;
- activity templates and quick logging;
- daily check-in;
- Five Precepts reflection;
- determinations;
- calendar/plans and reminders;
- timeline;
- system tray behavior;
- backup/export foundations.

### Milestone 2 — Assessments

Implement:

- PVT v1;
- Go/No-Go v1;
- physical assessment records;
- validity metadata and raw-trial storage;
- assessment scheduling and completion flows.

### Milestone 3 — Review and research

Implement:

- weekly review;
- monthly review;
- transparent descriptive/association views;
- research protocol creation and tracking;
- data-quality visibility;
- full backup/restore/export polish.

## Implementation discipline

- Preserve stable IDs, timestamps, source/provenance, protocol versions, and validity state.
- Do not expose arbitrary SQL to the frontend; keep data access controlled by the Rust core.
- Do not make external networking a requirement for core capability.
- Prefer a compact coherent product over a collection of disconnected pages.
- Keep unfinished/deferred features visible only when they add genuine clarity; do not create misleading placeholders.
- When a requirement is ambiguous, record a specific question and proposed options in `references/OPEN_QUESTIONS.md`.

## Completion evidence

For each milestone, deliver:

- source changes;
- schema/migration changes;
- concise mapping from accepted criteria to implemented behavior;
- test evidence for core data integrity and assessment logic;
- manual walkthrough notes;
- representative screenshots or visual-review notes covering core flows and state handling;
- any unresolved deviations or open questions.
