# kontu — master spec

Single-user terminal app to find and decide on a house to buy in Finland.
Rust + ratatui TUI ⇄ one Cloudflare Worker (D1 + R2 + Cron). Personal, non-redistributed.

Derived from two research waves + adversarial fact verification (all 2026-current).
This file is the source of truth for the build and the polish round.

---

## 1. Architecture & data planes

```
 ratatui TUI (Rust)  ──HTTPS+Bearer──▶  Cloudflare Worker (hono)
   - exact-param filter                   ├─ /api/*  (token-guarded REST)
   - side-by-side compare                 ├─ scheduled() chunked crawler
   - interactive cost model               ├─ D1  (listings, history, params, dossier, defaults)
   - inline photos (kitty/Ghostty)        └─ R2  (photos by content hash, raw snapshots)
   - open listing in browser
```

**Two strictly separated data planes — never conflate:**
- **Plane A — listings (discovery):** Oikotie `/api/cards` + Etuovi internal search API. Robots-disallowed, bot-detected, token-volatile. **Disposable by design** — the app must stay fully useful on Plane B if A breaks.
- **Plane B — valuation + geodata (the trustworthy backbone):** sanctioned open-gov APIs (Tilastokeskus StatFin, MML, SYKE, Digitransit, Traficom, OSM). Zero legal risk, CC BY 4.0.

**Operating constraints (encode, don't ignore):** single-user, low-volume, rate-limited, **normal browser User-Agent** (Etuovi robots blocks `ClaudeBot`/`anthropic-ai`), never redistribute agent personal data (GDPR household-use exemption holds only while private), keep R2 private.

---

## 2. Verified 2026 facts (the seeds — all confirmed by adversarial check)

**Transfer tax (varainsiirtovero):** kiinteistö **3.0%**, asunto-osake **1.5%**, on the **debt-free price** (sale price + assumed/taloyhtiö debt). Buyer liable. Return deadline 6 mo (kiinteistö) / 2 mo (shares). First-home exemption **abolished 1.1.2024** (still gone in 2026). Rates in force for deeds signed ≥ 12 Oct 2023.

**ASP** first-home scheme is **active and expanded 1.6.2026** (90% LTV, state interest subsidy 70% of interest above 3.8% for 10 yr, guarantee 25%/max €60k; area loan caps €230k largest cities/€160k elsewhere). Model as an optional financing mode.

**Registration fees:** lainhuuto **€172**, kaupanvahvistus **€143** (**€0** via e-conveyance Kiinteistövaihdannan palvelu), kiinnitys (mortgage deed) **€47**/deed. kuntotarkastus **€1,250–1,650**.

**kiinteistövero bands 2026:** permanent-residence building **0.41–1.00%**, general land **1.30–2.00%**, general building (incl. leisure) **0.93–2.00%**, unbuilt site **2.00–6.00%**. Base = verotusarvo (land 75% of area price, building = replacement cost − age depreciation, floor 20%), capped at käypä arvo. Typical OKT **€300–1,200/yr**.

**Mortgage 2026:** 12-mo Euribor **2.809%** (22 Jun 2026); new-loan avg **~2.84%**, margin avg **0.52%**. lainakatto **90%** (95% first-home). Term avg ~22–25 yr (statutory max being raised toward 40 yr). 12-mo Euribor resets yearly → hold rate constant within each 12-month block.

**Listings endpoints (live-verified):** Oikotie `GET https://asunnot.oikotie.fi/api/cards` returns **401** without `OTA-cuid`/`OTA-loaded`/`OTA-token` headers (harvested from page meta `cuid`/`loaded`/`api-token`, short-lived). Etuovi `POST https://www.etuovi.com/api/v3/announcements/search/listpage`, no auth, ~**800 ms**/req, backoff on 429.

---

## 3. Cost-of-ownership engine (the decision core)

**Convention (locked, asserted in tests): real, today's euros + NPV.** Mixing nominal flows with a real discount rate is the highest-impact silent bug. **Loan principal is NOT a cost** (cash→equity); only interest is.

**One-time (t=0):** down payment = price×(1−LTV); varainsiirtovero = rate(holding_form)×debt_free_price; lainhuuto €172 (kiinteistö); kaupanvahvistus €143/€0; kiinnitys €47×deeds; kuntotarkastus; loan arrangement fee; moving.

**Recurring (per year, each escalated by its own inflation; energy faster than CPI):**
| line | 2026 seed |
|---|---|
| kiinteistövero | rate_building×taxable_building + rate_land×taxable_land (bands above); €300–1,200 typical |
| kotivakuutus | €240–650 |
| heating (by `heating_type`) | maalämpö €700–1,100 · kaukolämpö €1,800–2,600 · IVLP €1,200–1,600 · öljy €2,100–4,200 · suora sähkö €3,000–6,000 |
| electricity (non-heat) | €600–1,200 (electric-heat overlaps heating) |
| water+sewer | municipal €500–1,200 · well+septic €100–300 cash + renewal reserve |
| jätehuolto | €40–600 |
| nuohous (if fireplace) | €76–150 |
| tiekunta (private road) | €100–1,500 |
| broadband | €230–960 + one-off connect |
| ground rent (vuokratontti) | from listing, indexed |
| vastike (As Oy only) | hoito + rahoitusvastike (vanishes when taloyhtiö loan repaid; toggle lump-sum velkaosuus) |
| maintenance reserve | **~1–2% of building (rebuild) value/yr**, realized as lumpy PTS projects |

**Financing — amortization (implement all three):** `tasalyhennys` (equal principal, lowest total interest), `annuiteetti` (level payment, recomputed at each rate reset), `kiintea_tasaera` (fixed instalment, term flexes). Monthly loop: `interest = balance × (euribor+margin)/12`; `principal = payment − interest`; roll balance. Hold rate constant within each 12-month block (12-mo Euribor). Annual interest sum = the cost line.

**TCO over N years (10/20/30):** t0 one-time + Σ(annual interest + recurring, escalated) + lumpy capex in scheduled years (weight heavily if `risk_structures`/inspection flagged) − resale terminal value (real growth, **rural/lakeside often flat-to-declining**; minus seller commission 2–5% incl VAT; luovutusvoittovero usually exempt if own home ≥2 yr) + **opportunity cost** of down payment + principal (discount at expected real after-tax portfolio return). **Output: total NPV + equivalent €/month + buy-vs-rent-and-invest on same basis.**

**Sensitivity sweeps (build into TUI):** Euribor 2/3/4%; repayment type × rate; discount rate 2–5% real; heating system (NPV the heat-pump switch net of €4,000 ELY grant + kotitalousvähennys); maintenance realization timing; resale price / holding period; holding_form (3% vs 1.5%).

---

## 4. Risk-scoring model (from buying-expertise; the "anything else" layer)

`RiskScore = clamp(Σ weighted flags, 0..100)` → bands 0–24 low / 25–49 moderate / 50–74 high / 75–100 severe. Separate signed **deferred-capex estimate (€)**. **Build year is the master multiplier.**

Key flags → field + capex:
- **valesokkeli** (1960–1990 wood frame) — highest-signal flag; €8k (dry) / €15–30k (moist) / €25–60k (rot). Demands rakenneavaus kuntotutkimus. → `risk_structures`.
- **asbestos** pre-1994 (haitta-ainekartoitus before reno), **kreosootti** pre-1950, lead/PCB → `build_year`.
- **putkiremontti** life 30–50 yr; €15–50k; act at 20–30 yr pipe age; insurance denies water claims past technical life → `renovation_events`/`build_year`.
- **salaojat** 30–50 yr life; €6–30k renewal; missing inspection wells → near-certain capex on 30+ yr houses.
- **roof** lifespans (huopa 20–35 / pelti 30–60 / tiili 40–100) vs age → `roof_material`+age.
- **windows/facade** cycles; **heating** conversion ROI (oil→pump payback 8–15 yr, savings €1–2.5k/yr).
- **jätevesi** haja-asutus compliance (asetus 157/2017; stricter ≤100 m from water / pohjavesialue) — non-compliance = ~€10k+ → `sewer_system`.

---

## 5. D1 schema (migrations/0001_init.sql)

Hot/filterable fields = typed columns; sparse source fields in `raw_json` TEXT (promote to indexed `json_extract` generated columns only when a new filter need appears). Tables:

- **properties** — canonical cross-portal dedup group: `fingerprint` UNIQUE = normalize(postal|street|house_no|round(m2)|rooms[|floor]); lat/lon; first/last_seen.
- **listings** — one row per portal listing. Normalized columns from the §6 parameter model (price_eur, debt_free_price_eur, size_m2, rooms, room_layout, year_built, property_type, holding_form, plot_ownership, ground_rent, shore, heating_type, energy_class, water_supply, sewer_system, broadband, road_access, city, postal_code, address, lat, lon, status …) + `raw_json` + `content_hash` + `UNIQUE(portal, portal_listing_id)`. Indexes: (city,price_eur), (status,price_eur), (property_type,city), property_id.
- **listing_events** — append-only (never UPDATE): price_change / status_change / first_seen / relisted; old/new price; observed_at. Free price-drop + days-on-market history.
- **listing_cost_inputs** — per-listing overrides of cost-engine assumptions (JSON).
- **listing_notes**, **listing_scores** (0–100 + rank), **listing_tags** — personal layer.
- **listing_photos** (R2 key = `photos/<sha256>`, position, content_type, source_url) + **seen_photo_urls** (skip re-download).
- **location_dossier** — per-property enrichment JSON (§7): distances to water/services, travel times, flood, zoning, broadband, noise, elevation.
- **market_stats** — cached Plane-B price benchmarks by municipality/postal (StatFin + MML), for price-fairness.
- **saved_searches** — named exact/range searches (params JSON, is_exact, last_run).
- **crawl_state** — per source: next_page, total_pages, cursor, status, last_tick, last_error (chunked resumable crawl).
- **source_config** — the volatile param/header maps (Oikotie buildingType codes, OTA meta names, Etuovi propertyType enums) kept in D1, **not hardcoded** (they drift).
- **cost_defaults** — the §2/§3 2026 seed values, overridable.

After index-adding migrations end with `PRAGMA optimize;`. D1 bills rows-read → index every filtered column; chunk batched INSERTs ≤100 bound params.

---

## 6. Finnish parameter model (the exact-filter taxonomy)

Groups (F = TUI filter). Enums in CHECK constraints / lookup:
1. **Identity/type:** `property_type` F (omakotitalo/paritalo/rivitalo/erillistalo/mökki/kerrostalo/maatila), **`holding_form` F** (kiinteistö 3% / asunto_osake 1.5% / määräala / hallinnanjako — switches whole cost branches), `kiinteistotunnus`, address/municipality/`postal_code` F, per-portal ids.
2. **Size/layout:** `living_area_m2` F, `total_area_m2`, `room_count` F + `room_config`, `floors` F, `basement`, `utility_room` F.
3. **Building/condition:** `build_year` F, `condition_class` F, `inspection_status`, **`risk_structures` set F** (valesokkeli, kaksoislaatta, …), `frame_material` F, `facade_material`, `roof_type`/`roof_material`, `renovation_events[]`, `windows`, `moisture_damage`.
4. **Plot:** **`plot_ownership` F** (oma/vuokra), `ground_rent_eur_yr`, `lease_end_year`, `plot_area_m2` F, `building_right`, `easements` (tieoikeus!), `outbuildings`.
5. **Water/coast (headline):** **`shore` F** (oma_ranta/rantaoikeus/ei_rantaa), `shoreline_length_m`, `shore_type`, `water_body`, `shore_sauna` F, `water_area_ownership`, `shore_zoning`.
6. **Heating/energy:** **`heating_type` F** (kaukolämpö/maalämpö/öljy/sähkö/puu/IVLP/…), `heat_distribution` (vesikiertoinen keeps maalämpö retrofit open), **`energy_class` F** (A–G + year), `e_value`, `ventilation`, `fireplace`/`masonry_heater`, `solar_pv`.
7. **Utilities:** **`water_supply` F** (kunnallinen/porakaivo/…), **`sewer_system` F** (kunnallinen/saostuskaivo/umpisäiliö/pienpuhdistamo), `main_fuse`, **`broadband` F** (valokuitu/…), `sauna` F, `parking` F.
8. **Costs:** `price_eur` F, `debt_free_price_eur`, `price_per_m2` F + stored listing-stated running costs.
9. **Legal:** `lainhuuto`, `rasitustodistus`, `zoning_status` F, `intended_use` (vakituinen/loma — affects financing, road upkeep, resale), `jatevesiselvitys`.
10. **Location:** `district`, `road_access` F (yleinen/yksityistie), views, + §7 enriched fields.

---

## 7. Geodata enrichment (Plane B; Worker background → dossier → TUI reads precomputed)

Work in EPSG:3067 (metric) for distances. Run in scheduled()/background at ingest + weekly reconcile; **never block a TUI request on live WFS/Overpass**. Pipeline:
1. Geocode — MML Pelias v2 (`/geocoding/v2/pelias/search`, cascade addresses→roads→names; fall back to portal card lat/lng).
2. Plot/building — Maastotietokanta OGC API Features (`rakennus`).
3. Elevation/slope — MML WCS DEM `korkeusmalli_2m`.
4. **Distance to water (headline)** — SYKE Ranta10 WFS nearest järvi/meri/uoma + VesiPetoDW lake attrs.
5. Distance to services — OSM Overpass `around:` (supermarket/school/pharmacy/health/town).
6. Neighbourhood — Tilastokeskus väestöruutu 1 km + Paavo postal-area WFS.
7. Travel time — Digitransit Routing v2 GraphQL (`finland` router, subscription key; car/transit/bike).
8. Flood — SYKE tulvavaaravyöhykkeet WFS (depth class + return period).
9. Zoning — Ryhti OGC API (only N/S Savo + voluntary until 2029) → **fall back to municipal KuntaGML** (needed for Outokumpu/Pohjois-Karjala).
10. Broadband — Traficom availability (fibre/≥100Mbit/≥1Gbit at address).
11. Noise — SYKE zones where they exist, else proxy = Digiroad KVL × distance to nearest road.

## 8. Price-fairness (Plane B)
- Tilastokeskus StatFin PxWeb `StatFin__ashi` (13mx municipality €/m² + counts, 13mp index) — osakeasunto stats; mind 8 Jun 2026 px-structure change + 2025=100 base.
- **MML Kiinteistökauppojen tilastopalvelu REST** (`khr.maanmittauslaitos.fi/tilastopalvelu/rest/1.1`, CC BY 4.0) — the right source for real-property (OKT kiinteistö) sold-price stats by kunta/postal/grid (median/mean/count, <3-tx suppression).
- Note `asuntojen.hintatiedot.fi` is **closed**; HTJ loan/charge data **not** an open API. Fairness verdicts are approximate → present as a band, not a number.
- Method: compare listing €/m² (size+age adjusted) vs area median, vs index trend, vs days-on-market → fairness band.

---

## 9. Worker API surface (token-guarded `/api/*`)
- `GET /health`
- `GET /api/listings?` exact multi-field filters + sort + pagination
- `GET /api/listings/:id` detail + events history + photos + dossier + cost inputs
- `GET /api/properties/:id` canonical group (portals merged)
- `POST /api/sync` trigger/inspect crawl
- `GET /api/cost-defaults`, `GET /api/market/:municipality`
- `GET/POST/PUT/DELETE /api/saved-searches`
- `PUT /api/listings/:id/{notes,score,tags,cost-inputs}`
- `GET /api/photos/:key` stream R2 (`writeHttpMetadata`, immutable cache)

## 10. TUI screens (Plane-independent; cost engine runs locally in Rust)
- **List** — sortable table (price, €/m², m2, year, type, score, days-on-market, RiskScore), live exact-param filter sidebar, fuzzy search, status colour.
- **Detail** — full param model + inline photos (kitty/Ghostty) + cost summary + RiskScore breakdown + price/status timeline + location dossier; `o` opens source in browser.
- **Compare** — side-by-side grid across price/€m²/TCO/energy/year/commute/RiskScore/score.
- **Filter** — form: every F field, ranges, exclude-keywords, max TCO, min energy, plot=oma only, max days-on-market, "price dropped since saved".
- **Cost model** — interactive: sliders for price/LTV/Euribor/margin/horizon/discount/repayment-type/heating; live amortization + NPV + €/mo + sensitivity table; per-listing overrides persisted.
- **Saved searches**, **Help overlay**, sticky footer keybar, theming.

## 11. Differentiators (none exist in Finnish portals)
TCO ranking · self-built listing history (price timeline, days-on-market, relist/withdrawn) · real **price-drop** alerts (not just new-listing) · weighted personal scoring + deal-breakers · side-by-side compare · commute filter (Digitransit) · CSV/JSON export · cross-portal dedup/merge.

## 12. Operating risks (carry into ops, not blockers)
Plane-A is robots/ToS-grey + bot-detected + schema-drifting → keep param maps in D1, snapshot raw to R2, self-test zero-result runs, treat as disposable. Worker egress IP is datacenter (higher block risk) → backoff + fallback. Lock the real-euros NPV convention in tests. Ryhti zoning gap until 2029 → KuntaGML fallback. HTJ/sold-price data not open → manual entry + approximate fairness.
