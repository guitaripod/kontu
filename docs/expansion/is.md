I now have the exact style. This is a terse, numeric, source-of-truth spec. Let me synthesize the Iceland FACTS PACK matching this register, merging the four research dimensions and applying the three fact-check corrections (0.625% not 0.6%; "1979" sourced not "mid-1979"; NTÍ deductible floor scoped to dwellings).

Here is the synthesized FACTS PACK:

# kontu — Iceland (IS) FACTS PACK

Source-of-truth for the Iceland build. Style/structure mirrors `SPEC.md`. Terse, numeric, hardcodable. Derived from four research waves (acquisition costs, recurring costs, construction-era risk, portals/geodata) + adversarial fact-check; all 2026-current. **FX: €1 ≈ 144 ISK** (live 2026-06-29; recurring-cost research used 150 — both noted inline, prefer 144 for new constants). Items the fact-check could not pin to a primary source are marked **(UNVERIFIED)** — keep them as ranges, never hardcode as precise constants.

---

## Iceland (IS) — verified facts

**Structural differences from Finland (encode these as the IS regime, do not port FI assumptions):**
- **No holding-form split.** Iceland registers detached house and apartment in the *same* land register — there is no `kiinteistö` vs `asunto_osake` distinction. Acquisition tax varies only by **buyer type** (individual vs legal entity) and **first-time status**, NOT property type. (Lög 138/2013) → IS `holding_form` is informational only; it never switches the tax branch the way FI does.
- **Heating is near-uniformly cheap geothermal** (~90% of homes on `hitaveita` district heat). Do NOT invert FI's heating-cost spread as the primary cost driver; default to cheap district heat, treat electric/oil as the rare expensive exception.
- **Radon = zero.** Basaltic bedrock; national survey mean 13 Bq/m³, max 79 — below all action levels. No radon zones, no reference level. Contribute **0** to the risk score.
- **The signature era-defect is ASR (alkalívirkni)** in concrete, not FI's valesokkeli — Iceland's housing stock is cast-in-place reinforced concrete (`steinsteypa`), not wood frame.
- **Two Iceland-only hazards absent in FI:** seismic exposure (South Iceland Seismic Zone) + mandatory catastrophe insurance (NTÍ).
- **Tax base is the official assessed value, not the price.** Both stamp duty (`stimpilgjald`) and municipal property tax (`fasteignagjöld`) are levied on **`fasteignamat`** (typically below market), so effective burden on price paid is lower than the headline rate. This is the single most load-bearing IS modelling fact.
- One authority (**HMS — Húsnæðis- og mannvirkjastofnun**) owns sold-prices + cadastre + valuation + addresses (moved from Þjóðskrá 2022/2023). Statistics: **Hagstofa**. Base geodata: **LMÍ**. Cleaner than FI's MML/Tilastokeskus split.

---

## 1. Acquisition costs (one-time)

Buyer is `rétthafi` (acquiring party). Cash individual unless noted. **All ad-valorem acquisition tax is on `fasteignamat`, NOT purchase price.**

| Line | Base | 2026 value | Source |
|---|---|---|---|
| **Stamp duty (stimpilgjald)** — individual | `fasteignamat` | **0.8%** | Lög 138/2013 §5(a) |
| stimpilgjald — **first-time residential buyer** (never previously a registered owner of `íbúðarhúsnæði` by purchase/inheritance/gift) | `fasteignamat` | **0.4%** (half) | Lög 138/2013 §5 "fyrstu kaup" |
| stimpilgjald — legal entity | `fasteignamat` | **1.6%** | Lög 138/2013 §5(b) |
| stimpilgjald — forced/foreclosure sale to lienholder | `fasteignamat` | half (0.4% indiv / 0.8% entity) | §6 |
| **Registration fee (þinglýsingargjald)** | fixed/document | **3,800 ISK ≈ €26** | Sýslumenn gjaldskrá |
| Lien/mortgage certificate (veðbókarvottorð), DD pull | fixed | **3,100 ISK ≈ €22** | id. |
| **Mortgage deed stamp/lien** | — | **0** — no percentage mortgage tax (1.5% mortgage-bond duty abolished by 2013 reform; only ownership-transfer docs bear stamp duty); financed buyer pays only the flat 3,800 ISK/doc; **cash buyer = 0** | PwC, Lög 138/2013 |
| **Building survey (ástandsskoðun)** — optional, buyer-commissioned | quote | full **95,000–125,000 ISK ≈ €650–870**; quick `hraðskoðun` ~50,000 ISK ≈ €350 | fasteignaskodun.is (incl 24% VAT) |
| **Realtor commission (söluþóknun)** | — | **0 to buyer** — seller pays the single licensed `fasteignasali` 1.7–3% + 24% VAT (baked into asking price) | agency gjaldskrár |
| **Legal/conveyancing** | optional | 150,000–400,000 ISK ≈ €1,040–2,780 **(UNVERIFIED — single secondary source)**; usually handled inside the regulated agent service, separate lawyer optional | movingtoiceland.com |

**Mechanics to hardcode:**
- Stamp-duty default = **0.8% × `fasteignamat`** (pull `Fasteignamat` from the listing detail page, §6); halve to **0.4%** when first-time flag set; **1.6%** for entity. No property-type differentiation — apartment and detached of equal `fasteignamat` pay identical duty.
- Registration is **fixed €26/doc**, NOT a percentage — some guides say "~0.1% of value"; that is a **misstatement**, use the fixed fee.
- VAT (VSK 24%) does NOT apply to the residential-property sale itself; no separate property-transfer VAT. Stamp duty is the only ad-valorem acquisition tax.
- Deadline: document must be registered (`þinglýst`) **within 2 months** of signing; late = surcharge up to **10%** of the stamp duty.
- **Worked example** (detached, `fasteignamat` 60,000,000 ISK ≈ €417k): repeat individual 0.8% = 480,000 ISK ≈ €3,330; first-time 0.4% = 240,000 ISK ≈ €1,670; company 1.6% = 960,000 ISK ≈ €6,670.

**Total buyer overhead (cash individual, non-first-time):** ≈ 0.8% of `fasteignamat` + ~3,800 ISK fixed + optional survey ~€650–870. Corroborated guide spread **~0.9–3.4% of value** (driven by first-time status + whether legal/survey bought).

Sources: althingi.is/lagas/nuna/2013138.html · taxsummaries.pwc.com/iceland/individual/other-taxes · island.is/en/registration-of-documents · island.is/s/syslumenn/gjaldskra-syslumanna · fasteignaskodun.is · fastlind.is/gjaldskra · heimili.is/gjaldskra

---

## 2. Recurring annual costs

Canonical dataset: **Byggðastofnun, *Samanburður fasteignamats og fasteignagjalda 2026*** (13 May 2026), reference detached house (`einbýlishús`) **161.1 m² / 476 m³ / 808 m² lot**, 103 valuation zones × 48 municipalities — use as defaults. (Recurring-cost research used **150 ISK/€**; EUR below at 150.)

| Line | ISK/yr | ≈ EUR/yr | Base / driver | Source |
|---|---|---|---|---|
| **Property tax (fasteignaskattur)** | 200,000–240,000 | 1,330–1,600 | `fasteignamat` × municipal A-rate (§3) | Byggðastofnun 2026 |
| Other municipal fees (lóðarleiga + vatnsgjald + fráveitugjald + sorpgjald) | 280,000–360,000 | 1,870–2,400 | mostly %-of-valuation; waste per-bin flat | id. |
| **→ Total fasteignagjöld** (all 5 line items) | **~480,000–600,000** | **~3,200–4,000** | national mean reference-house **535,600 ISK ≈ €3,570** | Byggðastofnun Tafla 3 |
| **Building insurance** (fire + NTÍ catastrophe + voluntary home) | 80,000–180,000 | 530–1,200 | `brunabótamat`; statutory levies ~0.07% of `brunabótamat` alone **(premium UNVERIFIED — individually quoted)** | NTÍ / insurers |
| **Heating — geothermal (hitaveita)** [default] | 90,000–130,000 | 600–870 | cheapest in Nordics; VAT 11% | Byggðastofnun/Orkustofnun |
| Heating — heat pump (varmadæla) | 150,000–230,000 | 1,000–1,530 | non-geothermal zones; ~14,200 kWh | (derived) |
| Heating — fired district heat (kynt fjarvarmaveita) | 250,000–350,000 | 1,670–2,330 | now below direct-electric (subsidy) | id. |
| Heating — direct electric (rafhitun) | 300,000–420,000 | 2,000–2,800 | Westfjords/East; 28,400 kWh; subsidised | id. |
| Heating — oil (olíukynding) | 500,000+ | 3,300+ | off-grid only (Grímsey-type); phased out | id. |
| **Electricity (non-heat, 4,500 kWh)** | 100,000–116,000 | 670–770 | energy ~13 ISK/kWh + distribution + 24% VAT; delivered ~25.8 ISK/kWh | Eurostat/Byggðastofnun |
| **Broadband (ljósleiðari)** | 115,000–140,000 | 770–930 | network access (Míla/Ljósleiðarinn 4,190–5,390 ISK/mo) + ISP | Sýn/Vodafone |
| Cold water (vatnsgjald) [inside fasteignagjöld] | mean 62,313 | ~415 | %-of-valuation or fixed+m²; VAT-exempt | Byggðastofnun Tafla 8 |
| Sewer (fráveitugjald) [inside fasteignagjöld] | mean 98,965 | ~660 | rate 0.055%–0.335% of valuation | Byggðastofnun Tafla 7 |
| Waste (sorpgjald) [inside fasteignagjöld] | mean 86,130 | ~575 | **per-bin flat** (≥4 sorted streams since 2023 hringrásarlög), NOT valuation-based | Byggðastofnun Tafla 9 |
| **NTÍ catastrophe levy** (always add) | 0.0375% × `brunabótamat` | — | temp +50% surcharge through 2035 (base 0.025%) | NTÍ / VÍS (see §9 risk) |
| **Off-grid** (well + septic, replaces municipal water/sewer) | 15,000–45,000 | 100–300 | **(UNVERIFIED — analogous to FI)** | — |
| Chimney sweep (sótun, if fireplace) | 10,000–25,000 | 70–170 | usually funded inside fasteignagjöld; **(UNVERIFIED — most homes wet-heated, no chimney)** | — |
| Private road (einkavegur/vegafélag) | — | 100–1,500 | Vegalög 45/1994, by-use share; **no fixed figure (UNVERIFIED, FI analogue)** | — |
| **Maintenance reserve** | 1.0–2.0% of `brunabótamat`/yr | — | 1.0–1.5% sound modern, up to 2% old/wood **(IS-specific norm UNVERIFIED — Nordic/FI calibration)** | — |

**Hardcode:** total municipal `fasteignagjöld` default ~480,000–600,000 ISK/yr (€3,200–4,000); geothermal heating default; **always add NTÍ premium = 0.000375 × `brunabótamat`** to annual cost for every property; maintenance reserve % of `brunabótamat` (not `fasteignamat`).

Sources: byggdastofnun.is/static/files/Fasteignamat/2026/fasteignagjold-2026.pdf · reykjavik.is/en/property-rates · hms.is/skyrslur/fasteignamatsskyrsla-2026 · vis.is · veitur.is · orkustofnun.is/en/natural_resources/district_heating

---

## 3. Property tax regime

`fasteignagjöld` are **entirely municipal** (no national property tax) under **Act on Municipal Revenue Sources No. 4/1995**. The bill bundles 5 items: `fasteignaskattur` (property tax), `lóðarleiga` (land rent), `fráveitugjald` (sewer), `vatnsgjald` (water), `sorpgjald` (waste).

**Property tax (fasteignaskattur) — the core levy:**
- **Base:** `fasteignamat` (combined building + land), set by HMS, effective **31 December** for the following year, with a **~10-month lag** (2026 base ≈ Feb 2025 price level).
- **Residential rate (A-flokkur / A-category):** legal max **0.5%**, municipalities may raise to **0.625%** (= 0.5% × 1.25; **statutory ceiling — use 0.625%, NOT 0.6%** [fact-check correction: one secondary source rounded sloppily]).
- **Actual 2026 rates span 0.166% → 0.625%:**
  - Lowest (high-valuation capital): **Seltjarnarnes 0.166%, Reykjavík 0.18%**, Kópavogur, Garðabær.
  - Off-capital low: Ölfus 0.200%, Vestmannaeyjar 0.225%, Reykjanesbær/Suðurnesjabær 0.230%.
  - Many rural municipalities at the **0.5% ceiling**; four at/above: **Strandabyggð & Vopnafjarðarhreppur 0.625%**, Þingeyjarsveit 0.595%, Langanesbyggð 0.575%.
- **Default for code:** capital ≈ **0.18%**, rural ≈ **0.5%** (0.625% high-rural).
- **2026 trend:** rate fell in 33/53 municipalities, rose in none, but krónur charges still +3–4% because valuations climbed **+11.7% nationally** (+15.2% Suðurnes).

**Categories (what triggers the jump):**
- **Permanent home AND private summer house (`sumarhús`) = same A-category** (up to 0.5%, max 0.625%). **No leisure-use surcharge.**
- **C-category (commercial) ~1.32% (max 1.65%)** applies ONLY if the property is **commercially rented as tourism (`ferðaþjónusta`)** — ~3–8× residential. B-category (1.32%) = public buildings (schools/hospitals), not leisure homes.

**Land rent (lóðarleiga):** only where lot is **leased from the municipality** (common in capital), % of land value (Reykjavík 0.20%). Owner-freehold rural lots pay 0. Reference-house mean ~69,632 ISK ≈ €465.

**Total reference-house fasteignagjöld 2026:** national mean **535,600 ISK ≈ €3,570**; highest zone Seltjarnarnes ~795,000 ISK ≈ €5,300; Selfoss/Egilsstaðir/Borgarnes 714,000–725,000 ISK.

Sources: althingi.is/lagas/nuna/1995004.html · byggdastofnun.is/static/files/Fasteignamat/2026/fasteignagjold-2026.pdf (p.38) · reykjavik.is/en/property-rates · gogg.is/is/stjornsysla/fjarmal-og-rekstur/fasteignaskattur · taxsummaries.pwc.com/iceland/individual/other-taxes

---

## 4. Construction-era risk flags (era → capex, for the risk model)

**Master pivot: concrete + build-year.** Three hard thresholds: **1979/1980 (ASR)**, **1976 (seismic, SISZ-only)**, **1983/1984 (asbestos)**. Radon = 0. FX €1≈144.

| Pathology | At-risk era / threshold | Clean after | Capex band (EUR) | Field | Confidence |
|---|---|---|---|---|---|
| **ASR (alkalívirkni)** — signature concrete defect | **1961–1979**, worst cohort 1968–72; **Reykjavík-capital weighted** (wetter + reactive aggregate) | **≥1980** (silica fume `kísilryk` intermilled into all IS cement from 1979; no ASR cast after) | surface treat **3,500–14,000**; **full ventilated re-clad 28,000–83,000** **(magnitude UNVERIFIED — extrapolated from multi-unit block data, no per-detached ISK rate published)** | `build_year` + `frame_material`=concrete + region | era HIGH; capex UNVERIFIED |
| **Frost + moisture + mould (raki/mygla)** — modern pan-era | **any era (NOT bounded)**; weight up basements (`kjallari`), crawl spaces, flat roofs, leak history; HMS states national remediation-expertise shortage | — | survey **700–2,000** + remediation **5,000–40,000** **(magnitude UNVERIFIED)** | moisture/leak flags | flag HIGH; capex UNVERIFIED |
| **Seismic** (SISZ/Reykjanes ONLY: Selfoss, Hveragerði, Hella, Hvolsvöllur, Þorlákshöfn) | **pre-1976** (seismic code ÍST 13 introduced 1976) | ≥1976 | residual = NTÍ **2% deductible, min ISK 400,000 ≈ €2,800/event** (dwelling only — see §9); structural varies | `build_year` + location-in-SISZ | HIGH |
| **Asbestos** | **pre-1984** (Iceland 1st in world to ban, 1983); **pre-1980 = likely** | ≥1984 | survey + abatement, regulated by Vinnueftirlitið **(cost UNVERIFIED)**; danger on disturbance (reno/demo) | `build_year` | HIGH |
| **Radon** | — none — | n/a | **0** | — | HIGH (negligible nationwide) |
| **Galvanised cold-water pipe** | **pre-~1980** galvanised | newer plastic/copper | repipe/reline (`lagnafóðrun`) **5,000–25,000 (UNVERIFIED)** | `build_year` / pipe age | era MED; capex UNVERIFIED |
| **Corrugated-iron roof (bárujárn)** | rust/paint condition, **NOT age** | maintained = 100+ yr | repaint cyclic ~7–15 yr; replace 40+ yr maintained / 25–35 yr neglected **(year bands UNVERIFIED)** | `roof_material` + paint condition | maintenance-driven |
| **Heating not on hitaveita** (electric/oil) | not geothermal-connected | geothermal = clean | conversion grant **50% of materials, cap ISK 1,496,000 ≈ €10,400** (Orkusjóður); flag = elevated annual heating cost | `heating_type` | HIGH |
| **Off-grid sewage** | non-EN-12566 / unpermitted | compliant 2-stage `rotþró + siturlögn` | upgrade **5,000–15,000 (UNVERIFIED)** | `sewer_system` | rule HIGH; capex UNVERIFIED |

**Notes for the engine:**
- **ASR pipe-renewal weak in IS:** geothermal hot water is deliberately oxygen-free (H₂S-scrubbed, pH ~9.5) → heating pipes corrode ~1 µm/yr, last unusually long. Only the **oxygen-rich cold/consumption** galvanised pipework in pre-~1980 houses is a renewal flag.
- ASR + frost act together (frost opens concrete, alkali expands it) — at-risk era × wet/freeze exposure is a **compounded** flag.
- **No MgO-board (DK) or blåbetong (SE) analogue** in Iceland — not applicable.
- **No statutory oil-heating ban** — phase-out is subsidy/grant economics, not prohibition (state subsidises electric heating transmission+distribution to 40,000 kWh/yr, law 78/2002). **(no ban date — correctly characterized; UNVERIFIED only as a negative.)**

**ASR scoring rule:** concrete house + 1961–1979 + Reykjavík area → **HIGH** (capex €28k–83k); 1955–1960 or non-capital → MEDIUM; ≥1980 → clear.

Sources: sciencedirect.com/science/article/abs/pii/S0008884698002397 · althingi.is testimony lthing=103 rnr=2241 · hms.is/fraedsla/rakaskemmdir-og-mygla · link.springer.com/article/10.1007/s10518-018-0413-x · thelancet.com PIIS2542-5196(19)30109-3 · orkustofnun.is/orkuskipti/eingreidslur · reglugerd.is/reglugerdir/allar/nr/1450-2025

---

## 5. Legal ownership-risk flags (boplikt/strandskydd/etc.)

Iceland has **no boplikt/bopælspligt residence obligation and no odel**. Three buyer-relevant gates/warnings (gates, not capex):

**(a) Foreign-buyer restriction — Act 19/1966 (am. 74/2022). THE hard gate.** Flag by **buyer nationality × residency × property type**.
- **EEA/EFTA/Faroese citizens domiciled in Iceland** (kennitala + registered residence) → buy ~like locals.
- **Non-EEA, non-resident** → need **Ministry of Justice permission**; "close-link" route = **one property, max 3.5 ha**, may own no other Icelandic property.
- **Agricultural/undeveloped "useful" land** → foreigners largely **cannot buy**; business-use exemption cap 25 ha.
- **National land-ownership cap (any nationality): 10,000 ha** without Minister-of-Agriculture authorisation (since Jul 2020).
- → A non-EEA non-resident may be legally *unable to complete*, or restricted to a single small plot. Hard gate, not a cost.

**(b) 50 m shoreline/water setback — Skipulagsreglugerð 90/2013 §5.3.2.14 (the strandskydd analogue).** Flag any **shore/lake/river** property for extension/rebuild risk.
- **No structure within 50 m** of sea/lake/river outside urban areas; shore pedestrian access must be preserved; holiday-plot boundary setback 10 m; stricter **100 m** for manure/animal houses near water sources.
- Exemption must be *applied for*, not guaranteed (nature/water-protection overlays can block). → waterfront plot may be **un-buildable/un-extendable within 50 m**; existing structures inside the line may be legally non-conforming.

**(c) Defect liability + ~10% "gallaþröskuldur" — Act 40/2002 on fasteignakaup.** Due-diligence warning, not a capex line.
- A used property is legally defective only if the defect reduces value **"significantly"** — case-law benchmark **~10% of price** (e.g. 50M ISK house → defect must cost ~5M to repair) — UNLESS seller acted culpably (concealment/false info), which removes the threshold.
- Buyer has a **strong duty of care (`aðgæsluskylda`)**. **No mandatory condition report** (ástandsskýrsla provisions never became law; 2024 reform proposing a seller questionnaire + 48-hr inspection right not yet enacted). → independent pre-purchase inspection essential; below-threshold defects are the buyer's loss.

Sources: government.is/topics/foreign-nationals/foreign-nationals-real-property-rights · globalpropertyguide.com/europe/iceland/buying-guide · island.is/reglugerdir/nr/0090-2013 · althingi.is/lagas/156a/2002040.html · island.is/samradsgatt/mal/3841

---

## 6. Listing portals (Plane A: endpoints, params, enum vocabulary)

**Single target: `fasteignir.visir.is`** — open, unauthenticated JSON search API backed by the shared agent backend (`api-beta.fasteignir.is`) every licensed `fasteignasali` pushes to → near-complete national coverage (detached, rural, summerhouse). robots permits `/api/` and `/search/`. **Skip mbl.is** (opaque hashed `q=` token + HTTP 403 to bots) and **remax.is** (RE/MAX-only subset + 403) — they add no coverage over the shared feed. Iceland is centralized: one adapter on visir covers the market (unlike SE's Hemnet+Booli).

**PRIMARY endpoint — JSON card feed (live-tested 2026-06-29):**
```
GET http://fasteignir.visir.is/api/search?onpage=1000&page=1&zip=103&stype=sale
```
Confirmed params:
- `onpage` — page size / hard cap per request (tested up to 1000)
- `page` — 1-based page index
- `zip` — single postcode per request (CSV NOT supported here)
- `stype` — `sale` (maps `sale_or_rent`→`"1"`) vs rent
- No category/price param on `/api/search` → filter client-side, OR use `/ajaxsearch/getresults` (returns HTML, accepts `zip` CSV, `price=min,max` ISK, `room=3,4`, `timespan=N` days; needs `X-Requested-With: XMLHttpRequest` + `Referer`).

**Pull strategy (mirror kontu's per-(municipality × type) fan-out):** iterate `/api/search` per `zip` × `stype=sale` with `onpage=1000`, then enrich each `id` from the detail page.

**Card JSON field shape (live, exact keys):**
```json
{
  "id": "1069437",
  "bedrooms": "0", "bathrooms": "0",       // often "0" — unreliable, use detail page
  "street_name": "Byggðarhorn 9b", "street_number": "",
  "sale_or_rent": "1",                       // "1" = sale
  "zip": { "zip": "801", "town": "Selfoss" },
  "price": "135000000",                      // ISK string; "0" = Tilboð/POA or commercial
  "size": "299,9",                           // m² string, COMMA decimal; for Lóð = plot m²
  "category": "Einbýlishús",                 // property-type enum (§8)
  "rooms": "4", "images_nr": "16",
  "image": "https://api-beta.fasteignir.is/pictures/<id>/<hash>-469x310.jpg",
  "latitude": "63.89978056",                 // WGS84 string or null (null for some lots)
  "longitude": "-21.00484821",
  "legit_realestate_agent": "Stefán Rafn Sigurmannsson",
  "openhouse": { "property": null, "date": null, ... }
}
```
Card carries **WGS84 coordinates** → no geocoding needed for most listings.

**Detail page `http://fasteignir.visir.is/property/<id>`** — adds (exact Icelandic labels):
- `Byggt` → **build year** (→ §4 era thresholds)
- `Stærð` (size m²), `Herbergi` (rooms), `Bílskúr` (garage y/n), `Útsýni` (view y/n — only structured shore proxy)
- `Fasteignamat` → **stamp-duty + property-tax base** (§1/§3)
- `Brunabótamat` → **insurance + NTÍ + maintenance-reserve base** (§2/§9)
- `Lýsing` → free-text description (**parse here for shore + heating** — no structured field)
- `Laus strax` (availability)

**NOT present structured anywhere:** heating type, energy class, water/sewer, plot size as field, floor count. Plot ("tæplega 2,5 hektara") + shore + heating live ONLY in `Lýsing` free text → **text-derived, mark lower confidence**.

**Auth / rate / bot posture / ToS:**
- visir `/api/search`: **no auth, plain HTTP, JSON.** robots disallows only `/system/ /modules/ /admin/ /agency/ /advertiser/ /cron/ /service/ /application/logs/` — `/api/` and `/search/` NOT disallowed; no `Crawl-delay`, no `Sitemap`. Green-light for single-user residential-IP. No published rate limit **(UNVERIFIED)** → self-throttle ~1 req/2–5 s, per-zip fan-out, set a normal browser UA (as kontu does for FI).
- `api-beta.fasteignir.is` is the **agent-facing WRITE API** (HTTP Basic, Companies/Properties/Units, JSON) — do NOT target; read via visir.
- mbl.is / remax.is → 403 bot-blocking, browser-only, skip.

Sources: gist.github.com/jokull/f8ba11a372db7eacfa0be06d0dad7a15 · github.com/andripp/fasteignir · api-beta.fasteignir.is/doc/request · work.iceland.is/living/house-hunting

---

## 7. Open-gov valuation & geodata (Plane B: sources, endpoints, licences)

One publisher (**HMS**) covers sold-prices + cadastre + valuation + addresses. **Risk engine pivots on IMO `ofanflóð` (avalanche/landslide) zoning, NOT flood** — that is the legally-binding hazard layer in Iceland.

| Dimension | Source / authority | Endpoint | Format | Auth | Licence |
|---|---|---|---|---|---|
| **Sold prices** (the MML-sold-price equivalent — *keystone*) | **Kaupskrá fasteigna** (HMS) | `fasteignaskra.is/gogn/grunngogn-til-nidurhals/kaupskra-fasteigna/`; CKAN `opingogn.is/dataset/kaupskra-fasteigna` | **CSV, daily** (post-midnight; was monthly pre-2023) | none (DL) | Open `is-ogl`; **no redistribution w/o agreement (distributor fee 438,500 ISK/yr — UNVERIFIED)** |
| **Price index** (Tilastokeskus-index equivalent) | **Vísitala íbúðaverðs** (Hagstofa), table **VIS01106** | `px.hagstofa.is/pxen/api/v1/en/Efnahagur/Efnahagur__visitolur__1_vnv__3_greiningarvisitolur/VIS01106.px` | **PxWeb** json/json-stat2/csv/px | none | Open (attribution) |
| **Geocoding** (Pelias/DAWA equivalent) | **Staðfangaskrá** (HMS) | bulk `skra.is/thjonusta/gogn/hra-gogn/`; mirror `github.com/flother/stadfangaskra` (WGS84 cols) | CSV/GeoJSON | none | CC BY 4.0 / OGL |
| Geocoding (REST, ID-keyed only — NOT free-text) | Já National Registers | `api.ja.is/skra/v1/` | REST JSON | **API key** | commercial |
| **Hazard zoning** (the SYKE/NVE equivalent) | **Ofanflóð** avalanche/landslide (IMO/Veðurstofa) | viewer `en.ofanflodakortasja.vedur.is`; data `vedur.is/gogn/` | viewer + SHP | none | attribution |
| **Cadastre + assessed value** (BBR/Matrikkelen equivalent) | **Fasteignaskrá / fasteignamat** (HMS) | `hms.is/fasteignaskra`; REST `developer.creditinfo.is/fasteignaskra/fasteignaskra-api` | web / REST | key (REST) | open (valuation DS); keyed (API) |
| **Base geodata / boundaries / water** (MML-basemap equivalent) | **IS 50V** (LMÍ) | WFS `ogc.gis.is/geoserver/wfs` (= `gis.lmi.is/geoserver/wfs`), WMS/WMTS; `opingogn.is` | OGC WFS 2.0.0 / WMS / WMTS + SHP | none | CC BY 4.0 |
| **Broadband** (Traficom equivalent) | Fjarskiptastofa | `fjarskiptastofa.is` (address check + stats) | web / reports | none | open stats |

**Modelling notes:**
- **Kaupskrá CSV is the single biggest lever** — free, daily, municipality+postcode+m²+year-resolved. Build the fair-price engine on it: parse → group by `(sveitarfélag, póstnúmer, tegund, byggingarár-band, fermetrar-band)` → median €/m². Fields: `kaupverð`, `útgáfudagur`, `fastanúmer`, `byggingarár`, `fermetrar`, `tegund` (residential/`sumarhús`/commercial), `póstnúmer`, `sveitarfélag`, `staðfang`. Coverage from **2006**. **(Exact CSV header strings UNVERIFIED — gated behind Vercel JS checkpoint; download once interactively to lock schema. Internal consume = fine; do NOT re-expose raw rows publicly.)**
- **VIS01106**: base **March 2000 = 100**, monthly, built from Kaupskrá. **PxWeb engine = StatFin → kontu PxWeb client reusable verbatim.** GET = metadata, POST JSON = data. **(Live GET/POST returned 400/429 to headless fetch — smoke-test the exact POST body before wiring.)**
- **Staðfangaskrá**: native CRS **ISN93 (EPSG:3057)**; flother mirror gives **WGS84 `LONG_WGS84`/`LAT_WGS84`** (no transform needed); refreshes Sundays 21:00 (mirror archived since Jan 2022 — verify freshness). Python: `iceaddr` (PyPI). Iceland has **no hosted free address→coord REST** like DAWA — geocode from the bulk register.
- **Ofanflóð = the IS risk-engine driver.** Zones **A / B / C** (C highest, constrains/prohibits residential use), legally binding under Law 49/1997 + Reg 505/2000. **Rule: property in IMO zone B or C = hard risk flag** (build/extension restrictions, forced buy-out in C). A dedicated OGC WMS/WFS for the hazard zones is **UNVERIFIED** (published via viewer + SHP); some `vedur:*` layers appear in the national geoserver.
- **No national coastal-flood / jökulhlaup property-zoning WFS exists (UNVERIFIED — none found)** — these are location-specific corridors, not per-listing layers. Use LMÍ IS 50V hydrography/coastline for **shore-distance / waterfront detection** (the `oma_ranta` analogue) — there is no strandskydd-style shoreline registry.
- **fasteignamat mechanics:** reassessed annually by HMS, owner notified each **June**, effective **Dec 31**, ~10-mo lag (Dec-31-2023 value ≈ Feb 2023 market). It is the base for municipal property tax AND stamp duty.
- **Broadband = non-differentiator** — **93.1% FTTH, >97.5% full-fibre access (end-2024)** after Ísland Ljóstengt (2016–22). Only flag genuinely-unconnected rural/summerhouse sites; don't model a broad EUR cost range like FI. A broadband-coverage open WFS/API is **UNVERIFIED** (delivered via site check, not confirmed API).
- Supporting: **apis.is** `GET /address?address=<str>` (Iceland Post) → address→postcode for fan-out/geocode fallback.

Sources: hms.is/gogn-og-maelabord/grunngogntilnidurhals/kaupskra-fasteigna · px.hagstofa.is · github.com/flother/stadfangaskra · en.ofanflodakortasja.vedur.is · ogc.gis.is/geoserver/wfs · gagnatorg.ja.is/docs/skra/v1 · fjarskiptastofa.is

---

## 8. Local enum vocabulary → kontu normalized enums

**Property type — visir `category` field (live distinct values):**
| Icelandic | English | → kontu enum |
|---|---|---|
| `Einbýlishús` | detached single-family | `detached_house` |
| `Parhús` | semi-detached / duplex | `semi_detached` |
| `Raðhús` | terraced / row house | `terraced_house` |
| `Hæð` | floor/storey unit (apartment-in-house) | `apartment` |
| `Fjölbýlishús` | apartment building (RE/MAX variants: `…með lyftu` = w/ elevator, `…með sameiginlegum inngangi` = shared entrance) | `apartment` |
| `Tví/Þrí/Fjórbýli` | two/three/four-unit house | `apartment` (or `multi_unit`) |
| `Sumarhús` (= `Sumarbústaður`) | summerhouse / holiday cottage — **the rural/lakeside category** | `cottage` / `leisure` |
| `Lóð` | building plot / land | `plot` / `land` |
| `Jörð` | farm/estate land **(UNVERIFIED as exact `category` string)** | `farm` / `land` |
| `Atvinnuhúsnæði` / `Skrifstofuhúsnæði` | commercial / office (price/rooms often `0`) | exclude (non-residential) |

**Heating — NOT structured; parse `Lýsing`. Low-signal (cheap nationwide).** Default `district_heat`.
| Icelandic | English | → kontu `heating_type` |
|---|---|---|
| `hitaveita` | geothermal **district heating** (~90% default) | `district_heat` (cheap baseline) |
| `heitt vatn` | hot water (geothermal supply) | `district_heat` |
| `rafhitun` / `rafmagnskynding` | electric heating (Westfjords/East) | `direct_electric` |
| `kynding` | generic heating | unknown |
| `olíukynding` | oil (rare/legacy) **(UNVERIFIED freq.)** | `oil` |

**Shore / waterfront — NOT a structured filter on ANY IS portal.** Parse `Lýsing`/address (text-derived → **lower confidence**, unlike FI's explicit field). Only structured proxy = detail `Útsýni` (view y/n).
| Icelandic | meaning | → kontu `shore` |
|---|---|---|
| `sjávarlóð` / `við vatnið` | waterfront/seaside plot, by the lake | `oma_ranta` (own shore) equivalent |
| `sjávarútsýni` / `Útsýni`=yes | sea view / view (NOT own-shore) | `sea_view` / `view` |
| `strönd` / `vatn` / `á` / `fljót` / `lækur` | coast / lake-water / river / stream | water-adjacent (verify) |

**Condition / holding-form / misc:**
| Icelandic | meaning | → kontu |
|---|---|---|
| `Byggt` / `Byggingarár` | build year | `build_year` (→ §4 era thresholds) |
| `Fasteignamat` | official assessed/tax value | stamp-duty + property-tax base |
| `Brunabótamat` | fire-insurance rebuild value | insurance + NTÍ + maintenance-reserve base |
| `Laus strax` | vacant/available now | availability/status |
| `Tilboð` (price `"0"`) | price-on-application | status = POA |
| `Bílskúr` | garage | parking/garage |
| **holding form** | **no kiinteistö/asunto-osake split** — single land register | `holding_form` informational only (does NOT switch tax branch); buyer-type (individual/entity) + first-time flag drive stamp duty instead |

Sources: live visir `/api/search` + `/property/<id>` (id 1069437) · talkpal.ai/vocabulary/real-estate-and-property-terms-in-icelandic · remax.is/fasteignir · islandsstofa.is

---

## Open UNVERIFIED items (leave as ranges, do NOT hardcode as precise constants)
ASR full re-clad capex €28k–83k · mould/pipe/septic remediation bands · legal-fee €1,040–2,780 · insurance premium (individually quoted) · off-grid water/sewer, chimney, private-road ISK (FI analogues) · IS maintenance-reserve % · Kaupskrá CSV exact headers · 438,500 ISK/yr distributor fee · live VIS01106 POST body · dedicated OGC WMS/WFS for ofanflóð zones & for Fjarskiptastofa broadband · any coastal-flood/jökulhlaup property-zoning WFS (none found) · `Jörð`/`olíukynding` exact portal strings.

## Three fact-check corrections applied
1. Property-tax A-category cap = **0.625%** (not 0.6% — one source rounded sloppily). §3.
2. ASR cutoff sourced as **"1979"** (not month-resolved "mid-1979"); ≥1980 = clean threshold holds regardless. §4.
3. NTÍ building deductible floor = **ISK 400,000 for a dwelling** (`húseign`); separate higher **ISK 1,000,000 floor applies to engineering structures (`mannvirki`)** — scope the 400,000 constant to dwellings only. §4/§9.
