# kontu — agent playbook

`kontu` is a CLI + TUI to find and decide on a house to buy in Finland. This file is
written for an AI agent driving the CLI on the user's behalf (e.g. the user opens
Claude Code, says "find me a lakeside house in Outokumpu under 120k", and you use
`kontu` to do it). `kontu guide` prints this file; `kontu --help` and
`kontu <cmd> --help` self-document every flag.

## Golden rules
- **Always pass `--json`.** Output is compact, parseable, and stable. The human tables
  are for the user, not for you.
- **Listing data must be pulled first.** The Worker's own crawler is bot-blocked by the
  portals' anti-datacenter measures, so YOU fetch listings from this machine with
  `kontu pull <municipality>` (idempotent; re-run to refresh). An area you haven't
  pulled is empty.
- **Cost and risk are computed locally and deterministically** in `kontu`; listing data
  comes from the Worker. Don't recompute them yourself — call `kontu cost` / `kontu risk`.

## Clarify the spec first
If the request is vague, ask the user 2–4 quick questions BEFORE searching, then encode
the answers as `list` filters and `cost`/`risk` inputs. Cover the dimensions that
actually change results:
- **Budget** — price ceiling (and, for the cost model, down payment / LTV).
- **Area(s)** — which municipalities; any commute/family anchor.
- **Type & holding** — omakotitalo / mökki / rivitalo …; kiinteistö vs asunto-osake.
- **Must-haves** — shore (oma_ranta?), min m²/rooms, year built / condition, heating, owned plot vs vuokratontti.
- **Deal-breakers** — e.g. exclude vuokratontti, exclude 1960–80s valesokkeli-era, max days-on-market.
- **Horizon & risk** — cost-model horizon (10/20/30 yr); appetite for renovation/risk.

Offer sensible defaults and proceed if the user wants speed; otherwise confirm the spec,
then run the workflow. If a spec was agreed earlier, reuse it instead of re-asking.

## Workflow (natural language → commands)
For "find a lakeside house in Outokumpu under 120k and tell me the 20-year cost":
1. `kontu pull Outokumpu` — ingest/refresh real listings for the area.
2. `kontu list --municipality Outokumpu --price-max 120000 --shore oma_ranta --sort price --json`
3. For a candidate: `kontu show <id> --json` (everything), or focused:
   `kontu cost <id> --horizon 20 --json`, `kontu risk <id> --json`.
4. Shortlist: `kontu compare <id> <id> <id> --json`.
5. Act: `kontu open <id>` (browser), `kontu score <id> 85 --deal-breaker`,
   `kontu note <id> "..."`.

## Commands
- `pull <municipality> [--price-max N] [--limit N]` — ingest REAL Oikotie listings for
  a municipality from this machine's IP into the Worker. Run before `list` for a new
  area, and to refresh. Idempotent (upserts; reports new/updated/skipped).
- `list [filters] [--sort C] [--desc] [--limit N] [--json]` — exact-parameter search.
  Filters: `--municipality --type <omakotitalo|paritalo|rivitalo|kerrostalo|mökki>
  --holding <kiinteisto|asunto_osake> --price-min --price-max --m2-min --rooms-min
  --year-min --shore <oma_ranta|rantaoikeus|ei_rantaa> --heating --plot <oma|vuokra>
  --max-dom --exclude <keyword> --price-dropped --text`.
  Sort C: `price|ppm2|size|year|dom|risk|score`.
- `show <id> [--json]` — full detail: every listing field + risk + cost summary +
  price/status history + your note/score/tags.
- `cost <id> [overrides] [--schedule] [--json]` — local total-cost-of-ownership model
  (real-euro NPV over a horizon; principal is not a cost). Overrides: `--price --ltv
  --euribor --margin --term --horizon --discount --heating --repayment`. `--schedule`
  adds the year-by-year breakdown.
- `risk <id> [--json]` — 0–100 buyer-risk score + deferred-capex flags (valesokkeli,
  putkiremontti, salaojat, jätevesi, era/material risks).
- `compare <id> <id> ... [--json]` — side-by-side price / €m² / modelled NPV / risk.
- `score <id> <0..100> [--deal-breaker]` and `note <id> "<text>"` — personal layer.
- `open <id>` — open the listing's real source URL in the browser.
- `defaults [--json]` — 2026 cost-model seed values (tax rates, Euribor, kiinteistövero).
- `market <municipality> [--json]` — area price statistics (price-fairness backbone).
- `sync` — ask the Worker to run its (IP-limited) crawl tick; mostly informational.
- `doctor [--json]` — connectivity + contract self-check.
- `guide` — print this playbook.

## Key JSON fields
- listing (`list`/`show`): `id, address, municipality, postal_code, price_eur,
  price_per_m2, living_area_m2, plot_area_m2, room_count, room_layout, year_built,
  property_type, holding_form, shore, heating_type, energy_class, plot_ownership,
  days_on_market, status, url`. `price_eur` is null = price-on-request.
- `cost`: `npv_cost, equivalent_monthly, one_time{down_payment, transfer_tax,
  lainhuuto, kaupanvahvistus, kiinnitys, inspection, moving}, total_loan_interest,
  terminal_equity, loan_principal` (+ `years[]` with `--schedule`).
- `risk`: `score, band, deferred_capex_eur, flags[{label, points, capex_eur}]`.

## Connection
Reads `~/.config/kontu/config.toml` (`server_url`, `api_token`). Override with
`--server`/`--token` or `KONTU_SERVER_URL`/`KONTU_API_TOKEN`. `kontu doctor` verifies it.

## Gotchas
- New area empty? You forgot `kontu pull <municipality>`.
- `price_eur: null` means the listing is price-on-request — not free.
- Finnish terms matter: `kiinteistö` (real property, 3% transfer tax) vs `asunto_osake`
  (housing-company shares, 1.5%); `oma_ranta` = own shore; `vuokratontti` = leased plot.
