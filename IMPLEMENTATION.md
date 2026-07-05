# LIAN — Implementation Notes (v0.1.0)

This document describes the delivered implementation. The package documents
(`PRODUCT_CHARTER.md`, `contracts/`, etc.) remain the authoritative contracts.
R1 audit remediation (LIAN-01…09) is complete; see `docs/RC_LOG.md` for the
verification record and `docs/SMOKE_CHECKLIST.md` for the manual smoke list.

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
- **Reproducibility**: PVT interstimulus pools and Go/No-Go sequences are
  generated from a persisted per-session seed (ChaCha8). Derived metrics are
  versioned (`metrics-1.0`) and recomputable from raw trials alone.
- **Assessment timing is a pure state machine**: the protocol logic (deadline
  fitting, false starts, response windows, held-key rejection) lives in
  `src/lib/assessment/{pvtMachine,gngMachine}.ts` with injected clocks and
  vitest coverage; the Svelte runners only execute the machines' actions with
  real timers.
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
  (currently 2), migrations embedded in the binary. Databases newer than the
  binary supports are refused rather than opened.
- Backups: `VACUUM INTO` snapshot + JSON manifest (created time, app version,
  schema version, SHA-256, size), logged in `backups_log`. Restore requires a
  *trusted* backup (integrity + manifest checksum + supported schema);
  manifest-less files need an explicit advanced-recovery confirmation, and
  integrity failures or future schemas are always refused. The swap is staged
  and rollback-safe: a safety copy is written first, the candidate is verified
  in place, and any failure puts the previous database back automatically.
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
npm ci
npm run tauri dev        # launches the desktop app with hot reload
```

Release build:

```bash
npm run tauri build      # produces deb + rpm under target/release/bundle/
sudo dnf install ./target/release/bundle/rpm/LIAN-0.1.0-1.x86_64.rpm
```

Repeatable verification (one command):

```bash
./scripts/verify.sh            # core tests, strict clippy, vitest, svelte-check, build
./scripts/verify.sh --shell    # + compile the desktop shell
./scripts/verify.sh --bundle   # + produce release bundles
```

Wayland note: the binary sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself
(webkit2gtk's DMABUF renderer crashes on some Wayland/GPU stacks); export the
variable yourself to override.

## Test evidence

- `crates/lian-core` unit tests (9): PVT pool reproducibility and deadline
  fit, metric classification (lapse/false-start/omission, commission/omission
  rates), validity rules (caution vs invalid, duration under/overrun,
  key-hold), quiet-hours windows crossing midnight, timezone-aware local
  dates, migration from empty.
- `crates/lian-core/tests/integration.rs` (16): quick log → enrich → audit →
  backfill-with-unknown-time → delete; check-ins with unknown ratings;
  precepts statuses; determination revision/review/supersede lifecycle;
  recurring plans (materialization idempotence, series edit immutability,
  explicit linking, neutral skip); deadline-respecting PVT and GNG session
  cycles with raw-trial persistence and validity metadata; input-method
  mismatch flagging; PVT overrun deviation; reminder safeguards (dedupe,
  quiet hours, indefinite vs self-clearing temporary pause, cross-midnight
  plan offsets); association transparency and promotion gate; protocol
  version discipline **and** protocol-linked analysis lifecycle
  (spec pre-registration → linked result → lock → amendment forks v2);
  restore trust gating and rollback (tampered/manifest-less/future-schema
  refusal, corrupt-candidate no-op, failed-open rollback, round-trip,
  forward schema guard).
- `src/lib/assessment/*.test.ts` (13, vitest, fake clock): PVT deadline
  boundary/normal/all-timeout runs, false-start/omission/RT recording,
  key-repeat rejection; GNG in-window and late-window acceptance,
  post-window and next-ISI rejection, held-key counting, full-sequence
  commit semantics.
- Runtime: debug and release binaries exercised on the Fedora target with
  seeded data; screenshots in `docs/screenshots/`; record in `docs/RC_LOG.md`.

## Known limitations / deferred

- Manual smoke items that need a human at the keyboard remain open in
  `docs/SMOKE_CHECKLIST.md` (tray clicks, a real 5-minute PVT sitting,
  notification delivery moment, dialog-driven restore rehearsal, RPM install).
- AppImage bundling requires linuxdeploy prerequisites not present on the
  build host; deb/rpm are the default targets.
- Automatic scheduled backups deferred to v1.1 (decision record 4).
- Parquet export deferred; the SQLite copy is the analysis-friendly format.
- No wearable/CSV import (architecture reserves `source='import'` and
  source-tracking columns).
- Drag-and-drop calendar rescheduling is approximated with one-click
  day-shift controls and full editing.
- Analysis offers median-split comparison + Spearman rank correlation with
  split-half consistency checks; no regression/confounder adjustment (by
  design — stronger claims require a protocol).
- The IPC boundary is `(method, JSON)` dispatched to typed handlers in Rust;
  it is validated and closed (no SQL crosses), but not compile-time-typed
  end-to-end.
