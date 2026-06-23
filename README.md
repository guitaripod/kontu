# kontu

A single-user terminal app to find and decide on a house to buy in Finland — tuned
to the Finnish market (kiinteistö vs asunto-osake, ranta-tontti, energialuokka,
varainsiirtovero, kiinteistövero, lämmitysmuoto, riskirakenteet) with a proper
total-cost-of-ownership model over time and a buyer-risk score.

`kontu` is Finnish for *homestead* (and the Finnish name for Tolkien's Shire).

```
 ratatui TUI (Rust)  ──HTTPS + Bearer──▶  Cloudflare Worker (hono)
   exact-param filter                       /api/*  token-guarded REST
   side-by-side compare                     scheduled() chunked crawler
   interactive cost model                   D1  listings · history · dossier · defaults
   inline photos (Ghostty/kitty)            R2  photos by content hash
   open listing in browser
```

- **`tui/`** — Rust + ratatui terminal UI. Exact-parameter filtering, side-by-side
  comparison, an interactive total-cost-of-ownership model (live amortization + NPV
  as you adjust inputs), a buyer-risk score, inline photo previews, and "open in
  browser" to the source listing. The cost & risk engines run locally in Rust.
- **`worker/`** — Cloudflare Worker. Pulls Etuovi + Oikotie on a Cron Trigger,
  normalizes both into one parameter model, dedupes across portals, tracks price &
  status history, enriches by location (distance to water/services, broadband,
  flood), stores in D1 (+ R2 for photos), and serves the API the TUI consumes.

Everything is derived from two rounds of fact-verified research captured in
[`SPEC.md`](./SPEC.md) — the authoritative reference for the schema, the cost-engine
formulas, the 2026 seed values, the risk model, and the scraping recipes.

## Two data planes

- **Plane A — listings (discovery):** Oikotie `/api/cards` + Etuovi internal search.
  Robots-disallowed, bot-detected, schema-drifting → **disposable by design**. The
  app stays fully useful on Plane B if A breaks; volatile param/header maps live in
  the `source_config` table, not in code.
- **Plane B — valuation + geodata (the backbone):** sanctioned open-government APIs
  (Tilastokeskus, MML, SYKE, Digitransit, Traficom, OSM). Zero legal risk.

Single-user, personal, non-redistributed. Not affiliated with any listing portal.

## Quick start (local)

Prereqs: Rust, Node, and `wrangler` (already installed on this machine).

```sh
# 1. Worker: seed a local D1 with realistic fixtures and run it
cd worker
npm install
npx wrangler d1 migrations apply kontu --local
npx wrangler d1 execute kontu --local --file=seed.sql
npx wrangler d1 execute kontu --local --file=fixtures.sql
echo 'API_TOKEN=devtoken' > .dev.vars      # gitignored
npx wrangler dev --port 8788               # leave running

# 2. TUI: point it at the local worker and run it (in another shell)
cd ../tui
KONTU_SERVER_URL=http://localhost:8788 KONTU_API_TOKEN=devtoken cargo run

# Headless end-to-end check (no TTY needed):
KONTU_SERVER_URL=http://localhost:8788 KONTU_API_TOKEN=devtoken cargo run -- doctor
```

Or use the helper: `./scripts/dev.sh` starts the worker (seeded) and launches the TUI.

## Configuration

The TUI reads `~/.config/kontu/config.toml` (created on first run):

```toml
server_url = "https://kontu.<your-subdomain>.workers.dev"
api_token  = "<the API_TOKEN secret you set on the Worker>"
theme      = "default"
```

`KONTU_SERVER_URL` and `KONTU_API_TOKEN` environment variables override the file.
Logs are written to `~/.local/state/kontu/kontu.log` (a TUI can't use stdout).

## Keys

`↑↓/jk` move · `Enter` detail · `c` cost model · `/` filter · `s` sort · `space`
mark · `v` compare · `o` open in browser · `r` refresh · `y` sync · `?` help · `q` quit.
In the cost model: `↑↓` pick an input, `←→` adjust it.
In the detail screen: `1`–`5` set a personal score, `d` deal-breaker, `n` edit a note.

## CLI (agent-native)

With **no subcommand** `kontu` opens the TUI. With a subcommand it's a scriptable
CLI — every command takes `--json` for machine-readable output, and `--help`
(plus `kontu <cmd> --help`) fully documents the surface, so an LLM can discover and
drive it from natural language. Self-describe with `kontu --help`.

```sh
kontu list --municipality Outokumpu --price-max 120000 --shore oma_ranta --json
kontu show 8002 --json                       # params + risk + cost + history + notes
kontu cost 8002 --ltv 0.7 --euribor 0.03 --horizon 25 --schedule --json
kontu risk 8002 --json                       # 0–100 score + deferred-capex flags
kontu compare 8002 8007 8010 --json          # price / €m² / modelled NPV / risk
kontu score 8002 80 --deal-breaker           # personal weighted score
kontu note 8002 "Lakeside; book a kuntotutkimus."
kontu defaults | market Outokumpu | sync | doctor
```

Commands: `list · show · cost · risk · compare · score · note · sync · defaults ·
market · open · doctor`. Listings/history/photos come from the Worker; the cost and
risk models run locally in Rust. Connection comes from `~/.config/kontu/config.toml`
(or `--server`/`--token`, or `KONTU_SERVER_URL`/`KONTU_API_TOKEN`).

## Deploy (requires your Cloudflare account)

```sh
cd worker
wrangler login                                   # or export CLOUDFLARE_API_TOKEN
./../scripts/deploy.sh                            # provisions D1 + R2, migrates, deploys
```

`scripts/deploy.sh` creates the `kontu` D1 database and `kontu-photos` R2 bucket,
writes the real `database_id` into `wrangler.jsonc`, applies migrations + seed,
prompts for the `API_TOKEN` secret, and deploys. Then put the deployed URL + token
into `~/.config/kontu/config.toml`.

## Tests

- Rust: `cd tui && cargo test` (cost engine, risk model, models, API client, and
  headless render tests for every screen).
- Worker: `cd worker && npm test` (normalization, fingerprint dedup, filter SQL).
