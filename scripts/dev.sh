#!/usr/bin/env bash
# Start the Worker on a local D1 seeded with fixtures, then run the TUI against it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT/worker"
[ -d node_modules ] || npm install
[ -f .dev.vars ] || echo 'API_TOKEN=devtoken' > .dev.vars
npx wrangler d1 migrations apply kontu --local >/dev/null
npx wrangler d1 execute kontu --local --file=seed.sql >/dev/null
npx wrangler d1 execute kontu --local --file=fixtures.sql >/dev/null 2>&1 || true

npx wrangler dev --port 8788 >/tmp/kontu-dev.log 2>&1 &
WPID=$!
trap 'kill $WPID 2>/dev/null || true' EXIT

printf 'starting worker'
for _ in $(seq 1 60); do
  curl -sf http://localhost:8788/health >/dev/null 2>&1 && { echo " — up"; break; }
  printf '.'; sleep 1
done

cd "$ROOT/tui"
KONTU_SERVER_URL=http://localhost:8788 KONTU_API_TOKEN=devtoken cargo run "$@"
