#!/usr/bin/env bash
# Build Otto.app from this checkout and swap it into /Applications (`pnpm app:install`).
# Quitting Otto closes its terminals; Claude conversations stay listed under each
# project, ready to resume.
set -euo pipefail
cd "$(dirname "$0")/.."

APP="/Applications/Otto.app"
BUILT="target/release/bundle/macos/Otto.app"
BUNDLE_ID="co.lauf.otto"

echo "→ Building Otto.app (a few minutes the first time)…"
pnpm tauri build --bundles app

if pgrep -qf "$APP/Contents/MacOS/otto"; then
  echo "→ Quitting Otto…"
  osascript -e "tell application id \"$BUNDLE_ID\" to quit" || true
  for _ in $(seq 1 40); do
    pgrep -qf "$APP/Contents/MacOS/otto" || break
    sleep 0.25
  done
  if pgrep -qf "$APP/Contents/MacOS/otto"; then
    echo "Otto didn't quit. Quit it yourself (⌘Q) and run this again." >&2
    exit 1
  fi
fi

echo "→ Installing…"
rm -rf "$APP"
ditto "$BUILT" "$APP"
open "$APP"
echo "✓ Otto is updated and open."
