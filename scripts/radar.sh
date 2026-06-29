#!/usr/bin/env bash
# kontu — one full cross-Nordic detection cycle. Ingests the bot-gated portals
# (SE Booli, NO Finn) and the open ones (IS visir) from THIS residential Nordic IP,
# then runs the watch cycle: Finland pull + cross-country match + diff vs seen.json
# + Telegram alert on genuinely-new gate matches + publish to the site (perfect
# matches first). Denmark is auto-crawled by the Worker cron (open API), and the
# Worker cron also runs geometric shore detection — this script covers the rest.
#
# Schedule it with the systemd-user timer in scripts/nordic/ (the residential
# machine must do the polling: the Worker's datacenter IP is bot-blocked by
# Booli/Finn). Safe to re-run: ingestion upserts, the watch only alerts on NEW.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
log() { echo "[radar $(date -Is)] $*"; }

# Only the bot-gated portals run here (the residential IP they require). The open
# APIs — Denmark (Boligsiden) and Iceland (visir) — are crawled by the Worker cron,
# which also runs geometric shore detection. scripts/nordic/visir.py exists for a
# manual deep backfill if ever needed.
log "ingesting Sweden (Booli) ..."
python3 scripts/nordic/booli.py 2>&1 | tail -3 || log "SE ingest had errors (continuing)"
log "ingesting Norway (Finn) ..."
python3 scripts/nordic/finn.py 2>&1 | tail -3 || log "NO ingest had errors (continuing)"

log "match + Telegram alerts + publish (incl. Finland pull) ..."
kontu watch run

log "cycle complete."
