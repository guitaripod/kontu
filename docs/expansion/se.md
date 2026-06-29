Now I have the SPEC.md style internalized: terse, numeric, source-of-truth, tables, inline source URLs, encode-don't-ignore directives. I'll synthesize the Sweden FACTS PACK with the 8 required H2 sections, applying all fact-check corrections (prisbasbelopp 59,200; enstegstätad builder-liability not seller; MgO safe window end 4 Mar 2015; inteckning 2.0% all buyer types; etc.).

The dimensions and corrections are fully in-context; no further file access is needed. Here is the synthesized FACTS PACK.

---

## Sweden (SE) — verified facts

Single-user kontu adapter for Sweden, mirroring the Finland model. Derived from researched dimensions + adversarial fact-check; **all fact-check corrections applied** (corrected values preferred; residual gaps marked `(UNVERIFIED)`). 2026-current.

**FX basis (store SEK as source-of-truth, convert at run-time):** 1 EUR ≈ **11.0–11.3 SEK** across the source set. **Use 11.3 SEK/EUR** as the build constant (the acquisition + risk sources used it); EUR figures below are indicative. Make the rate a single `cost_default`, not a literal.

**The master holding-form split (SE analogue of FI `kiinteistö` vs `asunto-osake`) — switches whole cost branches:**
- **`fastighet`** (freehold real property: `villa`, `radhus`/`kedjehus` sold with title, `tomt`, `gård`) → buyer registers direct title at **Lantmäteriet** → pays **stämpelskatt (stamp duty) + expeditionsavgift**. The FI-`kiinteistö` analogue.
- **`bostadsrätt`** (co-op share right; you buy shares in a `bostadsrättsförening`, not real estate) → **NO stamp duty, NO lagfart, NO pantbrev**; transfer registered with the BRF, not Lantmäteriet. The FI-`asunto-osake` analogue **but cheaper** — there is **no** 1.5%-style transfer tax on it. Buyer one-time ≈ optional inspection + bank/legal only.
- **`tomträtt`** (site-leasehold: own the building, lease land from kommun; annual `tomträttsavgäld` is a recurring cost) → **treated as `fastighet` for stamp duty** (stamp duty applies to "fastigheter och tomträtter"). Exact tomträtt mechanics `(UNVERIFIED)` to the same depth.

Sources: [Lantmäteriet — Stämpelskatt och avgifter](https://www.lantmateriet.se/sv/fastighet-och-mark/kopa-aga-salja-eller-ge-bort/Stampelskatt-och-avgifter/) · [Investropa SE fees](https://investropa.com/blogs/news/sweden-property-taxes-fees) · [Booli bostadstyp guide](https://www.booli.se/kunskap/guide-vad-innebar-de-olika-bostadstyperna-egentligen).

---

## 1. Acquisition costs (one-time)

Private **cash** buyer; encode at `t=0`. **All highest-stakes figures CONFIRMED against Lantmäteriet (2026), unchanged in budget prop. 2025/26:1 — safe to hardcode.**

| Cost | Formula / amount (SEK) | ~EUR | Applies to | Notes |
|---|---|---|---|---|
| **Stämpelskatt (lagfart, stamp duty) — private** | **1.5% × base**, base = **max(köpeskilling, prior-year taxeringsvärde)** rounded **down to nearest 1,000 SEK** | — | `fastighet` / `tomträtt` only | In arm's-length sales base = price. **4.25%** for legal entities (irrelevant to private buyer). Buyer+seller jointly liable; **buyer pays** by convention. |
| **Expeditionsavgift (lagfart registration fee)** | **825 SEK** fixed | ~73 | `fastighet` / `tomträtt` | Per application; **always payable even when no stamp duty due**. |
| **Pantbrev / inteckning (mortgage deed)** | **0 (cash buyer)** — else **2.0% of new mortgage amount** (rounded down to 1,000) **+ 375 SEK/deed** | 0 | freehold w/ loan | **CORRECTION:** inteckning is **2.0% for ALL buyer types** (private and legal entity) — do NOT split by buyer type here. Existing pantbrev transfer free; new tax only on the increment above existing deeds. |
| **Överlåtelsebesiktning (inspection, buyer)** | **7,000–15,000 SEK** (default **~10,000** villa); BRF apt 3,000–6,000 | ~620–1,330 / 265–530 | all (esp. houses) | Buyer normally pays. Statutory **undersökningsplikt** (Jordabalken 4:19) → effectively mandatory for houses. Okulär (visual only); add `fuktmätning` for upper end. |
| **Mäklararvode (realtor commission)** | **0 to buyer** | 0 | — | **SELLER always pays** (villa ~3.3–3.4%; BRF ~2.8–2.9%; national range 0.9–5.6%, regressive). Informational only; do NOT add to buyer total. |
| **Dolda fel-försäkring (hidden-defect insurance)** | **0 to buyer** | 0 | — | **SELLER cost** (~5k–15k, ~10k typical house). Seller is liable 10 yr (Jordabalken 4:19) regardless; buyer gains no legal benefit from it. Do NOT add to buyer total. |

**Total buyer one-time (freehold villa, cash):** `1.5% × price + 825 SEK + ~7,000–15,000 inspection`. On a 3.5 MSEK villa ≈ 52,500 + 825 + 10,000 ≈ **63,300 SEK (~5,600 EUR) ≈ 1.8% of price**. All-in closing per Investropa ~2.5–5.5% (houses) vs ~0.5–2.0% (bostadsrätt).
**Total buyer one-time (bostadsrätt, cash):** stamp/lagfart/pantbrev = **0**; just optional lighter inspection (3,000–6,000 SEK) + any bank/legal → **~0.3–0.8% of price**.

**Deadlines/liability:** lagfart application within **3 months** of acquisition; stamp-duty invoice payable within **30 days** of Lantmäteriet's decision. Gift/inheritance → normally **no stamp duty** (only 825 SEK fee) if consideration < gift threshold (≈85% of prior-year taxeringsvärde; exact % `(UNVERIFIED)`).
**Exclude for cash buyers:** bank "administrativ avgift" (~750 SEK, e.g. Nordea) is a lender charge, not a Lantmäteriet fee.

Sources: [Lantmäteriet — Stämpelskatt och avgifter](https://www.lantmateriet.se/sv/fastighet-och-mark/kopa-aga-salja-eller-ge-bort/Stampelskatt-och-avgifter/) · [Hemnet lagfart/pantbrev guide](https://www.hemnet.se/artiklar/guider/2026/01/09/pantbrev-och-lagfartskostnad-allt-du-behover-veta) · [Setterwalls — 2026 budget tax](https://setterwalls.se/en/article/swedish-government-budget-proposal-for-2026-this-is-proposed-in-the-tax-area/) · [Hemnet husbesiktning](https://www.hemnet.se/artiklar/guider/2026/01/06/husbesiktning-och-overlatelsebesiktning-viktigt-att-veta) · [GarBo dolda fel-försäkring](https://www.garbo.se/sv-se/alla-forsakringar/dolda-fel-forsakring) · [Mäklararvode villor](https://www.maklararvode.se/artiklar/m%C3%A4klararvoden-f%C3%B6r-villor).

---

## 2. Recurring annual costs

Per year, each escalated by its own inflation (energy faster than CPI), mirroring the FI engine. Detached house (`villa`) defaults; SEK is load-bearing, EUR @ 11.0–11.3.

| Line | SEK/yr (range; **default**) | ~EUR/yr | Conditioning variable |
|---|---|---|---|
| **Kommunal fastighetsavgift** (property fee) | `min(0.75% × taxeringsvärde, CAP)`; **CAP 2026 = 10,425** (2025 = 10,074) | ≤ ~948 | **0 if värdeår ≥ 2012 (first 15 yr)** — see §3 |
| — half cap, leased land (arrende/ofri grund) | 5,212 (2026) | ~473 | own house not land |
| — statlig fastighetsskatt (edge) | **1% × taxeringsvärde**, no cap | — | under-construction / bare plot only |
| **Villaförsäkring** (home/building ins.) | 5,000–10,000; **~6,300** | 455–910; ~575 | size/year/wet-rooms/location |
| **Heating — bergvärme** (ground-source HP) | 7,000–20,000 | 640–1,820 | cheapest; ~4,000–8,000 kWh/yr el |
| **Heating — luftvärmepump** (air HP) | 6,000–20,000 | 545–1,820 | el-zone, hard-cold backup |
| **Heating — fjärrvärme** (district) | 17,500–19,500 | 1,590–1,770 | municipal monopoly price (+30–60% / 5 yr) |
| **Heating — direktverkande el** | 23,000–25,000 | 2,090–2,270 | el price (120 m² @ 1.80 SEK/kWh) |
| **Heating — olja** (oil) | ~25,000–40,000 `(UNVERIFIED)` | ~2,270–3,640 `(UNVERIFIED)` | "dyrast"; phase-out; inferred range |
| **Electricity, non-heat (energy)** | 9,000–11,000 (~5,500 kWh × ~1.70 SEK) | 820–1,000 | 2026 ~1.70 SEK/kWh all-in (range 1.20–2.20 by zone) |
| **Elnätsavgift** (fixed grid fee) | 4,200–8,400; **~5,000** | 380–765; ~455 | fuse size, zone |
| **Vatten + avlopp — municipal (VA)** | 6,000–12,000; **~10,500** | 545–1,090; ~955 | municipality (Örebro ~7,890 ↔ Värmdö ~19,360) |
| **Vatten + avlopp — off-grid** | infiltration/markbädd 1,500–4,000 · minireningsverk 5,000–12,000 · sluten tank 15,000–25,000; **default ~3,000–5,000** | 135–2,270 | system; own well = ~pump el only |
| **Sophämtning / renhållning** (waste) | 2,000–4,000; **~2,500–3,000** | 180–365 | bin size/interval/fractions |
| **Sotning + brandskyddskontroll** (chimney) | **0** if no hearth; else 150–700 | 0 / 15–65 | only if solid-fuel hearth |
| **Enskild väg / samfällighet** (private road) | **0** if municipal road; else 0–3,000 (~1,500–2,000) | 0–275 | andelstal; only if samfällighet |
| **Bredband / villafiber** | 3,600–7,200; **~4,800** (1 Gbit ~8,400–12,000) | 330–655 | speed/provider (connect 15k–45k = capex) |
| **Tomträttsavgäld** (ground rent) | from listing, indexed | — | only `tomträtt` holding form |
| **Maintenance reserve** | **1% of building/rebuild value/yr** (range 1–2%); pre-1980 or deferred → 1.5–2%; on building-value basis 0.5–1% | — | realized as lumpy capex (§4) |

**Two facts that shape SE vs FI:** (1) the home levy is a **low, nationally-capped fee (~€945 max)** with a **15-yr zero window for värdeår ≥ 2012** — flatter/cheaper than a %-of-value tax; (2) **heating spread is huge (~€640 bergvärme → ~€2,300 direktel/olja)** gated by the **four el-zones (SE1–2 north cheap hydro, SE4 south dear)** — heating system + el-zone are the dominant discriminators.

Sources: [Skatteverket fastighetsavgift](https://www.skatteverket.se/privat/fastigheterochbostad/fastighetsavgiftochfastighetsskatt.4.69ef368911e1304a625800013531.html) · [Ekonomifakta](https://www.ekonomifakta.se/sakomraden/skatt/skatt-pa-fastigheter-och-formogenhet/fastighetsskatt-och-fastighetsavgift_1212479.html) · [Hedvig villaförsäkring](https://www.hedvig.com/se/forsakringar/hemforsakring/villaforsakring/vad-kostar-en-villaforsakring) · [Vatten och Värme — fjärrvärme vs värmepump](https://vattenochvarme.se/artiklar/fjarrvarme-vs-varmepump-vad-ar-bast-for-ditt-hus/) · [Hemsol elkostnad villa](https://hemsol.se/solceller/elkostnad-villa/) · [Svenskt Vatten — vad kostar vatten](https://www.svensktvatten.se/om-oss/verksamhet-och-strategi/fakta-om-vatten/vad-kostar-vatten/) · [Villaägarna underhållsekonomi](https://www.villaagarna.se/radgivning-och-tips/boendeekonomi/artiklar/underhallsekonomi-for-villaagare/).

---

## 3. Property tax regime

Sweden replaced the state home **tax** with a capped municipal **fee** in 2008. Three levies; for a finished house only A normally applies.

**A. Kommunal fastighetsavgift (normal case):** `annual_fee = min(0.0075 × taxeringsvärde, CAP)`, where taxeringsvärde ≈ 75% of market value.
- **CAP (takbelopp), indexed yearly to inkomstbasbeloppet — CONFIRMED:** **2025 income year = 10,074 SEK (~€915)**, **2026 = 10,425 SEK (~€948)**. Cap "bites" above taxeringsvärde ≈ **1,343,200 SEK (2025 threshold)** / ≈ **1,390,000 SEK (2026)**. Half cap (arrende/ofri grund): 5,037 (2025) / **5,212 (2026)**.
- **Permanent vs fritidshus = IDENTICAL** (both `småhus`, same 0.75% / same cap). Only *relief mechanisms* attach to a permanent home.

**B. New-build exemption (high-signal, era-based) — CONFIRMED:** buildings with **värdeår ≥ 2012 → ZERO fastighetsavgift for 15 income years** (land included; permanent AND holiday). **Clock starts värdeår + 1:** värdeår 2012 → exempt income years **2013–2027**; värdeår 2025 → exempt **2026–2040**, full cap from **2041**. Older regime (värdeår ≤ 2011: 0 yr 1–5, half yr 6–10) has fully expired (full fee from income year 2022).

**C. Statlig fastighetsskatt (state property tax) — edge cases — CONFIRMED:** **1% of taxeringsvärde**, no cap, no permanent/holiday split, on **unbuilt residential plots (obebyggd tomtmark)** and **houses under construction**.

**D. Relief tied to a permanent home only:** `begränsningsregel` — pensioners ≥65 / sickness-or-activity-compensation recipients: fee on their permanent home capped so it ≤ ~4% of income (tax credit). Not for holiday homes.

**Contrast for calibration:** unlike Norway (eiendomsskatt optional per kommune) or Denmark (value-based + grundskyld), SE's home levy is a **nationwide flat-rate-but-low-capped fee** → effective annual property tax on a normal Swedish house is **bounded at ≈ €945/yr**, structurally cheap and predictable.

**Stability — CONFIRMED:** stamp duty 1.5% private rate **unchanged in 2026** (no change in budget prop. 2025/26:1; `fastighetsskatt`-återinförande is opposition-only, not policy). Safe to hardcode 1.5% + 825 SEK and the 2026 caps.

Sources: [Skatteverket fastighetsavgift och fastighetsskatt](https://www.skatteverket.se/privat/fastigheterochbostad/fastighetsavgiftochfastighetsskatt.4.69ef368911e1304a625800013531.html) · [Skatteverket — nedsättning (rättslig vägledning)](https://www4.skatteverket.se/rattsligvagledning/edition/2025.2/2852.html) · [Budgetprop. 2026](https://www.riksdagen.se/sv/dokument-och-lagar/dokument/proposition/budgetpropositionen-for-2026_hd031/html/) · [PwC budget](https://blogg.pwc.se/taxmatters/budgetpropositionen).

---

## 4. Construction-era risk flags (era → capex, for the risk model)

`RiskScore = clamp(Σ weighted flags)`; separate signed deferred-capex (SEK→EUR @ 11.3). **Build year / värdeår is the master multiplier** (FI parity). Capex are market price-guide ranges, not statutory.

| Flag | At-risk era / threshold | Capex (SEK → ~EUR) | → kontu field | Notes |
|---|---|---|---|---|
| **Enstegstätad putsad fasad** (single-stage rendered façade) — THE Swedish "fuktskandalen", **highest-signal** (valesokkeli analogue) | **1990–2007** (render direct on insulation, no drained air gap) | superficial 270k → ~24k; rot+vapour-barrier **500k+ → ~44k+**; court awards 300–500k; diagnosis besiktning 3,000–5,000 → ~270–440 | `risk_structures` | ~22,000 detached + ~144,000 flats; >50% of inspected façades had serious moisture damage. **CORRECTION (liability):** HD 2015 (Myresjöhus, T 916-13) ruled it a **konstruktionsfel** — but liability runs against the **builder/entrepreneur, NOT a private seller**. HD **denied** compensation/price-reduction on **private-to-private resales** (Dec 2015 + 2016 cases). Encode as a **capex/condition flag**, not a seller-disclosure claim. Repaired walls can leak again → residual flag persists. |
| **Blåbetong** (alum-shale aerated concrete) — radon | **1929–1975** (manufacture ceased 1975; used in construction to ~1978–1980) | ventilation upgrade low-5-figure SEK `(UNVERIFIED tight €)`; full material removal high-cost (rare) | `build_year` | Indoor radon up to ~1,000 Bq/m³; reference level **200 Bq/m³**. Fix via **ventilation/sealing**, NOT a `radonsug` (that targets soil radon). Confirm with 60-day measurement (Oct–Apr). |
| **Asbest / Eternit** | pre-**1982** (total ban; crocidolite partial ban 1976). Eternit roof life 50–70 yr → 1960s–70s at EOL | Eternit roof: removal 250–550/m² + new 550–1,400/m²; ~150 m² villa **200k–280k → ~18k–25k**; sample 500–1,500/prov | `build_year` / `roof_material` | No high-pressure washing. Licensed firm; notify Arbetsmiljöverket ≥7 days. |
| **Stambyte** (pipe/stack renewal) — putkiremontti analogue | pipe age **40–50 yr** (cast-iron 50–70, copper 40–60); *miljonprogram* 1965–75 now at EOL | villa (1 bath+1 kitchen) **200k–350k**; range 150k–500k → ~13k–44k (slab bilning +20k–50k); BRF flat 120k–300k/flat | `renovation_events` / `build_year` | Relining ~⅓ cost but life only 20–30 yr; camera-inspect first. |
| **Dränering / krypgrund** (foundation drainage / crawl-space moisture) — salaojat analogue | pre-**~1970** often deficient/none; redo ~every 30 yr (life 25–50) | re-drainage 3k–7k/löpmeter; villa total **100k–250k**, up to 300k → ~9k–27k; moisture survey 6k–15k; rotted joist 10k–60k | `risk_structures` | ~70% of crawl-space moisture is summer-air condensation → cheap dehumidifier + vapour barrier may beat full re-drainage. Diagnose first. **"sulfitbetong/sulfatbetong" is NOT a verified separate flag** (conflated with blåbetong) — do **not** encode. |
| **Enskilt avlopp** (off-grid sewage) — jätevesi/157-2017 analogue | system age **> 20 yr** (infiltration/markbädd saturate); **trekammarbrunn-only = non-compliant** | infiltration 50k–80k · markbädd 70k–120k · minireningsverk 80k–150k; broad **80k–200k → ~7k–18k**; rock blasting +20k–100k | `sewer_system` | Tillstånd required pre-install; normal/hög skyddsnivå (hög = P-reduction near water). Municipal area-by-area tillsyn; owner liable. Running cost: infiltration 1,500–4,000/yr; minireningsverk 5,000–12,000/yr. |
| **Roof material vs age** | huopa/papp 20–30 · plåt 30–60 · betongpannor 40–60 · tegel 50–100; underlayment (tätskikt) ~30 forces re-roof | papp 400–700/m² · plåt 600–1,000 · betong 700–1,200 · tegel 900–1,500; full ~100 m² 150k–250k, ~200 m² 300k–400k → ~27k–35k | `roof_material` + age | |
| **Markradon zones** (geographic, not era) | SGU soil-radon class högrisk/normalrisk/lågrisk | mitigated by radonsug (sub-slab depressurisation) `(UNVERIFIED €)` | location/dossier | Soil radon is the **most common** indoor source. Maps too coarse for one plot — use as prior; even lågrisk can exceed 200 Bq/m³. |
| **Oil heating phase-out** (weaker than FI/NO) | no general SE oil-boiler ban, but **cannot install a new oil boiler**; oil = most expensive | conversion: bergvärme 80k–180k · luft/vatten ~100k · fjärrvärme connect 40k–50k + ~30k; buried oil-tank decommission 8k–15k | `heating_type` | ~60,000 oil boilers remain; payback 4–6 yr. **SE energy-grant for småhus closed 1 Jun 2025; 2026 reinstatement proposed, NOT in force `(UNVERIFIED future state)`.** ROT-avdrag (30% labour, max 50,000 SEK/person/yr) subsidises HP install. |

**Nordic neighbours (multi-country model, encode as era thresholds):** NO fossil-oil heating **banned 1 Jan 2020** (mineral oil + parafin; new-build fossil heat banned since **2017**); NO våtrom SINTEF life 15–20 yr / drenering 20–60 yr (planning intervals, not legal limits). DK **MgO-board façade** at-risk **2010–2015**, society cost ~DKK 1–2 bn; **CORRECTION — contractor safe window = 27 Dec 2013 → 4 Mar 2015** (liable before/after; the 5 May 2015 date is the revised erfaringsblad, not the cutoff). DK Eternit/asbest roof: pre-1984 always asbestos, 1984–88 maybe, post-1988 free; life 40–60 yr. IS **ASR ("alkalívirkni") concrete pre-1979** (1961–1979 houses unprotected; 1979 ferrosilicon/silica-fume turning point) — inspection trigger, repair € `(UNVERIFIED)`. DK hulmur cavity-wall moisture: `(UNVERIFIED)` — do not encode.

Sources: [Villaägarna enstegstätade putsfasader](https://www.villaagarna.se/radgivning-och-tips/produktgranskning/artiklar/risk-for-fuktskador-i-dranerade-enstegstatade-putsfasader/) · [lagen.nu NJA 2015 s.110](https://lagen.nu/dom/nja/2015s110) · [Konsumentverket — KO mot Myresjöhus](https://www.konsumentverket.se/aktuellt/dom-i-malet-mellan-ko-och-myresjohus/) · [SSM radon FAQ](https://www.stralsakerhetsmyndigheten.se/omraden/radon/fragor-och-svar-om-radon/) · [Byggahus blåbetong](https://www.byggahus.se/renovera/blabetong-hus-fakta-atgarder) · [Arbetsmiljöverket asbest](https://www.av.se/halsa-och-sakerhet/kemiska-risker/risker-for-vissa-amnen-produkter-och-verksamheter/asbest/) · [VVS-experter stambyte villa](https://www.vvsexperter.se/prisguider/vad-kostar-ett-stambyte-for-en-villa) · [Byggahus enskilt avlopp](https://www.byggahus.se/enskilt-avlopp-guide-till-regler-priser-och-ratt-val-for-din-tomt).

---

## 5. Legal ownership-risk flags (boplikt/strandskydd/etc.)

Surface to buyer; these are usability/liquidity flags, not always capex.

**SE — Strandskydd (shoreline building ban) — the headline SE legal flag — CONFIRMED:**
- **Trigger:** automatic **100 m** from sea/lake/watercourse (land **and** water), extendable to **300 m** by Länsstyrelsen. Codified **Miljöbalken 7 kap. 13–18 §§** (note amendment **SFS 2025:512** may shift exact paragraph numbering).
- **Buyer impact:** within the zone you **may not build, dig, fill, change building use, or add private-look features** (decking, jetties, furniture) — even attefallshus/bryggor that need no bygglov still need a **strandskyddsdispens** (needs *särskilt skäl*, **not guaranteed**, costs 2,000–5,000 SEK, takes 6–12 weeks, **expires 2 yr** after laga kraft). Free public passage must be preserved. Don't demolish an existing building first (can shrink buildable footprint). A "lakeside plot" may carry **near-zero extension/rebuild rights** → real value/usability flag.
- **2025 change (1 Jul 2025):** general strandskydd removed at lakes **< 1 ha**, watercourses **< 2 m** wide, and waters created after 30 Jun 1975.

**Nordic neighbours (multi-country):**
- **NO — boplikt / konsesjon / odel:** konsesjonsfri for ordinary homes ≤100 daa total & ≤35 daa cultivated (file egenerklæring); **nullgrenseforskrift** kommuner can impose residence obligation on a normal house (check Landbruksdirektoratet/kommune). Boplikt (>35 daa cultivated / >500 daa forest, kin/odel takeover): **move in ≤1 yr, live ≥5 yr**, else apply for konsesjon. Price control waived for built farm with usable house < NOK 3,500,000. → binding 5-yr live-there + price control = hard usability/liquidity flag.
- **DK — bopælspligt / flexbolig:** helårsbolig default has bopælspligt (case-law ≈ **occupied ≥180 days/yr**, met by folkeregister). Sommerhus = inverse (full-time living only 1 Mar–31 Oct). **Flexbolig** consent may be **personal (lapses on sale → reapply, may be refused)** or follow the property — verify on **ois.dk** + kommune. From 1 Jan 2021 lokalplan-designated helårsboliger carry bopælspligt from first occupancy permit.

Sources: [Boverket strandskydd](https://www.boverket.se/sv/samhallsplanering/sa-planeras-sverige/planeringsfragor/strandskydd/) · [Naturvårdsverket strandskydd](https://www.naturvardsverket.se/amnesomraden/skyddad-natur/olika-former-av-naturskydd/strandskydd/) · [regjeringen.no konsesjon og boplikt](https://www.regjeringen.no/no/tema/mat-fiske-og-landbruk/landbrukseiendommer/innsikt/konsesjon/id2482552/) · [Bolius flexbolig](https://www.bolius.dk/hvad-er-en-flexbolig-36861).

---

## 6. Listing portals (Plane A: endpoints, params, enum vocabulary)

**Disposable by design** (FI parity). **Primary: Hemnet** (dominant aggregator, broker-fed — Oikotie analogue). **Secondary/fallback: Booli** (Hemnet-owned; aggregates active + deep `slutpriser` sold-price DB — closest SE has to structured price-history). Brokerage chains (Fastighetsbyrån, Svensk Fast, LF Fastighetsförmedling) all re-publish to Hemnet → skip. No rural-only portal; Hemnet covers `villa`/`fritidshus`/`gård`/`tomt`.

**6A. Hemnet — internal GraphQL behind Next.js (no public listing API).**
- **Recommended Plane-A path:** drive the public search URL, parse the embedded **`<script id="__NEXT_DATA__">`** JSON (`props.pageProps` cards + `buildId`); client-nav route is `https://www.hemnet.se/_next/data/<buildId>/<route>.json` (buildId rotates per deploy, read from `__NEXT_DATA__`). `(UNVERIFIED whether `_next/data` JSON is still served vs pure RSC — confirm via DevTools; treat `__NEXT_DATA__` parse as robust.)`
- GraphQL backend runs on **Hive Gateway behind Cloudflare** (a Cloudflare Worker routes by operation name) with **persisted queries (SHA-256 hashes)** → naive POSTs rejected; must replay operation name + variables. Exact host (`gql.hemnet.se` vs `www.hemnet.se/graphql`) `(UNVERIFIED)` — capture via DevTools.
- **Search URL grammar (base `https://www.hemnet.se/bostader`; sold `…/salda/bostader`):**

| Param | Meaning | Example |
|---|---|---|
| `location_ids[]` | numeric kommun/district/street ID, **repeatable** (`location_ids%5B%5D=`) | `813512` (Hedemora) |
| `item_types[]` | property type, **repeatable** | `item_types[]=villa` |
| `housing_form_groups[]` | coarse group | `vacation_homes` |
| `price_min` / `price_max` | asking SEK `(spelling UNVERIFIED)` | |
| `rooms_min` / `rooms_max` | rooms, floats allowed | `rooms_max=3.5` |
| `living_area_min` / `living_area_max` | m² `(names UNVERIFIED; by=living_area sort confirmed)` | |
| `by` / `order` | sort | `by=living_area&order=asc` |
| `sold_age` | sold window | `sold_age=all` |
| `page` | pagination | `page=2` |

- **Caps:** **~2,500 listings/for-sale query**, **50 pages/sold query** → **tile by (kommun location_id × item_type)** (kontu already does per-(municipality × property_type)). **location_id** resolution: internal autocomplete/typeahead (GraphQL area search), not documented — capture via DevTools or pre-seed a kommun→id table.
- **Do NOT use the Hemnet BostadsAPI** (`integration.hemnet.se/documentation/v1`) — it's a broker **write/publish** API behind an Annons- och Förmedlingsavtal. **But it is the authoritative field/enum source** (esp. `EnergyClassification`: `performance`, `classification`, `heatingType`, `heatingCost`, `declarationUrl`).

**6B. Booli — `https://api.booli.se/graphql` (internal, site's own data API) + URL grammar.**
- GraphQL confirmed filter inputs: `location` (area ID or name), `minPrice`/`maxPrice` (SEK), `minRooms`/`maxRooms`, `minArea`/`maxArea` (m²); separate area-lookup tool resolves name→ID.
- URL grammar (base `https://www.booli.se/sok/till-salu`; sold `…/sok/slutpriser`): `areaIds=<id>` (**Booli internal IDs, NOT SCB kommun codes** — Stockholm kommun=`1`, county=`2`, Södermalm=`115341`), `objectType=` (comma-separated), `page=`. Other param names `(UNVERIFIED)` — use GraphQL `min*/max*`. **Legacy REST key registration effectively closed** post-acquisition — do not depend on it.

**6C. Bot-detection / robots / ToS:**
- **Hemnet** behind **Cloudflare**; `robots.txt` returned **403 to automated fetch** → expect JA3/JA4 TLS-fingerprint + header checks; plain `reqwest`/`requests` likely 403. Use browser-grade TLS (`curl_cffi`/rustls browser cipher order) + full headers (`Accept-Language: sv-SE`, `sec-ch-ua*`), jittered pacing (~1 req/1.5–3 s). Hard ceilings = the 2,500/50-page caps.
- **Booli robots.txt (verified): `/sok`, `/graphql`, `/api` NOT disallowed** (only `/auth/`,`/save/`,`/user/`,`/mail/` etc.) → **Booli is the ToS-cleanest path**. Apify note: needs **Swedish residential proxies** (geo/IP-gated).
- **Both gate on Swedish residential IP** → run `pull` from the user's residential SE IP (FI parity: kontu already runs `pull` from the user's machine). Treat as single-user, low-volume, personal-use. `(Whether Hemnet anti-bot is Cloudflare-only vs DataDome: UNVERIFIED.)`

Sources: [Hemnet GraphQL/Hive/Cloudflare-Worker architecture](https://career.hemnet.se/posts/how-hemnet-migrated-its-graphql-backend-without-anyone-noticing) · [Hemnet BostadsAPI docs](https://integration.hemnet.se/documentation/v1) · [Booli api.booli.se/graphql + MCP filters](https://lobehub.com/mcp/matt1as-booli-mcp-cc) · [Booli robots.txt](https://www.booli.se/robots.txt) · [lexis-solutions Hemnet scraper enums](https://apify.com/lexis-solutions/hemnet-se-scraper) · [Booli scraper needs SE residential proxies](https://apify.com/lexis-solutions/booli-se-scraper).

---

## 7. Open-gov valuation & geodata (Plane B: sources, endpoints, licences)

The trustworthy backbone (FI parity). **2025 watershed:** since **2025-02-09** (EU HVD reg.) Lantmäteriet core geodata is free — open map products **CC0 1.0**; address/property "Direkt" APIs **avgiftsfri under CC BY 4.0** but still need a **signed licence** (personal data in the register). Older "paid" docs are stale.

| Need | Source / endpoint | Format / auth | Licence |
|---|---|---|---|
| **Price benchmark (municipal)** | **SCB småhus price stats** (one-/two-dwelling, by kommun, rolling 3-mo + annual) via **PxWeb API** | REST, **no key**; HTTP 429 if over rate | **CC0** |
| **Official house-price index** | **SCB Fastighetsprisindex** table **`FastpiPSRegKv`** (permanent småhus, base 1981=100, quarterly 1986K1→, **region-only, NOT municipal**). PxWeb base `https://api.scb.se/OV0104/v1/doris/{en\|sv}/ssd/...` | GET metadata / POST query; json, json-stat2, px, csv; **no key** | **CC0** |
| **Transaction-level sold prices** (gated enrichment) | **Lantmäteriet Fastighetsprisregistret** / Real Property Price Download — price, date, designation, central-point coords, type code, assessed value | ordered via **Geotorget**; **privacy-gated** (personal-id vetting) | not CC0 |
| **Address geocoding** | **Lantmäteriet Belägenhetsadress Direkt** (M2M only) | REST/JSON; **Basic Auth or OAuth2**, key via API-portal, **signed licence via Geotorget** | **CC BY 4.0** (free since 2025) |
| — bulk geocode alt | **Belägenhetsadress Nedladdning, vektor** (290 files, 1/kommun) + STAC | file download | **CC0** |
| **Flood** | **MSB Översvämningsportalen** — 100-yr / 200-yr / calculated-highest (~10,000-yr) + EU-Floods-Directive risk; new DEM 2×2 m | **WMS** (incl. INSPIRE WMS; append `?request=getcapabilities`); bulk Shape + GeoTIFF | open |
| **Strandskydd** (shoreline ban) | **NOT in MSB nor Naturvårdsverket "Skyddad natur"** — administered by **Länsstyrelserna**, via **Geodatakatalogen (GDK)** WMS/WFS | WMS/WFS | free |
| **Protected areas / water-protection** | **Naturvårdsregistret** WMS `https://geodata.naturvardsverket.se/naturvardsregistret/wms` (+ WFS/REST), nightly | WMS/WFS/REST | **CC0** |
| **Broadband** | **PTS Bredbandskartan** — by län/kommun/**250×250 m grid** + technology; layers as **WMS** (Hajk front-end). `>98%` can get 1 Gbit/s | WMS; stats portal `statistik.pts.se` `(path UNVERIFIED)` | free |
| **Building/cadastre** | **Lantmäteriet Fastighetsregistret / Byggnad** — footprints, parcels, addresses; specs under `namespace.lantmateriet.se/distribution/...`; live host `api.lantmateriet.se` | bulk vectors CC0; register attrs licence-gated | CC0 (geom) / gated (register) |

**Geodata enrichment pipeline (SE-mapped, Worker background → dossier; never block TUI on live WFS):**
1. Geocode → Belägenhetsadress Direkt (fall back to portal card lat/lng).
2. Plot/building → Lantmäteriet Byggnad (OGC API / WMS).
3. **Distance to water (headline)** → MSB/Lantmäteriet water polygons + SMHI (the SE analogue of SYKE Ranta10) for shore-adjacency cross-check.
4. **Flood** → MSB Översvämningsportalen WMS (depth class + return period).
5. **Strandskydd** → Länsstyrelsen GDK WMS/WFS (the SE-specific legal overlay; no FI equivalent).
6. Broadband → PTS Bredbandskartan WMS at address/grid.
7. Distance to services → OSM Overpass `around:` (FI parity).

`(UNVERIFIED to confirm before coding: exact Belägenhetsadress Direkt REST base path; `statistik.pts.se` open-data path.)`

Sources: [Lantmäteriet öppna data](https://www.lantmateriet.se/oppnadata) · [Open data EN](https://www.lantmateriet.se/en/geodata/our-products/open-data/) · [SCB FastpiPSRegKv](https://www.statistikdatabasen.scb.se/goto/sv/ssd/FastpiPSRegKv) · [SCB PxWeb API description](https://www.scb.se/globalassets/vara-tjanster/px-programmen/pxwebapi1.0_description_2024.pdf) · [Belägenhetsadress Direkt](https://www.lantmateriet.se/sv/geodata/vara-produkter/produktlista/belagenhetsadress-direkt/) · [MSB Översvämningsportalen](https://gisapp.msb.se/Apps/oversvamningsportal/index.html) · [Länsstyrelsen Geodatakatalogen](https://gis.lansstyrelsen.se/geodata/geodatakatalogen/) · [Naturvårdsverket öppna data](https://oppnadata.naturvardsverket.se/) · [PTS Bredbandskartan](https://bredbandskartan.pts.se/).

---

## 8. Local enum vocabulary → kontu normalized enums

**8A. property_type** (Hemnet `item_types[]` ↔ Booli `objectType`):

| Hemnet `item_types[]` | Booli `objectType` | → kontu `property_type` |
|---|---|---|
| `villa` | `Villa` | `detached_house` (FI omakotitalo) |
| `parhus` / `kedjehus` | `Kedjehus-Parhus-Radhus` (single combined value) | `semi_detached` (FI paritalo) |
| `radhus` | `Kedjehus-Parhus-Radhus` (collapse) | `terraced` (FI rivitalo) |
| `lagenhet` / `bostadsratt` | `Lägenhet` | `apartment` (FI kerrostalo) |
| `fritidshus` | `Fritidshus` | `leisure_home` (FI mökki) — **most lakeside cabins** |
| `gard` | `Gård` | `farm` (FI maatila) |
| `tomt` | `Tomt/Mark` (encode `Tomt%2FMark`) | `plot` |

**8B. holding form / tenure (upplåtelseform)** — the master cost-branch switch:

| Swedish | Meaning | → kontu `holding_form` | Stamp duty |
|---|---|---|---|
| `fastighet` / `äganderätt` | freehold real property / full title | `freehold` (FI kiinteistö) | 1.5% + 825 SEK |
| `bostadsrätt` | co-op share right (avgift to BRF) | `coop` (FI asunto_osake) | **none** |
| `tomträtt` | site-leasehold (own building, lease land; annual avgäld) | `leasehold_land` | as `fastighet` |
| `hyresrätt` | rental (rare in for-sale) | `rental` | — |

**8C. heating_type (uppvärmning):**

| Swedish | → kontu `heating_type` (FI analogue) |
|---|---|
| `fjärrvärme` | `district` (kaukolämpö) |
| `bergvärme` / `jordvärme` / `berg-/jord-/luft-vattenvärmepump` | `ground_source` (maalämpö) |
| `luftvärmepump` / `luft-luft` | `air_source_heatpump` (IVLP) |
| `direktverkande el` / `elvärme` | `direct_electric` (suora sähkö — highest op-cost) |
| `vattenburen el` | `electric_hydronic` |
| `pellets` / `ved` / `biobränsle` | `wood_biomass` (puu) |
| `olja` | `oil` (öljy — phase-out flag) |

**8D. shore — IMPORTANT GAP (no structured filter confirmed on either portal):** match free-text + geo cross-check (snap lat/lon to Lantmäteriet/SMHI water polygons — the robust signal; broker text is noisy).

| Swedish lexicon | → kontu `shore` (FI analogue) |
|---|---|
| `egen strand` / `sjötomt` / `strandtomt` / `egen brygga` (own jetty = strong signal) | `own_shore` (oma_ranta) |
| `sjönära` / `strandrätt` (shared) | `shore_right` (rantaoikeus) |
| `sjöutsikt` / `havsutsikt` (view only) | `water_view` |
| none | `no_shore` (ei_rantaa) |

Proxy via `item_types`: `fritidshus` + `gård` capture bulk lakeside stock. Booli `sjö*`/`strand` filter field `(UNVERIFIED — introspect api.booli.se/graphql)`.

**8E. condition / energy_class:** Boverket energy class **`A`–`G`** (A best), reported as **kWh/m²/år**; **from 2026-05-25 a new top class `A0`** (nollutsläppsbyggnad) prepended → full enum **`A0, A, B, C, D, E, F, G`** (new-builds must reach ≥ C). Map to a `energy_class` ordinal (A0=0). Listing condition language is free-text (no Hemnet/Booli structured `condition_class` enum confirmed) → derive from `byggår` + `energiklass` + `renovation` description; treat like FI `condition_class` heuristically.

**8F. water_supply / sewer_system:** `kommunalt VA` → `municipal`; `egen brunn`/`borrad brunn` (well) → `well`; `enskilt avlopp` variants → `infiltration` / `markbädd` / `minireningsverk` / `sluten tank` (the >20-yr / trekammarbrunn-only compliance flag in §4). `road_access`: `kommunal väg` → `public`; `enskild väg`/`samfällighet` → `private` (drives the §2 private-road line).

**Detail-page fields to parse** (beyond card): `boarea` (living m²) + `biarea` + `tomtarea` (plot m²), `byggår`/`constructionYear`, **energiklass + energiprestanda (kWh/m²/år)**, **driftkostnad (SEK/yr)** + **uppvärmning**, `avgift` (BRF monthly), broker, free-text description (shore/well/heating often hide here), lat/lon. Hemnet `EnergyClassification` is the cleanest structured heating/energy source.

Sources: [Hemnet scraper enums](https://apify.com/lexis-solutions/hemnet-se-scraper) · [Booli objectType live URLs](https://www.booli.se/sok/till-salu?areaIds=64&objectType=Fritidshus,Villa,Tomt/Mark,Kedjehus-Parhus-Radhus&page=8) · [Hemnet energideklaration + A0 from 2026-05-25](https://www.hemnet.se/artiklar/guider/2025/12/16/energideklaration-en-guide-for-bostadsagare) · [Boverket energideklaration](https://www.boverket.se/sv/energideklaration/energideklaration/energideklarationens-innehall/) · [Hemnet driftkostnad/uppvärmning](https://www.hemnet.se/artiklar/guider/2025/12/27/driftkostnad-i-bostader-sa-raknas-det-ut) · [Booli wrapper field schema](https://github.com/filipsalo/booliapi/blob/master/README.md).

---

### Fact-check corrections applied (audit trail)
- **prisbasbelopp 2026 = 59,200 SEK** (not 58,000; förordning 2025:872) → dolda-fel deductibles (1–3 PBB) = **59,200–177,600 SEK** (seller cost, not in buyer total).
- **Enstegstätad liability → builder/entrepreneur, NOT private seller** (HD denied private-to-private compensation) → encoded as capex/condition flag (§4), not seller-disclosure.
- **DK MgO contractor safe window = 27 Dec 2013 → 4 Mar 2015** (not 5 May 2015).
- **Inteckning 2.0% for ALL buyer types** (no 4.25% entity split on inteckning).
- **New-build exemption clock starts värdeår + 1** (15 income years; worked examples consistent).
- Minor: asbestos 1976 partial (crocidolite) before 1982 total ban; NO new-build fossil-heat ban since 2017 (broader than the 2020 oil framing).
