#!/usr/bin/env bash
# LIAN repeatable verification. Run from the repository root.
#
#   ./scripts/verify.sh          # headless checks (core + frontend)
#   ./scripts/verify.sh --shell  # also compile the desktop shell (needs webkit2gtk)
#   ./scripts/verify.sh --bundle # also produce release bundles (slow)
#
# Desktop runtime behavior (tray, notifications, restore swap) additionally
# needs the manual smoke checklist: docs/SMOKE_CHECKLIST.md

set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n\033[1m== %s ==\033[0m\n' "$1"; }

step "npm ci (locked install)"
npm ci

step "Rust core tests"
cargo test -p lian-core

step "Rust lints (warnings are errors)"
cargo clippy -p lian-core --all-targets -- -D warnings

step "Frontend unit tests (assessment state machines)"
npm test

step "svelte-check (0 errors, 0 warnings required)"
CHECK_OUT=$(npm run check 2>&1) || { echo "$CHECK_OUT"; exit 1; }
echo "$CHECK_OUT" | tail -2
if ! echo "$CHECK_OUT" | grep -q "0 ERRORS" || ! echo "$CHECK_OUT" | grep -q "0 WARNINGS"; then
  echo "svelte-check reported errors or warnings"; exit 1
fi

step "Frontend production build"
npm run build

if [[ "${1:-}" == "--shell" || "${1:-}" == "--bundle" ]]; then
  step "Desktop shell compile + lints"
  cargo clippy -p lian-app -- -D warnings
  cargo build -p lian-app
fi

if [[ "${1:-}" == "--bundle" ]]; then
  step "Release bundles (deb/rpm/AppImage)"
  npm run tauri build
fi

printf '\n\033[1mAll verification steps passed.\033[0m\n'
