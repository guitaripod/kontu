I have the SPEC.md style locked. Now I'll synthesize the Norway FACTS PACK, applying all fact-check corrections (329/357 levy, 28 zero, 3.3‰; two new-build dokumentavgift conditions; non-licensed tilstandsrapport author; NOK 545 flat fee; SSB 07221 sub-municipal history to 2005). The output is my final response per the harness contract.

# kontu FACTS PACK — Norway (NO)

Single source-of-truth for the Norway build. Terse, numeric, code-grade. Mirrors `SPEC.md` conventions. Derived from 6 researched dimensions + adversarial fact-check; corrected values applied, residual uncertainties marked `(UNVERIFIED)`. All NOK figures load-bearing; EUR approximate.

**FX: €1 ≈ NOK 11.1–11.3** (ECB/Norges Bank, Jun 2026; store NOK as source-of-truth, convert at a configurable rate). EUR figures below use the dimension's own rate; normalize to one rate in `cost_defaults`.

**Two-plane discipline (same as FI):** Plane A = FINN.no listings (undocumented, ToS-sensitive, disposable). Plane B = Geonorge/Kartverket/SSB/NVE/Nkom open-gov (NLOD/CC BY 4.0, zero legal risk). App must stay fully useful on Plane B if A breaks.

---

## Norway (NO) — verified facts

- **Master cost fork = `holding_form` (`eierform`)**, exactly mirroring FI's kiinteistö/asunto-osake split:
  - **`Selveier` / fast eiendom** (freehold, registered deed `skjøte`) ≈ FI `kiinteistö` → **dokumentavgift 2.5% applies**.
  - **`Andel`/borettslag** (housing co-op share) ≈ FI `asunto_osake` → **NO dokumentavgift**, carries `fellesgjeld` (share of building debt).
  - **`Aksje`/aksjeleilighet** (share-company flat) → **NO dokumentavgift**.
  - The swing is the dominant acquisition variable: a NOK 4M freehold owes NOK 100,000; the same-priced co-op flat owes ~NOK 0. On a NOK 3M Oslo flat the swing is NOK 75,000.
- **No national property tax.** Eiendomsskatt is purely municipal & optional — **28 of 357 municipalities levy none** (2026).
- **Fossil mineral-oil heating banned since 1 Jan 2020** — a still-present oil boiler is a conversion-capex + soil-contamination tail-risk flag, NOT a running fuel cost.
- **Condition-report regime since 1 Jan 2022** (avhendingslova / tryggere bolighandel): listings carry graded TG0–TG3 defect inventory with cost estimates on every TG3 — parse it directly into the risk model.
- **FINN.no is a near-monopoly portal** (~80% of listing revenue, ~98.5% of homes sold via agent, virtually all on FINN). Build Plane A around FINN alone.
- **Sold-price is semi-closed:** no open address-level transaction register (unlike FI MML). Use SSB municipality sq-metre tables (open) + Eiendom Norge regional index (open extracts); the tinglyst price register sits behind access-controlled Grunnbok-API.

---

## 1. Acquisition costs (one-time)

**Buyer rule-of-thumb: ~2.5–3.0% of price on a freehold** (dokumentavgift dominates; everything else fixed & small). Co-op ≈ near-zero transfer cost.

| Item | Amount (2026) | ≈ EUR | Base / liability / note |
|---|---|---|---|
| **Dokumentavgift** (transfer tax) | **2.5% × market value at registration** | — | Freehold (`Selveier`) only. Buyer liable (party registering deed). Base = `omsetningsverdi` at `tinglysingstidspunktet`, NOT contract price — below-market/family/debt-assumption still taxed on full market value; Kartverket can demand a valuation. **Flat 2.5%, no bands, no threshold.** Round base **down to nearest NOK 1,000**, tax **down to nearest NOK 10**, **min NOK 250**. Statens Kartverk invoices on transfer, 14-day terms. |
| → **new-build from developer** | **2.5% × LAND/tomt value only** (building exempt) | — | **TWO BINDING CONDITIONS (corrected — encode both):** (1) building **wholly new** — no reused parts, *not even the foundation*; renovations/extensions pay full rate; (2) **not yet "taken into use"** — renting it out before sale forfeits relief; >3 yrs in use → full rate. `tomteverdi` includes `vei/vann/kloakk` (road/water/sewer) dev costs. |
| → **co-op (`Andel`/`Aksje`)** | **NOK 0** | — | Not a real-property transfer (uses "overføring av hjemmel til andel", not a skjøte). |
| **Tinglysingsgebyr — deed (`skjøte`)** | **NOK 545** | ≈ €48 | **Flat, per document, all types** (electronic = paper). Set annually in statsbudsjettet — re-check yearly. Payable even when no dokumentavgift due. |
| **Tinglysingsgebyr — share transfer** (co-op andel) | **NOK 545** | ≈ €48 | Same flat fee. *(Blog figures NOK 585/440 are the stale 2020 schedule — DO NOT CODE; use 545.)* |
| **Eierskiftegebyr** (co-op ownership-change admin, to `forretningsfører`) | **NOK ~2,000–5,000** | ≈ €180–440 | NOT a tax; co-op-specific, not statutory **(UNVERIFIED national figure)**. |
| **Pantedokument** (mortgage-deed registration) | **NOK 0 (cash buyer)** — else NOK 545 | ≈ €48 | **No percentage-of-loan stamp duty** (unlike SE pantbrev 2%). Flat fee only. User buys cash → omit. |
| **Buyer inspection** | **~NOK 0** | — | Seller provides tilstandsrapport; do NOT auto-add a buyer survey line for freehold (unlike FI kuntotarkastus). |

**Seller-side (encode only if model covers resale/round-trip):**
- **Meglerprovisjon** (realtor commission, **seller pays**): 1.5–3% (avg ~2%); min NOK 30,000–50,000; fixed-price alt NOK 40,000–65,000; hourly NOK 1,700–4,000/hr. Total selling cost (commission + tilrettelegging NOK 15–25k + marketing NOK 15–30k) ≈ **3–5% of price**, rarely under NOK 100,000.
- **Tilstandsrapport** (`bygningssakkyndig`/takstmann, **seller-funded**): apartment from ~NOK 7,000; enebolig **NOK 8,000–15,000** typical, up to NOK 18,000–25,000 large.

**Worked (used freehold):** NOK 4,000,000 → dokumentavgift NOK 100,000 + skjøte NOK 545 = **NOK 100,545** (≈ €8,900). NOK 5,000,000 → **NOK 125,545** (≈ €11,100).

**Exemptions (dokumentavgift):** spouse transfers during marriage; surviving spouse on death; legal heirs on inheritance (but *forskudd på arv* / advance-on-inheritance NOT exempt); qualifying cohabitant split. Source: [Kartverket fritak](https://www.kartverket.no/en/property/dokumentavgift-og-gebyr/fritak-for-dokumentavgift).

Sources: [Kartverket dokumentavgift](https://www.kartverket.no/en/property/dokumentavgift-og-gebyr/dokumentavgift-ved-overforing-av-fast-eigedom) · [Kartverket tinglysingsgebyr 2026](https://www.kartverket.no/en/property/dokumentavgift-og-gebyr/tinglysingsgebyr) · [Skatteetaten document tax](https://www.skatteetaten.no/en/business-and-organisation/vat-and-duties/excise-duties/about-the-excise-duties/document-tax/) · [DNB nybygg dokumentavgift](https://dnbeiendom.no/altombolig/nybygg/lavere-dokumentavgift-i-nybygg) · [NEF dokumentavgift nybygg](https://nef.no/fagstoff/dokumentavgift-salg-nybygg/) · [Kartverket borettslag](https://www.kartverket.no/en/property/borettslag/transfer-of-unit-in-a-housing-cooperative).

---

## 2. Recurring annual costs

Per year, each escalated by its own inflation (energy faster than CPI). Store NOK; default detached house ~150 m².

| Line | NOK/yr | ≈ EUR/yr | Notes (model branch) |
|---|---|---|---|
| **Eiendomsskatt** | 0 – ~12,000 (avg ~3,867) | 0 – ~1,060 (avg ~340) | **Default 0 unless municipality levies** (28/357 levy none). See §3 for formula. |
| **Husforsikring** (building insurance) | 9,000–12,000 (spread 5,000–40,000) | 800–1,060 | Scale by rebuild value; **coastal/Vestlandet ≈ 2× Østlandet**; old 1960s steel pipes / no earth-fault breaker +30–50%. ~NOK 42/m²/yr rule. |
| **Heating — air-air HP** (default, 85% installs) | 3,000–6,000 | 265–530 | SCOP 4.0–5.5; cuts heating kWh 30–60%. No Enova subsidy. |
| **Heating — ground-source** (bergvarme) | 9,000–18,000 | 800–1,600 | Best ≥30,000 kWh/yr homes; install NOK 200–300k, Enova ≤ ~NOK 55,000. |
| **Heating — district** (fjernvarme) | 15,000–28,000 | 1,330–2,480 | **Price-capped at electric-heating cost by energiloven.** Norgespris-for-fjernvarme available. |
| **Heating — direct electric** (panelovner) | 25,000–35,000 | 2,210–3,100 | ~1.50 NOK/kWh × 18–25k kWh; Norgespris materially lowers. |
| **Heating — wood** (vedfyring, primary) | 8,000–15,000 | 710–1,330 | Bulk birch ~1 NOK/kWh; small sacks 1.68–3.37 (old stove ~40% eff → 3× cost). |
| **Heating — oil** | **N/A — BANNED 2020** | — | See §4; model as conversion-capex flag, not a fuel line. |
| **Electricity (non-heat)** | 5,000–12,000 | 440–1,060 | ~4,000–8,000 kWh; lower in North or under Norgespris; EV adds. |
| **Kommunale avgifter** (water+sewer+waste+sweep, `selvkost`) | 14,000–20,000 (avg ~18,029) | 1,240–1,770 | SSB std 120 m²/1 occupant. Spread NOK ~11,500 (Stavanger) → ~28,000 (Skiptvet). **Water+sewer +24% projected 2025→2029**; VAT on water/sewer cut 25%→15% from 1 Jul 2025. |
| → off-grid **water** (egen brønn) | 1,000–3,000 reserve | 90–265 | Removes municipal water fee; adds pump/testing/re-drill risk. |
| → off-grid **sewer — septic** (slamavskiller) | 500–1,800 | 45–160 | Emptying NOK 1,800–3,800 every 2–4 yr + municipal slamgebyr. |
| → off-grid **sewer — minirenseanlegg** | 5,000–10,000 | 440–885 | Electricity + mandatory service contract (NOK 3,500–8,000/yr). |
| **Chimney sweep** (feieavgift) | ~300–700 | 27–62 | Only if chimney/wood stove; bundled inside kommunale avgifter **(UNVERIFIED standalone national avg)**. |
| **Privat vei** (veilag share) | 3,000–7,000 | 265–620 | `veglova §54` usage-split. Underlying NOK 40–90k/km/yr; snow-driven (coast 10–15 vs inland 25–35 ploughings). |
| **Bredbånd** (fiber) | 6,000–12,000 | 530–1,060 | Intro NOK 500–900/mo → post-campaign NOK 900–1,300/mo. Rural trenching NOK 3,000–15,000 one-off. ~83% fiber coverage. |
| **Festeavgift** (ground rent, if `festet`/leased plot) | from listing | — | Indexed; `Eid` vs `Festet tomt` in detail page. |
| **Felleskostnader** (co-op only) | from listing | — | Monthly common cost; incl. fellesgjeld servicing. |
| **Maintenance reserve** | 25,000–48,000 (~1.5–2% rebuild value) | 2,210–4,250 | Norm 1–3% rebuild/yr; measured avg ~NOK 47,800. **Suppress first ~10 yr on new builds** (~50% of used-home need by age 15). |

**Country-specific cost flags:**
1. **North–South electricity split** (NO3/NO4 cheap ~24 øre/kWh spot 2025; NO1/NO2/NO5 dear ~96 øre) — heating cost must be **price-area-aware**.
2. **Norgespris**: opt-in fixed **50 øre/kWh power-component** incl. VAT (≈ €0.044), cap 5,000 kWh/mo, **1 Oct 2025 → 31 Dec 2026** (power+fjernvarme); grid (nettleie) + taxes on top. Strømstøtte remains the alternative.

Sources: [SSB eiendomsskatt](https://www.ssb.no/offentlig-sektor/kommunale-finanser/statistikk/eiendomsskatt) · [SSB table 12842](https://www.ssb.no/statbank/table/12842) · [Huseierne kommunale avgifter](https://www.huseierne.no/nyheter/kommunale-avgifter-2024/) · [regjeringen Norgespris](https://www.regjeringen.no/no/tema/energi/strom/sporsmal-og-svar-om-norgespris/id3089310/) · [SSB elprisstatistikk](https://www.ssb.no/energi-og-industri/energi/statistikk/elektrisitetspriser) · [Krogsveen bokostnader](https://www.krogsveen.no/magasin/hva-koster-det-a-bo-her).

---

## 3. Property tax regime

**Eiendomsskatt** — `eigedomsskattelova`. **Purely municipal & optional**; no national property tax. Kommunestyre decides annually (levy? rate? allowance?).

**Coverage (corrected to 2026 SSB):** **329 of 357 municipalities levy; 28 levy none.** 229 (2025) applied it municipality-wide. → **Default to NOK 0 unless municipality is on the levying list** (pull live at ingest).

**Formula:** `tax = (value × 0.70 − bunnfradrag) × rate‰`
- **Mandatory reduction factor 30%** → base ≤ 70% of value.
- **Rate band homes/holiday homes: 1–4‰** (cap cut 7→5‰ 2020, 5→4‰ 2021). First year capped 1‰; ±1‰/yr max (2‰ the year a bunnfradrag is introduced). Commercial cap 7‰.
- **Average rate actually applied (SSB 2026): 3.3‰** (corrected from 2025's 3.2‰) — **use 3.3‰ as model default where levied.**
- **Bunnfradrag** (basic allowance, optional, per dwelling unit): Oslo NOK 4.9M @ **1.7‰** (below ~NOK 7.25M value pays 0); Bergen NOK 750,000 @ 2.6‰; Trondheim NOK 700,000 @ 2.65‰ (own appraisal); Stavanger no allowance @ 1.0‰ (own appraisal).
- **Value source (2 methods):** (a) Tax Administration calculated market value (`formuesverdi`-derived, ~110 municipalities), or (b) `kommunal takst` (own appraisal — Stavanger/Trondheim). **2-year lag** (2026 bill uses 2024 tax-return value). New national valuation model affects eiendomsskatt only from **2028**.
- **Holiday homes (`fritidsbolig`):** same 4‰ cap + 30% factor, but usually valued by **municipal appraisal × 0.70**; some hytte-heavy municipalities set higher rate / no allowance → a NOK 3M cabin can owe more than a NOK 3M permanent home next door.

**Typical bill (SSB 2025): NOK 3,867 (≈ €340) avg where levied.** Worked: NOK 4M dwelling, 2.8‰, NOK 1M allowance → `(4M×0.70 − 1M)×0.0028 = NOK 5,040`. Model range where levied: **NOK 0 – ~12,000/yr**.

Sources: [Skatteetaten property tax](https://www.skatteetaten.no/en/person/taxes/get-the-taxes-right/property-and-belongings/houses-property-and-plots-of-land/property-tax/) · [SSB eiendomsskatt](https://www.ssb.no/offentlig-sektor/kommunale-finanser/statistikk/eiendomsskatt) · [Huseierforbundet](https://www.huseierforbundet.no/eiendomsskatt-pa-bolig) · [E24 eiendomsskatt](https://e24.no/naeringsliv/i/aJv45L/saa-mye-oeker-eiendomsskatten).

---

## 4. Construction-era risk flags (era → capex, for the risk model)

`RiskScore = clamp(Σ weighted flags, 0..100)` + separate signed deferred-capex (NOK). **Build year is the master multiplier.** No single dominant era-scandal (unlike FI valesokkeli); signal is distributed. EUR ≈ NOK/11.1.

| Flag | At-risk threshold | Capex NOK (≈ EUR) | → field |
|---|---|---|---|
| **Wood-frame moisture/rot band** (NO valesokkeli analogue) | **build 1970–1995** | bunnsvill/rot repair 50,000–250,000 (4.5k–22.5k); full sill+drainage 300k–600k+ (UNVERIFIED single fig) | `risk_structures` / `build_year` |
| **No ground moisture barrier** (fuktsperre) | **build < 1980** + slab/basement | folds into drainage | `build_year` |
| **"Multimur" sandwich foundation** (frame+PU foam+gypsum, external cast) | **1980–1995** | (UNVERIFIED) | `risk_structures` |
| **Render-on-insulation façade** (ETICS/EPS, NO enstegstätad analogue) | **~2000–2010**, or any era w/ no ventilated cavity | 150,000–500,000+ (13.5k–45k) (UNVERIFIED) | `facade_material` |
| **Coastal wood-rot exposure** | regional multiplier (Vestland/Nordland/west coast) | scales §wood-frame | region flag |
| **Asbestos** (eternit etc.) | **build ≤ 1985** (≤1980 sharper); banned 1979 products / 1985 full | survey + licensed removal; eternit roof 100,000–250,000+ (UNVERIFIED) | `build_year` |
| **PCB** (fugemasse 1960–78, sealed-glass units 1965–80, fluorescent capacitors) | **build 1960–1980** | survey + hazmat disposal (≥50 mg/kg = hazardous waste) | `build_year` |
| **Chlorinated paraffins** (window glue/gaskets) | **windows 1975–1990** | hazmat disposal | `windows`/`build_year` |
| **Creosote** (kreosot, PAH) | dominant **pre-1940**; new private use banned (pre-2002 restricted) | hazmat disposal; never burn | `build_year` |
| **Radon** | geology (DSA `høy/særlig høy aktsomhet`); action 100 / limit 200 Bq/m³ | mitigation 15,000–60,000 (1.4k–5.4k) (UNVERIFIED) | geocode → radon class |
| **Drainage** (drenering ≈ salaojat) | life **30–50 yr**; gravel/fabric 20–30 yr; pre-1980 untouched → camera-inspect | **150,000–400,000** (13.5k–36k); large 80–100 lm to 450k; >100 lm/hard ground >600k | `build_year`/`renovation_events` |
| **Pipe renewal** (rørfornying ≈ putkiremontti) | act **~30–50 yr** pipe age (copper/galvanised supply, cast-iron drains) | relining **80,000–288,000** (~2–5k/m); full rørbytte **150,000–500,000** (1.5–2.5× relining) | `renovation_events` |
| **Roof** (taktekking) | felt ~25 yr / concrete-clay tile 30–50 (to 75) yr / sheet-metal 30–50 yr; **underlay/battens drive timing** | scale to roof area | `roof_material`+age |
| **Oil heating** (oljefyr) | **fossil oil banned 1 Jan 2020**; tank present = liability | air-water HP 100,000–150,000 (Enova ≤20k); tank removal 10,000–30,000; **soil remediation 40,000 → millions** (owner liable, forurensningsloven §7; insurance often excludes) | `heating_type` |
| **Off-grid sewage** (spredt avløp ≈ jätevesi) | mains-off + permit >15 yr, or septic-only (old systems clean 5–15% vs req 90% P + 90% BOF5) | upgrade 150,000–400,000+ (UNVERIFIED); **forced mains connection → into the millions** for remote (2026 Statsforvalter struck down municipal cost caps) | `sewer_system` |
| **Legal** (boplikt/konsesjon/odel) | see §5 | forced-sale risk, not capex | §5 fields |

**Latent tail-risks to weight heavily (asymmetric):** buried oil-tank soil remediation; forced municipal sewer connection; off-grid plant >30-yr upgrade mandate.

**TG-regime advantage (parse directly):** since 1 Jan 2022 condition reports grade each building part **TG0–TG3** (NS 3600). **TG2** = action within years; **TG3** = serious, immediate, *must carry a cost estimate* (`kostnadsanslag`). Documented items → buyer's risk; undocumented hidden defects actionable above **~NOK 10,000**. **CORRECTION:** the bygningssakkyndig is **NOT a licensed/authorized profession** (autorisasjonsordning proposal was dropped) — "anyone" following procedure can write it → **calibrate report-trust down**; weight **TG3-with-estimate** and **TGiU (not inspected) on moisture-critical parts** (crawlspace, drainage, wet rooms, roof underlay) most heavily; use era thresholds above as the prior when TG absent.

Sources: [Huseierne fukt/råte 1970–1995](https://www.huseierne.no/vedlikeholdskalender/mai/slik-unngar-du-fukt-og-rate-i-din-bolig-bygget-mellom-1970-og-1995/) · [SINTEF/Murkatalogen P5](https://www.handverksmur.no/images/marketing/murkatalogen/p5_puss_p__isolasjon.pdf) · [DSA radon](https://www.dsa.no/radon) · [Miljødirektoratet PCB](https://www.miljodirektoratet.no/ansvarsomrader/kjemikalier/den-norske-prioritetslista/flammehemmere/polyklorerte-bifenyler-pcb/) · [Miljødirektoratet oljetanker](https://www.miljodirektoratet.no/ansvarsomrader/forurensning/forurenset-grunn/nedgravne-oljetankar/nedgravde-oljetanker/) · [regjeringen oljefyringsforbud](https://www.regjeringen.no/no/aktuelt/forbud-mot-bruk-av-mineralolje-til-oppvarming-av-bygninger-fra-2020-vedtatt/id2606491/) · [Lovdata forskrift 2018-1060](https://lovdata.no/dokument/SF/forskrift/2018-06-28-1060) · [Oppussingsguiden drenering](https://www.oppussingsguiden.no/pris/kjeller/koster-drenering-rundt-hus/) · [VVSTrygg rørfornying](https://www.vvstrygg.no/2025/11/07/rorfornying-pris-2025/) · [DiBK tak](https://www.dibk.no/smartere-oppussing/raad/tak/) · [DiBK forskrift til avhendingslova](https://www.dibk.no/regelverk/forskrift-til-avhendingslova-tryggere-bolighandel) · [Lovdata forskrift 2021-1850](https://lovdata.no/dokument/SF/forskrift/2021-06-08-1850/KAPITTEL_2).

---

## 5. Legal ownership-risk flags (boplikt/strandskydd/etc.)

Must surface to buyer; mostly forced-sale risk, not capex. (FI has no analogue — NO-specific.)

- **Konsesjon** (acquisition concession): most ordinary dwellings are **concession-free** if total area < **100 dekar** AND cultivated land < **35 dekar**. Buyer files `egenerklæring om konsesjonsfrihet` — **without registered concession status the deed cannot be tinglyst.** → `konsesjon_status`.
- **Boplikt** (residence obligation, 3 variants — all buyer-critical):
  1. **Nedsatt konsesjonsgrense / "nullgrense"** (municipal zero-limit): in designated municipalities **all built property > 0 dekar is concession-liable unless used as a year-round home**. Avoided by year-round use; met while *someone is folkeregistrert resident* (renting often allowed). **No time limit — entire ownership period.** *This is the flag that bites cabin/second-home buyers.* List at Landbruksdirektoratet → `nullgrense_kommune` flag.
  2. **Lovbestemt (statutory) boplikt** on larger farms (> 35 dekar cultivated **or** > 500 dekar productive forest) taken by odel/close family: **personal** — move in within **1 yr, live 5 consecutive yrs, cannot rent out.**
  3. **Boplikt as concession condition** on open-market buys (municipality sets personal/impersonal).
  - **Sanction:** breach → ordered to apply for concession; refusal → municipality sets sale deadline, can force **tvangssalg**. Buyer bears assessment responsibility; false declaration = serious.
- **Odel** (allodial right): relatives in the odel line have a statutory **redemption right (odelsløsning)** — a buyer of an odel-burdened agricultural property can be forced to surrender it. **Flag any `landbrukseiendom`/`Gårdsbruk` for both odel and boplikt.** → `odel_exposure`.
- **Shore / 100-metersbeltet:** Norway has **no fixed national strandsone WFS / shore-building-ban API** like SE strandskydd. The rule is the **byggeforbud i 100-metersbeltet langs sjø** under plan- og bygningsloven, administered **per-municipality**. Model shore-build-restriction via geocoded distance-to-water (NVE/Kartverket layers), not a sanctioned protection-zone dataset **(UNVERIFIED that a unified national layer exists)**. Text terms: `strandsone`, `strandtomt`, `sjøtomt`.

Sources: [Landbruksdirektoratet konsesjon](https://www.landbruksdirektoratet.no/nb/eiendom/konsesjon-paa-eiendom) · [Regjeringen konsesjon/boplikt](https://www.regjeringen.no/no/tema/mat-fiske-og-landbruk/landbrukseiendommer/innsikt/konsesjon/id2482552/) · [NEF boplikt](https://nef.no/fagstoff/boplikt-juristen-svarer/) · [Statsforvalteren landbrukseiendommer/boplikt](https://www.statsforvalteren.no/portal/landbruk-og-mat/landbrukseiendommer-og-boplikt/).

---

## 6. Listing portals (Plane A: endpoints, params, enum vocabulary)

**Source = FINN.no (Schibsted), near-monopoly.** Verticals: `homes` (houses+apartments), `leisuresale` (cabins — the lakeside lane), `leisureplots`, `newbuildings`; rural via `homes` + `property_type=Gårdsbruk/Småbruk`. Secondaries (cross-check/dedup only, no API): Hjem.no, DNB Eiendom, EiendomsMegler 1, Krogsveen, PrivatMegleren — all push to FINN.

**Official REST API = DEAD END** for a private buyer: `https://cache.api.finn.no/iad/search/realestate-homes`, header `x-FINN-apikey`, requires business relationship + `orgId` + data-ownership; returns Atom/OpenSearch XML. `api.finn.no/iad/` reachable only inside FINN's network. → do not use.

**Actual residential-IP path = website hydration** (mirrors FI Oikotie `/api/cards`). FINN is a Next.js app; search pages server-rendered, same query params drive an internal JSON search service.
- **Search URLs (robots-allowed):**
  - `https://www.finn.no/realestate/homes/search.html`
  - `https://www.finn.no/realestate/leisuresale/search.html`
  - `https://www.finn.no/realestate/leisureplots/search.html` · `…/newbuildings/search.html`
  - **Detail:** `https://www.finn.no/realestate/homes/ad.html?finnkode=<ID>` (and `…/leisuresale/ad.html`)
- **Extraction:** parse embedded `__NEXT_DATA__` / `pageProps`, or replicate the XHR to FINN's internal search API; returns price, area, rooms, energy label, **GPS coords**. **Do not hardcode the internal route** (it drifts); request `…/search.html?<params>`, read `__NEXT_DATA__`, paginate `&page=N` (1-based).

**Query params** (OpenSearch-ext; verified on live URLs):

| Purpose | Param | Notes |
|---|---|---|
| Location | `location` | **FINN-internal dotted taxonomy, NOT kommunenummer.** Region/fylke = `0.<id>` (Oslo `0.20061`, Innlandet `0.22034`, Vestland `0.22046`, Telemark `0.20009`, Trøndelag `0.20016`, Nordland `0.20018`); municipality = `1.<fylke>.<kommune>` (Stavanger `1.20012.20196` — **partially UNVERIFIED** kommune mapping). Repeatable for multi-select. Resolve from OpenSearch description doc / site inspection into a refreshable lookup. |
| Price | `price_from` / `price_to` | NOK (prisantydning) |
| Living area | `area_from` / `area_to` | m² |
| Build year | `year_from` / `year_to` | |
| Bedrooms | `no_of_bedrooms` | param name **UNVERIFIED** — confirm via URL inspection |
| Property type | `property_type` | **numeric code** (enums §8) |
| Ownership form | `ownership_type` | **numeric code** (e.g. live `ownership_type=3`) |
| Facilities incl. **shore** | `facilities` | **numeric code**; **`Strandlinje` (shoreline) is a real facilities option** = the waterfront filter |
| Sort | `sort` | `sort=2` price asc etc. |
| Pagination | `page` (1-based), `rows` | official API caps `rows≤1000`, `page≤50` |

**Critical gotcha:** `property_type`/`ownership_type`/`facilities` take **numeric codes, not words**, and the label→code map is **unpublished + drifts**. Discover by inspecting live filter URLs / empty-search OpenSearch facets; store as a **refreshable D1 lookup (`source_config`), not constants** — exactly like FI Oikotie buildingType codes.

**Card fields:** `finnkode` (numeric ad ID = join key) · address (street+municipality+postcode) · `price` (Prisantydning) · `totalpris` · area m² (BRA/**P-rom**) · room/bedroom count · `ownership_type` · `property_type` · `fellesgjeld` · `felleskostnader` · thumbnail · `Visning` (viewing date) · **lat/lng**.
**Detail page adds:** areas (`BRA`, `P-rom`, `BRA-i` internal, `BRA-e` external, `Tomteareal`, `Eid`/`Festet tomt`); `Byggeår`; `Energimerking`; lat/lng; `Prisantydning`/`Totalpris`/`Omkostninger` (incl. dokumentavgift 2.5%)/`Formuesverdi`/`Kommunale avgifter`/`Eiendomsskatt`/`Festeavgift`. **Heating, water/sewer, shore are free-text** (`Fasiliteter` + description) — **no first-class `heating_type`/`water_supply` field**; parse from text. `Strandlinje`/`Peis`/`Lademulighet` are facilities flags.

**robots / bot posture:** [robots.txt](https://www.finn.no/robots.txt) does **NOT** disallow `…/search.html` or `…/ad.html` (only `preview.html`, `newbuildings/ad.html`, `/pf-api/`, `/distribution`, `.ics`); **no Crawl-delay**. Schibsted runs Akamai-class bot management → **normal browser User-Agent, few req/sec with jitter, residential IP, no datacenter IP, no login needed** for public for-sale listings. ToS-sensitive (gated to owned data) — read-only personal house-hunt only, never redistribute (same posture as FI Oikotie/Etuovi). Per-rate throttle **UNVERIFIED**. **Fan out per (municipality × vertical × property_type)** — same as FI.

**UNVERIFIED to confirm via live DevTools before coding:** exact internal search route; numeric codes for `property_type`/`ownership_type`/`facilities` (incl. `Strandlinje`); `no_of_bedrooms` param name; `1.x.y` municipality mappings; throttle.

Sources: [FINN API getting-started](https://www.finn.no/api/getting-started) · [FINN search doc](https://www.finn.no/api/doc/search) · [FINN robots.txt](https://www.finn.no/robots.txt) · [PropAPIS FINN fields](https://www.propapis.com/platforms/europe/finn) · [Apify FINN scraper](https://apify.com/logiover/finn-no-scraper) · [lifestyle client](https://github.com/finn-no/lifestyle) · [NVE energy labelling](https://www.nve.no/energy-consumption-and-efficiency/energy-labelling-of-housing-and-buildings/).

---

## 7. Open-gov valuation & geodata (Plane B: sources, endpoints, licences)

Backbone = **Geonorge/Kartverket + SSB + NVE + Nkom**. Almost all **NLOD 1.0** (`http://data.norge.no/nlod/no/1.0`) or **CC BY 4.0** — attribution-only, free commercial, no auth. **Structural difference from FI: person-linked property data + sold prices are NOT open** (Matrikkel/Grunnbok SOAP, application-only, businesses-only) — treat sold-price as semi-closed.

| Dimension | Source | Endpoint | Format | Auth | Licence |
|---|---|---|---|---|---|
| **HPI (official)** | SSB **tbl 07221** "Prisindeks for brukte boliger", 2015=100, **1992K1–**, quarterly, **hedonic**, ~13 days post-quarter | `https://data.ssb.no/api/v0/en/table/07221` | REST POST JSON-stat2 / v2-beta GET | none | NLOD / CC BY 4.0 |
| **Sold-price sq-m by municipality (open)** | SSB **tbl 06035/06696** (borettslag), **13500** (new detached), **07241** (borettslag quarterly, discontinued ~2024K4 *(UNVERIFIED end quarter)*) | `https://data.ssb.no/api/v0/...` | REST JSON-stat2/CSV | none | NLOD / CC BY 4.0 |
| **New-dwelling HPI** | SSB **tbl 07230** | `data.ssb.no/api/v0/...` | REST | none | NLOD |
| **Sold-price register (tinglyst, CLOSED)** | Kartverket **Grunnbok-API** | `https://www.matrikkel.no` / `https://nd.matrikkel.no` (SOAP) | SOAP | **agreement (utleveringsforskriften), businesses only; private individuals cannot get access; no marketing use** | restricted |
| **Industry resale index** | Eiendom Norge + Eiendomsverdi (SPAR/producer) + Finn.no; monthly (3rd biz day 11:00), 7 regions, history to 2003, ~70% of source transactions, ~80–90% brokered secondary market | [eiendomnorge.no](https://eiendomnorge.no/boligprisstatistikk/) | reports | partial | extract free w/ attribution |
| **Geocoding (fwd/rev)** | Kartverket **Adresse-API** (reads Matrikkelen-Adresse) | `https://ws.geonorge.no/adresser/v1/sok` (forward), `…/punktsok` (reverse) | REST JSON | **none** (max **10,000 items/query**) | NLOD 1.0 |
| Place-name geocoding | Stadnamn-API | `https://ws.geonorge.no/stedsnavn/v1/` | REST | none | NLOD |
| **Flood hazard zones** (high-signal) | NVE **Flomsoner** — ~140 river stretches, **20/200/1000-yr + 200-yr/2100 climate**, TEK17 classes | `https://gis3.nve.no/map/services/Flomsoner/MapServer/` + Geonorge WMS/WFS (UUID `fc5f7878-8696-47f3-a9a7-d8bf51068203`) | WMS/WFS/ArcGIS REST | none | NLOD |
| **Flood awareness** (national) | NVE **Flom aktsomhetsområder** — coarse, **20 m buffer** (2025), no return period | Geonorge UUID `60c5024f-bf93-4d7a-888a-5fe001427195` / `gis3.nve.no/...` | WMS/WFS | none | NLOD |
| **Storm surge / sea level** (coastal) | Kartverket **Se havnivå / Stormflo** — today + 2100/2150, 20/200/1000-yr, DSB 2024 guidelines | `https://stormflo-konsekvens.kartverket.no/` + WMS (UUID `1de88bc6-ecba-4e1b-a4f0-e9551eb1f2bd`) | REST/WMS | none | CC BY 4.0 / NLOD |
| Quick-clay (kvikkleire) hazard | NVE | `gis3.nve.no/map/services/` / Geonorge | WMS/WFS | none | NLOD |
| **Broadband** | **Nkom** dekningskart + per-address CSV (`Alle_teknologier`, `Kablet`, `Erbolig`=1 if year-round bldg <160, `Erfritid`=1 codes 161–163) | `https://api.nkom.no/dataplattform/odata/` (`$filter`, `format=csv`) + `https://dekningskart.nkom.no/` | OData/CSV | none | NLOD |
| **Building/cadastre (open)** | **Matrikkelen** via Geonorge — parcels, addresses, building **points/footprints** | `https://nedlasting.geonorge.no/api/` + `https://wms.geonorge.no/skwms1/wms.<layer>` / `wfs.geonorge.no/skwms1/wfs.<layer>` | REST/WMS/WFS | none | NLOD 1.0 |
| **Building/cadastre (person-linked, CLOSED)** | **MatrikkelAPI** (owner name/DOB, encumbrances, full building attrs incl. **byggeår/floor area/heating**) | `https://www.matrikkel.no` / `https://nd.matrikkel.no` (SOAP) | SOAP | **agreement, businesses only** | restricted |

**SSB access (PxWebApi):** v1 POST JSON body `https://data.ssb.no/api/v0/en/table/<id>` (`Region`/`Boligtype`/`ContentsCode`/`Tid` dims, `"response":{"format":"json-stat2"}`); v2-beta GET URL-only `https://data.ssb.no/api/pxwebapi/v2-beta/tables/<id>/data?lang=en&valueCodes[Tid]=top(4)` (**offline 05:00–08:15 daily + weekends**); ready-made `https://data.ssb.no/api/v0/dataset/<id>`.

**Key model implications:**
1. **No open address-level transaction feed** — lean on SSB municipality sq-m tables + Eiendom Norge regional index for price-fairness (`market_stats`), not a free per-sale feed.
2. **No BBR-grade open per-building attribute API** — open data gives parcels/addresses/building points only. **`byggeår`/floor-area/heating must come from the listing (Plane A) or the restricted Matrikkel API**, not a free register. Plan era/heating/age risk inputs accordingly.
3. **SSB 07221 sub-national history reaches back only to ~2005 for most regions** (whole-country, Oslo+Bærum, Akershus-without-Bærum go to 1992; pre-2009 model had 4 regions not 11). From 2025 sq-m prices switch **P-rom → BRA-i** (NS3940:2023) — **pre/post-2025 sq-m values NOT directly comparable.**
4. **No unified national strandsone/100-metersbeltet WFS** (per-municipality) — derive shore proximity from water-body geometry.

**UNVERIFIED:** exact `wms/wfs.geonorge.no` Flomsoner layer slugs; SSB 07241 end-quarter; existence of any unified national strandsone WFS; any SSB address-level AVM (address-level estimators are commercial — Eiendomsverdi/Virdi/Finn).

Sources: [SSB tbl 07221](https://www.ssb.no/en/priser-og-prisindekser/boligpriser-og-boligprisindekser/statistikk/prisindeks-for-brukte-boliger) · [SSB PxWebApi](https://data.ssb.no/api/) · [Kartverket adresse-API](https://www.kartverket.no/en/api-and-data/eiendomsdata/brukarrettleiing-adresse-api) · [NVE WMS docs](https://api.nve.no/doc/web-map-service-wms/) · [Geonorge Flomsoner](https://kartkatalog.geonorge.no/metadata/flomsoner-wms/fc5f7878-8696-47f3-a9a7-d8bf51068203) · [Kartverket Se havnivå](https://www.kartverket.no/en/at-sea/se-havniva) · [Nkom bredbåndsdekning](https://nkom.no/statistikk/nokkeltall-og-interaktive-dashbord/bredbandsdekning) · [Geonorge APIs](https://www.geonorge.no/en/for-developers/apis/) · [Grunnbok/Matrikkel API docs](https://kartverket.github.io/api-dokumentasjon/docs/eiendom/).

---

## 8. Local enum vocabulary → kontu normalized enums

**`property_type` (`boligtype`):**
| Norwegian | kontu | FI analogue |
|---|---|---|
| `Enebolig` | detached | omakotitalo |
| `Tomannsbolig` | semi_detached/duplex | paritalo |
| `Rekkehus` | terraced | rivitalo |
| `Leilighet` | apartment | kerrostalo |
| `Gårdsbruk/Småbruk` | farm/smallholding | maatila |
| `Hytte`/`Fritidsbolig` (in `leisuresale`) | leisure/cabin | mökki |
| `Garasje/Parkering`, `Tomt`, `Andre` | other/exclude | — |

**`holding_form` (`eierform`)** — switches whole cost branch:
| Norwegian | kontu | dokumentavgift | FI analogue |
|---|---|---|---|
| `Selveier` (freehold) | freehold | **2.5%** | kiinteistö (3%) |
| `Andel`/borettslag | co_op (carries `fellesgjeld`) | **none** | asunto_osake (1.5%) |
| `Aksje`/aksjeleilighet | share_company | **none** | asunto_osake |
| `Obligasjon` | bond | none | — |
| `Annet` | other | — | — |

**`heating_type`** (parse from free-text description, no structured field):
| Norwegian text | kontu | FI analogue |
|---|---|---|
| `Vannbåren varme` | waterborne (distribution flag — keeps HP retrofit open) | vesikiertoinen |
| `Varmepumpe` `luft-til-luft` | air_air HP | IVLP |
| `Varmepumpe` `luft-til-vann` | air_water HP | — |
| `væske-til-vann`/`bergvarme`/`jordvarme` | ground_source | maalämpö |
| `Fjernvarme` | district | kaukolämpö |
| `Elektrisk`/`panelovn`/`varmekabler` | direct_electric | suora sähkö |
| `Vedfyring`/`peis`/`ildsted`/`pelletskamin` | wood/pellet | puu |
| `Oljefyr`/`parafin` | oil (**phase-out flag**, banned 2020) | öljy |

**`shore`** — facilities flag **`Strandlinje`** = own shoreline → kontu `oma_ranta`. Text terms: `strandtomt`, `sjøtomt`, `vannkant`, `egen strandlinje`, `strandsone` (regulated 100-m zone = also build-restriction risk); related: `båtplass` (boat slip), `naust` (boathouse).

**`condition` / energy:**
- **`Energimerking` — handle BOTH formats:** (a) **legacy (pre-Apr 2025):** letter **A–G** + **heating colour** (green=renewable/district, light-green=efficient HP, yellow=mixed, orange=mostly electric, red=electric/fossil) — colour is a heating-system proxy when text is thin; (b) **new (heating grade removed Apr 2025; combined single A–G from 1 Jan 2026)** folding in energy source, rewarding fjernvarme/grid-friendly. Mandatory in marketing for buildings >50 m² → reliably present. Parse a letter, and a colour iff present.
- **Condition/risk text → flags:** `tilstandsrapport`/`tilstandsgrad TG0–TG3` (TG2/TG3 = defect flags, mandatory since 2022), `oppussingsobjekt` (fixer-upper), `totalrenovert`, `drenering` (≈ salaojat), `våtrom`/`bad` age (wet-room), `rør`/`røropplegg` age (≈ putkiremontti), `tak` (`takstein`/`betongtakstein`/`shingel`/`torvtak`), `asbest`, `vannbåren`, `septik`/`slamavskiller`/`minirenseanlegg` (off-grid sewage ≈ jätevesi).

**`plot_ownership`:** `Eid tomt` = owned; `Festet tomt` = leased (carries `Festeavgift` ground rent — index it).

---

### Residual `(UNVERIFIED)` register (do not hardcode without live confirmation)
Multimur foundation repair cost · render-façade re-clad cost · eternit-roof replacement cost · radon mitigation cost · off-grid sewage upgrade single figure · full sill-plate+drainage combined figure · standalone national feieavgift average · eierskiftegebyr national range · FINN internal search route + numeric codes for `property_type`/`ownership_type`/`facilities`/`Strandlinje` + `no_of_bedrooms` param + `1.x.y` municipality mappings + per-rate throttle · `wms/wfs.geonorge.no` Flomsoner layer slugs · SSB 07241 discontinuation quarter · unified national strandsone WFS existence · municipality-by-municipality eiendomsskatt levying list (pull live from SSB at ingest).
