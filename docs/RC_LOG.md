# Release-candidate log — v0.1.0-rc2 (2026-07-05, Fedora 44)

Follow-up to the R1 source audit; all P0/P1/P2 items (LIAN-01…09) addressed.
Environment: Fedora 44, rustc 1.92.0, node 22.22.2, KDE Wayland.

## Automated verification (all executed on the target machine)

| Check | Command | Result |
|---|---|---|
| Locked install | `npm ci` | ok, 0 vulnerabilities reported |
| Core tests | `cargo test -p lian-core` | **25 passed, 0 failed** (unit 9 + integration 16) |
| Core lints | `cargo clippy -p lian-core --all-targets -- -D warnings` | clean |
| Shell lints | `cargo clippy -p lian-app -- -D warnings` | clean |
| Frontend unit tests | `npm test` (vitest) | **13 passed** (PVT/GNG state machines, fake clock) |
| Type/AY11 check | `npm run check` | **0 errors, 0 warnings** |
| Frontend build | `npm run build` | ok |
| Release bundles | `npm run tauri build` | `LIAN_0.1.0_amd64.deb` (5.8 MB), `LIAN-0.1.0-1.x86_64.rpm` (5.8 MB) |

One command for all of the above: `./scripts/verify.sh --bundle`.

AppImage: `linuxdeploy` fails on this host (FUSE/appstream tooling); target
removed from defaults. deb/rpm cover Fedora/Debian; AppImage can be produced
on a host/CI with linuxdeploy prerequisites by adding `"appimage"` back to
`bundle.targets`.

## Runtime verification (real desktop, this machine)

- Debug and **release** binaries launched on the Wayland session
  (XWayland backend for scripted window capture).
- Found and fixed a real crash: webkit2gtk DMABUF renderer aborts with
  Wayland protocol error 71 on this GPU stack. The binary now sets
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself (user-overridable).
- Database created at `~/.local/share/org.lian.app/lian.sqlite3`,
  migrated to schema v2, seeded; survived restart.
- ~5 weeks of demo data written through the real core APIs
  (`cargo run -p lian-core --example seed_demo`), then every primary view
  exercised in the running app via a dev-only autopilot (which also proved
  frontend→backend round-trips by clearing its own trigger flag in settings).
- Screenshots captured from the running app: `docs/screenshots/`
  (onboarding, today [release build], timeline, calendar, assessments,
  weekly review, research explorer, quick-log modal, settings).
- RPM contents verified (`/usr/bin/lian-app`, desktop entry, icons).

## Remaining manual steps (interactive-only; see docs/SMOKE_CHECKLIST.md)

Installing the RPM (`sudo dnf install ./target/release/bundle/rpm/LIAN-0.1.0-1.x86_64.rpm`),
tray interaction, a real 5-minute PVT sitting, held-key GNG probe, notification
delivery at a reminder moment, and the backup→restore rehearsal with dialogs.
These need a human at the keyboard; everything they exercise is covered by
automated tests at the logic level.

## Note on current data

The live database currently contains the seeded demo data from this session.
To start clean: Settings → Data → "Delete…" (type the phrase), or delete
`~/.local/share/org.lian.app/lian.sqlite3*` while the app is closed.
