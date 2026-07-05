# LIAN — Implementation Notes (v0.1.0)

This document describes the delivered implementation. The package documents
(`PRODUCT_CHARTER.md`, `contracts/`, etc.) remain the authoritative contracts.

## Architecture

```
┌───────────────────────────────────────────────┐
│ Svelte 5 + TypeScript UI (src/)               │
│  App shell · Today · Timeline · Calendar      │
│  Assessments (PVT/GNG runners) · Review       │
│  Research · Determinations · Settings         │
└──────────────┬────────────────────────────────┘
               │ one Tauri command: api(method, payload)
┌──────────────▼────────────────────────────────┐
│ src-tauri (thin shell)                        │
│  window/tray lifecycle · notification loop    │
│  restore/purge (connection swap) · dialogs    │
└──────────────┬────────────────────────────────┘
┌──────────────▼────────────────────────────────┐
│ crates/lian-core (pure Rust, no Tauri)        │
│  db/migrations · repositories · assessment    │
│  engines · analysis · reminders · backup/     │
│  export · typed API dispatch                  │
└──────────────┬────────────────────────────────┘
        SQLite (WAL) — source of truth
```

Key decisions:

- **All domain logic lives in `lian-core`**, which has no GUI dependency and is
  fully covered by headless tests (`cargo test -p lian-core`). The Tauri shell
  is ~200 lines and forwards `(method, payload)` to `lian_core::api::dispatch`.
  No SQL crosses the IPC boundary.
- **Reproducibility**: PVT interstimulus schedules and Go/No-Go sequences are
  generated from a persisted per-session seed (ChaCha8). Derived metrics are
  versioned (`metrics-1.0`) and recomputable from raw trials alone.
- **Missing ≠ zero, everywhere**: unlogged days are absent rows; analysis
  reports them as `missing_count`; review coverage shows them as "unknown";
  reminders and plans never convert absence into failure.
- **Audit trail**: edits and deletions of events, check-ins, plans,
  determinations, sessions, and protocols snapshot prior values into
  `audit_log` with an optional reason.
- **Recurrence**: `plan_series` holds the rule; occurrences materialize into
  `plans` idempotently. Series edits regenerate only future, unlinked,
  untouched occurrences (see `references/OPEN_QUESTIONS.md`, decision 6).

## Storage

- Live database: `~/.local/share/org.lian.app/lian.sqlite3` (Tauri
  `app_data_dir` on Linux; per-user, durable, outside the install dir).
  Settings → Data shows the exact path with an "open location" action.
- SQLite in WAL mode, foreign keys on, schema version in `PRAGMA user_version`
  (currently 1), migrations embedded in the binary.
- Backups: `VACUUM INTO` snapshot + JSON manifest (created time, app version,
  schema version, SHA-256, size), logged in `backups_log`. Restore verifies
  integrity + checksum, writes a safety copy (`safety-copies/pre-restore-*`),
  then swaps the file with the connection closed.
- Export: CSV per domain table (23 tables incl. raw `assessment_trials`),
  a full SQLite copy for analysis tools, and `export-manifest.json` with a
  data dictionary and conventions (timestamps, missing-data semantics).
- Permanent deletion: typed-confirmation purge removes the database files
  entirely and restarts from an empty, reseeded schema.

## Building and running (Fedora / Linux)

System dependencies (one-time):

```bash
sudo dnf install -y webkit2gtk4.1-devel gtk3-devel librsvg2-devel \
  openssl-devel libappindicator-gtk3 curl wget file
# Rust ≥1.80 via rustup, Node ≥20
```

Development:

```bash
npm install
npm run tauri dev        # launches the desktop app with hot reload
```

Release build:

```bash
npm run tauri build      # produces deb/rpm/AppImage under src-tauri/target/release/bundle/
```

Verification without GUI dependencies:

```bash
cargo test -p lian-core  # 20 tests: contracts, assessments, analysis, backup
npm run check            # svelte-check (0 errors)
npm run build            # frontend production build
```

## Test evidence

- `crates/lian-core/src` unit tests (9): PVT/GNG sequence reproducibility and
  composition, metric classification (lapse/false-start/omission, commission/
  omission rates), validity rules (caution vs invalid), quiet-hours windows
  crossing midnight, timezone-aware local dates, migration from empty.
- `crates/lian-core/tests/integration.rs` (11): quick log → enrich → audit →
  backfill-with-unknown-time → delete; check-ins with unknown ratings;
  precepts statuses (invalid status rejected); determination revision/review/
  supersede lifecycle; recurring plans (materialization idempotence, series
  edit immutability, explicit linking, neutral skip); full PVT and GNG session
  cycles with raw-trial persistence and validity metadata; association
  analysis transparency (counts, points, caveats, evidence labels, promotion
  gate); protocol version discipline (amend-after-view forks v2, conclusion
  label whitelist); reminder safeguards (dedupe, pause, quiet hours); backup/
  verify/export cycle against real files.

## Known limitations / deferred

- **The Tauri shell binary was not compiled in the delivery environment** —
  the machine lacked webkit2gtk/gtk3 dev packages and sudo. `lian-core`
  (all logic) is fully tested; the frontend passes svelte-check and builds.
  The shell is intentionally minimal, but `npm run tauri build` on a machine
  with the listed packages is the remaining verification step, and a visual
  pass in the running app may surface small polish items.
- Automatic scheduled backups deferred to v1.1 (decision record 4).
- Parquet export deferred; the SQLite copy is the analysis-friendly format.
- No wearable/CSV import (architecture reserves `source='import'` and
  source-tracking columns).
- Drag-and-drop calendar rescheduling is approximated with one-click
  day-shift controls and full editing.
- Analysis offers median-split comparison + Spearman rank correlation with
  split-half consistency checks; no regression/confounder adjustment (by
  design — stronger claims require a protocol).
