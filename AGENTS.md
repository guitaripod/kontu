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

### A) Open-ended "find me a house" → spec + match (preferred)
1. Read the saved spec: `kontu spec --json`. If empty, or the request adds/changes
   criteria, CLARIFY (above) then save it, e.g.:
   `kontu spec set --anywhere --type omakotitalo --type mökki --price-max 100000 --shore required --privacy required --ev plus --fiber plus --owned-plot --minimize-tco --note "lakehouse, no direct neighbours, can charge a Tesla"`
2. Get ranked matches: `kontu match --pull --json` (`--pull` fetches fresh listings for
   the spec from THIS machine first; omit it to rank already-pulled data). Returns
   listings best-first with `score`, `npv_cost`, `monthly`, `risk`, and `reasons`.
3. Drill in: `kontu show <id> --json`, `kontu compare <id> <id> --json`, `kontu open <id>`.

### B) Specific query → filter directly
1. `kontu pull <municipality> [--type … --shore --price-max N]` (omit the municipality
   to pull from all of Finland).
2. `kontu list --municipality Outokumpu --price-max 120000 --shore oma_ranta --json`
3. `kontu cost <id> --horizon 20 --json`, `kontu risk <id> --json`,
   `kontu score <id> 85 --deal-breaker`, `kontu note <id> "…"`, `kontu open <id>`.

### C) Deep-verify a shortlist (the judgment layer kontu can't do)
`pull --deep` already extracts the STRUCTURED facts (shore ownership, lake-vs-river,
kuntoluokka, heating, renovation years). For the **top ~5–15** matches, fan out one
sub-agent per listing to read its real detail page (`kontu show <id> --json` for the
URL, then fetch it) and judge what a parser can't, returning a short evidence quote each:
- **fibre**: installed vs only "saatavilla/tilattu" (available/ordered) vs none;
- **shore**: cross-check `oma_ranta` against prose — is it a real home or a plot pitched
  as "mahdollisuus rakentaa ympärivuotiseen" (build-it-yourself)? lake vs river vs pond;
- **privacy**: "ei naapureita / haja-asutus" vs in-town vs on a main road;
- **condition**: the renovation SCOPE (did the peruskorjaus cover pipes/roof/electrics?).
Then rank by how many of the buyer's must-haves each CONFIRMS, and report each one's
specific catch. This is the only part of house-hunting that genuinely benefits from
multiple agents; everything else is deterministic and lives in the commands above.

## Commands
- `spec` / `spec set <flags>` / `spec clear` — show/edit the saved house-hunting spec.
  Read with `spec --json`. Flags: `--anywhere | --area <m>` (repeat), `--type <t>` (repeat),
  `--price-max N --price-min N --min-plot-m2 N --min-m2 N --min-rooms N --year-min N`,
  `--shore|--ev|--fiber|--privacy|--winterized|--condition any|plus|required|avoid`
  (`--winterized required` hard-drops summer-only mökki; `--condition required` drops
  clearly renovation-needed / unrenovated valesokkeli-era ~1960–85), `--owned-plot --require-infra
  --cash --minimize-tco --max-dom N --horizon N --exclude <kw> --note "…"`. `--cash` models a
  no-mortgage purchase (LTV 0) everywhere. The bool flags each have a `--no-…` form
  (`--no-owned-plot`, `--no-cash`, …) to clear them without `spec clear`.
- `match [--pull] [--limit N] [--scan N]` — rank listings by fit to the spec, best first
  (`score` 0–100; TCO-dominant + shore/privacy/ev/fiber/infra signals + risk; `reasons[]`
  explains each). `--pull` refreshes listings for the spec from your IP first. `monthly`
  is the opportunity-cost NPV equivalent; `monthly_living` is the year-1 running cost
  (the everyday cost of living) — the table/alerts show the latter.
- `watch [run|config|test|install|status]` — push new spec-matching listings to Telegram.
  Setup: `watch config --token <BotFather token>` → message the bot once →
  `watch config` (auto-detects the chat id) → `watch test` → `watch run --seed` (baseline,
  no alerts) → `watch install` (systemd-user timer; runs `watch run` on a schedule).
  `watch run [--no-pull] [--min-fit N] [--seed]` does one cycle: pull → match → diff against
  the baseline in `~/.config/kontu/seen.json` → alert on new. Needs a residential IP, so it
  runs on the user's machine, not the Worker.
- `pull [municipality] [--type <t>…] [--shore] [--price-max N] [--limit N] [--portal …] [--shallow]`
  — ingest REAL listings from this machine's IP into the Worker (Oikotie + Etuovi by
  default). Omit the municipality for ALL of Finland (use filters). `--shore` = lakehouses
  only. Idempotent (upserts). By default each Oikotie listing's DETAIL page is fetched and
  parsed for the real structured fields (kuntoluokka condition, shore ownership, water-body
  lake-vs-river, heating, plot ownership, energy class, roof/pipe renovation years, full
  description) — slower but far richer; `--shallow` skips it. `match --pull` and `watch`
  always enrich.
- `list [filters] [--sort C] [--desc] [--limit N] [--json]` — exact-parameter search.
  Filters: `--municipality --type <omakotitalo|paritalo|rivitalo|kerrostalo|mökki>
  --holding <kiinteisto|asunto_osake> --price-min --price-max --m2-min --rooms-min
  --year-min --shore <oma_ranta|rantaoikeus|ei_rantaa> --heating --plot <oma|vuokra>
  --max-dom --exclude <keyword> --price-dropped --text`.
  Sort C: `price|ppm2|size|year|dom|risk|score`.
- `show <id> [--json]` — full detail: every listing field + risk + cost summary +
  price/status history + your note/score/tags + cover-photo count (the TUI renders it).
- `cost <id> [overrides] [--schedule] [--json]` — local total-cost-of-ownership model
  (real-euro NPV over a horizon; principal is not a cost). Overrides: `--price --ltv
  --euribor --margin --term --horizon --discount --heating --repayment`. `--schedule`
  adds the year-by-year breakdown.
- `risk <id> [--json]` — 0–100 buyer-risk score + deferred-capex flags (valesokkeli,
  putkiremontti, salaojat, jätevesi, era/material risks).
- `compare <id> <id> ... [--json]` — side-by-side price / €m² / modelled NPV / risk.
- `score <id> <0..100> [--deal-breaker]` and `note <id> "<text>"` — personal layer.
- `open <id>` — open the listing's real source URL in the browser.
- `card <id> [--lang fi|en] [--out <path>]` — render a shareable PNG ownership
  one-pager (cover photo + price, all-in acquisition cost, monthly running cost,
  kiinteistövero, condition, what recurring obligations it avoids, modelled 20-yr
  value). Self-contained (no external tools). Defaults to `~/kontu-<muni>-<id>.png`.
- `defaults [--json]` — 2026 cost-model seed values (tax rates, Euribor, kiinteistövero).
- `market <municipality> [--json]` — area price statistics (price-fairness backbone).
- `sync` — ask the Worker to run its (IP-limited) crawl tick; mostly informational.
- `doctor [--json]` — connectivity + contract self-check.
- `guide` — print this playbook.

## Key JSON fields
- listing (`list`/`show`): `id, address, municipality, postal_code, price_eur,
  price_per_m2, living_area_m2, plot_area_m2, room_count, room_layout, year_built,
  property_type, holding_form, shore, water_body, condition_class, heating_type,
  energy_class, plot_ownership, roof_year, pipes_renovated_year, days_on_market, status,
  url, description`, plus `fairness:{band,ratio,benchmark,confidence}` (band:
  underpriced|below_market|fair|above_market|overpriced|unknown — vs the MML area median;
  `unknown` for small municipalities with suppressed data). `price_eur` null =
  price-on-request. `water_body` (jarvi|joki|meri|lampi), `condition_class`, `roof_year`,
  `pipes_renovated_year` and a full (non-teaser) `description` are populated only by a deep
  pull; `shore` distinguishes a lake from a river via `water_body` in match scoring.
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
