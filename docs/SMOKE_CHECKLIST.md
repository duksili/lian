# LIAN desktop smoke checklist

Run against a real build (`npm run tauri dev` for iteration; the installed
RPM/AppImage before calling a release done). Automated coverage already
handles data contracts (`cargo test -p lian-core`), assessment timing logic
(`npm test`), and type/build checks — this list covers what only a running
desktop can prove.

Legend: [x] verified on Fedora (2026-07-05, debug build driven headlessly,
screenshots under the release-candidate log) · [ ] to re-verify on the
installed bundle.

## Startup and shell

- [x] App launches, opens its window, and creates
      `~/.local/share/org.lian.app/lian.sqlite3` (schema migrated, seeded).
- [x] First run shows onboarding; completed onboarding lands on Today.
- [x] Relaunch with existing data goes straight to Today with data intact.
- [ ] Closing the window keeps the tray icon; "Open LIAN" restores the window.
- [ ] Tray "Pause reminders 2h" suppresses reminders and self-clears after 2h
      (verify `notifications_paused` flips back via Settings display).
- [ ] Tray "Quit LIAN" exits the process.

## Daily loop

- [x] Today shows plans, recorded practice, check-in, precepts, determinations.
- [x] Quick log (button and `L`) records an activity; it appears on Today and
      Timeline immediately.
- [ ] Timer flow: start timer, wait, press `L`, banner converts elapsed time
      into the entry (source shows "timer").
- [x] Check-in save and edit; precepts reflect and re-edit; both survive relaunch.
- [x] Timeline backfill on an earlier day; unknown days show the unknown note.

## Calendar and reminders

- [x] Week/month views render plans, completed events, context pills.
- [x] Recurring series materializes occurrences; day-shift arrows reschedule.
- [ ] A plan with a reminder offset produces exactly one system notification
      at the right moment; dismissing it never creates a failure state.
- [ ] Quiet hours suppress reminders; snooze delays a rule.

## Assessments

- [ ] PVT: full 5-minute run ends on its own close to 5:00 (check
      `ended_at - started_at` in the session detail); counter feels instant;
      false start feedback appears when pressing early; Esc offers
      keep/discard; results show metrics + validity.
- [ ] Go/No-Go: full 160-trial run; held-key produces no phantom responses
      (hold space through several trials — expect `accidental_key_hold_detected`
      caution, no responses attributed); commission/omission counts sane.
- [ ] Physical check form saves attempts; session appears in history.
- [x] Session history lists validity pills; detail shows raw trials.
- [ ] Losing window focus mid-test yields the visibility caution.

## Data safety

- [ ] Backup to a chosen directory produces `.sqlite3` + manifest; verify
      shows "trusted".
- [ ] Restore rehearsal: create data → backup → add more data → restore →
      exactly the backed-up state returns; safety copy exists.
- [ ] Restore of a tampered/manifest-less file is refused unless the advanced
      recovery confirmation is given.
- [ ] Export produces CSVs + SQLite copy + manifest.
- [ ] Purge with typed phrase wipes data and restarts empty; wrong phrase is
      rejected (also enforced in Rust).
- [x] "Open data location" reveals the database in the file manager
      (permission `opener:allow-reveal-item-in-dir` only).

## Review and research

- [x] Weekly review renders volume, plan-vs-actual, coverage grid, precepts,
      context; reflection saves.
- [x] Monthly review renders week bars and metric/dimension trends.
- [x] Research explorer runs an association and shows counts, scatter,
      caveats, evidence label; saving persists to Results.
- [ ] Candidate → protocol → activate → "Run predefined analysis" → linked
      result appears in protocol detail; amending afterwards forks version 2.

## Window behavior

- [x] Compact width (~960px) collapses the nav rail to icons; nothing
      essential is hidden.
- [ ] Wayland session: app starts without `WEBKIT_DISABLE_DMABUF_RENDERER`
      set by hand (the binary sets it itself).
