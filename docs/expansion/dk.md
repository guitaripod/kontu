I now have the full picture of kontu's conventions: the SPEC.md style (terse, numeric, bolded seeds, inline tables), the normalized enum vocabulary (property_type, holding_form, shore, heating_type, etc.), the cost_defaults key/value model, and the Plane A/Plane B split. I have everything needed to synthesize the authoritative Denmark FACTS PACK.

# kontu — Denmark (DK) FACTS PACK

Source-of-truth for the DK country dimension. Same role as SPEC.md §2/§6/§7/§8 for Finland. Terse, numeric, hardcodable. All EUR via the ERM II peg **1 EUR ≈ 7.46 DKK** (DKK floats ±2.25% around 7.46038; treat as fixed). Year-thresholds and reform mechanics are load-bearing — they drive cost/risk defaults. `(UNVERIFIED)` = no single primary source; model as a band, never a point.

## Denmark (DK) — verified facts

- **Holding-form split is the master structural fact** (the DK analogue of FI `kiinteistö` 3% vs `asunto-osake` 1.5%): **ejerbolig/ejerlejlighed** (freehold house / owner-flat = real property in the *Tingbog*) pays deed duty; **andelsbolig** (co-op share, *not* real property) pays **DKK 0** deed duty. Encode `holding_form ∈ {ejer, andel}` and branch the whole acquisition-cost model on it.
- **No buyer transfer tax beyond the deed-registration duty.** The single state levy on title transfer is **tinglysningsafgift = 0.6% × price (round UP to nearest DKK 100) + DKK 1,850 fixed**. No separate land-registry application fee on top (the DKK 1,850 *is* it). No buyer realtor fee (seller pays). No buyer mortgage duty for a cash buyer.
- **Both property taxes reformed 1 Jan 2024** (new *Ejendomsskatteloven*): collected by the **state via the owner's forskudsopgørelse** (preliminary income assessment), reconciled on the tax return — NOT the old quarterly municipal *ejendomsskattebillet*. Tax base for both = **80% of public assessment** (20% *forsigtighedsprincip* / precautionary discount).
- **Rich BBR data is exposed on the listing itself** (see §6) — heating, roof, wall, build-year, plus the per-kommune grundskyld ‰ and council-tax % baked into each Boligsiden listing. DK is the lowest-friction of the four Nordic targets: both dominant portals serve open, unauthenticated JSON.
- **Two temporary/conditional levers to model explicitly:** (a) **elafgift cut to ~0.8 øre/kWh excl. VAT for 2026–2027 ONLY** (down from ~70–90 øre, ~99% cut → ~DKK 4,000/yr saving for a heat-pump parcelhus) — build a **revert path to ~70+ øre for 2028+**; (b) **pant duty cut 1.45%→1.25% from 1 Jan 2026** (irrelevant to cash buyer).

## 1. Acquisition costs (one-time)

Cash buyer of a residential **ejerbolig/ejerlejlighed**. (For **andelsbolig**: every deed-duty line below = **DKK 0**.)

| Line item | 2026 value | ≈ EUR | Notes / encode |
|---|---|---|---|
| **Tinglysningsafgift — variable** | `ceil(0.006 × base / 100) × 100` DKK | — | round UP to nearest DKK 100. Worked: 2,015,000 → 0.6%=12,090 → **12,100**; +1,850 = **13,950**. |
| **Tinglysningsafgift — fixed** | **DKK 1,850** | ~248 | this IS the title-registration fee; no extra land-registry charge |
| *Same, andelsbolig* | **DKK 0** | 0 | no skøde — share transfer isn't real property (an *overdragelsesaftale* is used) |
| Mortgage (pant) duty | **1.25% × principal + DKK 1,825** | — | **DKK 0 cash buyer**; rate cut 1.45%→1.25% on 1 Jan 2026; *stempelrefusion* lets a buyer reuse duty on seller's existing registered pant |
| Buyer's legal advisor (*boligadvokat / køberrådgiver*) | **DKK 5,000–10,000** flat | ~670–1,340 | near-universal (*advokatforbehold*); budget/digital from ~2,000–4,950; andel package ~4,950. NOT included in the deed duty. |
| Ejerskifteforsikring (buyer's half, 5-yr basic) | **DKK 7,500–20,000** | ~1,000–2,680 | optional risk-transfer; seller legally must offer to pay the other half of a 5-yr basic premium. Full 5-yr premium DKK 15,000–40,000. `(UNVERIFIED band — market quote, not a tariff)` |
| Tilstandsrapport + elinstallationsrapport | **Seller pays** | — | DKK 6,000–13,000 seller cost (condition report valid 6 mo, el-report 12 mo) |
| Realtor commission (*ejendomsmæglersalær*) | **Seller pays** | — | ~1–2.5% of price; buyer DKK 0 unless hiring a *købermægler* |

**Base rule (the nuance):** since the 1 Jan 2021 Tinglysningsafgiftsloven §4 rewrite, the variable duty is computed on the **purchase price (*ejerskiftesum*)** as the *general* rule for fri handel — even where the public value is higher (it is NOT merely a "residential carve-out").
**Non-arms-length / family transfer (*interesseforbundne parter*) — CONDITIONAL FLOOR, encode the branch:** base = `max(purchase_price, floor% × ejendomsværdi)` where **floor% = 85 if the operative valuation is an OLD §§87–88 (ejendomsvurderingsloven) valuation, else 80** (new post-reform valuation). Both numbers are live and conditional — do not pick one. For normal fri handel this branch does not fire.

**Liability split on the deed duty (local custom, negotiable in the *købsaftale*):** **buyer pays 100% east of the Storebælt (Zealand/Sjælland incl. Copenhagen); ~50/50 west (Funen/Jutland).** Model: `buyer_share = 1.0 if region east of Storebælt else 0.5`.

**Headline for the model:** cash ejerbolig buyer faces essentially **one state tax** (0.6% rounded + DKK 1,850) + flat **~DKK 5–10k legal** + optional **~DKK 7.5–20k insurance half-premium**. **Zero deed duty if andelsbolig.**

Sources: SKM satsoversigt https://skm.dk/tal-og-metode/satser/satser-og-beloebsgraenser-i-lovgivningen/tinglysningsafgiftsloven · Skattestyrelsen 2026 pant cut https://skat.dk/erhverv/afgifter-paa-varer-og-ydelser-punktafgifter/nyhedsbrev-afgifter/tinglysningsafgift-ny-afgiftssats-pr-1-januar-2026 · Skøde Centret https://www.skoedecentret.dk/oss/tinglysningsafgift/ · andel https://www.skoedecentret.dk/ordbog/a/hvad-betyder-det-egentlig-at-koebe-en-andelsbolig-og-er-det-en-god-ide-for-mig/ · boligejer.dk https://boligejer.dk/tinglysningsafgiftsloven

## 2. Recurring annual costs

Typical year-round detached house (*parcelhus*, ~130–160 m²), 2026. Escalate each by its own inflation; energy faster than CPI.

| Cost line | DKK/yr | ≈ EUR/yr | Driver / encode |
|---|---|---|---|
| Ejendomsværdiskat | see §3 | — | assessed dwelling value × 80% × {0.51% / 1.4%} |
| Grundskyld | see §3 | — | land value × 80% × municipal ‰ |
| House insurance (*husforsikring*) | **4,000–10,000** (avg ~4,500–5,500) | 535–1,340 | size/roof/coast. Thatch (stråtag) DKK 15,000+; coastal +20–40%. Not legally required but lender-required. |
| Heating | **6,400–17,000** (HP→district); oil 26–29k | 860–2,280 | by `heating_type`, see table below |
| Electricity (non-heat, ~4,000 kWh) | **5,000–9,000** (2026 post-cut) | 670–1,210 | was 8,000–12,000; 2026 elafgift cut removes ~DKK 0.89/kWh ≈ −3,560/yr |
| Water + sewer (municipal grid) | **9,000–13,000** | 1,210–1,740 | ~DKK 64–68/m³ incl. + ~DKK 1,400 fixed; family 120–170 m³/yr; 2,000+ suppliers → wide |
| Water + sewer (well + septic, rural off-grid) | **1,000–2,000** | 135–270 | mandatory ≥1×/yr slamsugning + pump elec; no m³ charge |
| Waste (*renovation/affald*) | **4,000–5,000** | 535–670 | per household, must be fee-financed; e.g. Fredensborg 2025 DKK 4,608 |
| Chimney sweep (*skorstensfejer*) | **0** or **800–1,500** | 0 or 105–200 | only if solid-fuel/oil flue; mandatory ≥1×/yr if present |
| Private road (*privat fællesvej / vejlaug*) | **500–3,000** `(UNVERIFIED band)` | 70–400 | only if on a privat fællesvej; one-off resurfacing spikes far higher |
| Broadband (*fibernet*) | **3,000–5,300** | 400–710 | YouSee ~279/mo, Telenor 1000Mbit ~439/mo |
| Maintenance reserve | **~1.0–1.5% of value/yr** (older ~DKK 100/m²/yr) | — | **DK anchors on purchase price / value or kr/m², NOT rebuild/insurance value** (unlike FI). `(rebuild-value % UNVERIFIED for DK)` |

**Heating cost by system** (typical ~130 m² parcelhus, 2025 base; apply 2026 elafgift cut to all-electric):

| `heating_type` | DKK/yr | ≈ EUR/yr | Note |
|---|---|---|---|
| Ground-source HP (*jordvarme*) | 6,400–10,000 | 860–1,340 | lowest running, capex 140–300k. **2026: −3,500–4,500/yr** elafgift cut |
| Air-to-water HP (*luft-til-vand*) | 6,900–9,000 | 925–1,210 | capex 110–140k. **2026: −3,500–4,500/yr** |
| Air-to-air HP (*luft-til-luft*) | ~11,400 | ~1,530 | supplemental. **2026: −cut** |
| District heating (*fjernvarme*) | avg ~13,900–17,000; range 3,500–33,700 | 1,860–2,280 avg | hugely location-dependent (Hvide Sande ~3,478 → Annasminde ~33,689) |
| Wood-pellet (*pillefyr*) | ~15,000 | ~2,010 | 4,500 kg/yr |
| Natural gas | ~21,800 | ~2,920 | phase-out (political 2035, no law) |
| Oil (*oliefyr*) | 26,000–29,000 | 3,485–3,890 | most expensive; phase-out target 2030 → conversion liability (§4) |

Sources: Bolius opvarmning https://www.bolius.dk/det-koster-de-forskellige-opvarmningsformer-887 · fjernvarme https://danskfjernvarme.dk/viden-vaerktoejer/statistik/fjernvarmepriser-i-danmark · elafgift 2026 PwC https://www.pwc.dk/da/artikler/2025/08/elafgiften-saenkes.html + Skattestyrelsen https://skat.dk/erhverv/afgifter-paa-varer-og-ydelser-punktafgifter/nyhedsbrev-afgifter/midlertidig-nedsaettelse-af-elafgiften-i-2026-og-2027 · DANVA vand https://www.danva.dk/media/11419/vand-i-tal-2025-k10-final2.pdf · husforsikring https://www.gfforsikring.dk/forsikringer/husforsikring/prisen-paa-husforsikring/ · maintenance https://www.bolius.dk/vedligehold-din-bolig-kom-godt-i-gang-med-udendoers-vedligehold

## 3. Property tax regime

**Two recurring property taxes, both base = 80% of public assessment** (20% forsigtighedsprincip), both collected via forskudsopgørelse since 2024.

**A. Ejendomsværdiskat** (state tax on the *building/dwelling* value):
- Rates **2024–2026 unchanged: 0.51% (5.1‰)** up to the progression threshold; **1.4% (14‰)** above it.
- Progression threshold (*millionærknæk*), on the **post-20%-deduction base**: **2026 = DKK 9,007,000** (= DKK 11,259,000 assessed; ≈ EUR 1.21M base / 1.51M assessed). 2025 was 9,200,000 — now indexed, falling ~200k/yr. *(Use 9,007,000; skat.dk's illustrative 9,706,000 is `(UNVERIFIED as operative)`.)*
- **Same rates + threshold apply to summerhouses/flats/houses alike** — no separate leisure regime (unlike SE).
- Worked (2026): assessed 12,000,000 → base 9,600,000 → 9,007,000×0.51% = 45,936 + 593,000×1.4% = 8,302 → **DKK 54,238/yr (≈ EUR 7,270)**.

**B. Grundskyld** (municipal tax on the *land* value):
- Rate = municipal **‰** of the 80% land-value base. **Locked per-kommune by law for income years 2024–2028** (may only go lower than the bilag-1 ceiling, not higher).
- **Statutory cap = 0.3% (30‰).** Reform **abolished the old 16‰ floor and cut the ceiling 34→30‰**. **The pre-reform 16–34‰ range is OBSOLETE — do not use it.**
- **2024 national average ≈ 7.4‰ (0.74%)** `(contested — some sources cite ~13‰; prefer the per-kommune lookup)`. 2026 actual range ~**3.1‰ → 17.7‰** (Varde 17.7‰ = highest; Copenhagen ~5.1‰; most 5–10‰).
- **Increase limiter (*stigningsbegrænsning*): +4.75%/yr** for normal homes (3.5% almene). Applies identically to sommerhuse.
- **Per-kommune ‰ is available off each Boligsiden listing** as `municipality.landValueTaxLevelPerThousand` (§6) — read it off the listing rather than maintaining a table; fall back to Vurderingsportalen.

**Transition (existing owners only — buyer model assumes NONE):** *skatterabat* (discount if 2024 tax rose) **vanishes on sale** → a buyer enters the full new regime, no rabat. *Indefrysning* (now permanent) lets increases be deferred as an interest-bearing loan, repaid on sale.

Sources: SKM ejendomsskatteloven https://skm.dk/tal-og-metode/satser/satser-og-beloebsgraenser-i-lovgivningen/ejendomsskatteloven · info.skat.dk C.H.4.2.5.1 https://info.skat.dk/data.aspx?oid=1948982 · Nordea millionærknæk 2026 https://www.nordea.com/da/nyhed/millionaerknaekket-saenkes-i-2026-flere-boligejere-skal-betale-den-hoeje-sats · Vurderingsportalen grundskyldspromiller https://www.vurderingsportalen.dk/ejerbolig/boligskat/forstaa-din-boligskat/grundskyld/kommunale-grundskyldspromiller-for-ejerboliger/ · EY https://www.ey.com/da_dk/insights/tax/den-nye-ejendomsskattelov-hvordan-paavirker-den-dig

## 4. Construction-era risk flags (era → capex, for the risk model)

`build_year` (and roof/heating/wall material from BBR) is the master multiplier, exactly like FI valesokkeli. EUR via ÷7.46. `(UNVERIFIED)` = model as range/project-uplift, not a fixed sum.

| Flag | At-risk era / threshold | Capex DKK | ≈ EUR | Maps to |
|---|---|---|---|---|
| **MgO wind-barrier board** (*MgO-plader*) — highest-signal DK flag (the valesokkeli-equivalent) | install **2010 → spring 2015** (rare 2007–08); Byggeskadefonden warning 6 Mar 2015 | **150,000–350,000** | 20k–47k | `build_year` + facade material; liability shifted to bygherre after BYG-ERFA 27 Dec 2013 → treat as buyer-borne |
| **Asbestos present** | built **before 1990**; eternit/asbestos-cement prod. ban **1986** | project-uplift `(UNVERIFIED DKK)` | — | `build_year` + `roof_material=Fibercement herunder asbest`. Screen before any pre-1990 eternit roof work |
| **Eternit early-failure** (asbestos-FREE fibre-cement crumbled) | **~mid-1980s → mid-1990s** reformulation window | = roof renewal | — | `roof_material` fibre-cement + age |
| **Gasbeton/porebeton back-wall + light timber-frame** | built **1960–1979** (~450k parcelhuse) | variable `(UNVERIFIED)` | — | `build_year`; **elevated-but-moderate** ("bedre end deres rygte"), gate on the specific defect (gasbeton wall / light timber façade / old undertag) |
| **No omfangsdræn + clay soil / basement** (the salaojat-equivalent) | any age, clay (*lerjord*) or *kælder* | **60,000–270,000** | 8k–36k | drain DKK 2,000–10,000/m; pump well 25–30k; ~70,000 water-damage claims/yr nationally |
| **Radon** | built **before 2010** (worse pre-1998); East DK/islands/**Bornholm/Funen** zone | sug 15,000–75,000 (typ 25–50k) / membrane 40,000–100,000 | 2k–13.4k | `build_year` + municipality zone. Limits: act >100 Bq/m³, always >200. Measure DKK 300–800 (only conclusive test) |
| **Sewer (*kloak*) renewal** | clay pipe **pre-1970** (lerrør 50–75yr, beton 50–100, PVC 75–100) | reline ~1,000/m; TV-inspect 1,500–4,000; dig-up more | — | `build_year`; +DKK 3,000–8,000/well |
| **Stack (*faldstamme*) renewal** (the stambyte-equivalent) | cast-iron **pre-~1980**, life 70–80yr | reline 15k–30k / replace 35k–45k per stack | 2k–6k | `build_year` |
| **Roof end-of-life** | by material (see below) | material-dependent | — | `roof_material` + age. **undertag (sub-roof membrane) usually fails first** — the dominant 1960–70s roof-moisture defect |
| **Oil heating** | install banned 2013; **replace banned 1 Jul 2016** where fjernvarme/gas available; phase-out target **2030** | convert oil→HP **80,000–120,000** (less subsidy ~17k–27k) | 11k–16k | `heating_type=oil` → latent forced-conversion liability |
| **Off-grid sewage non-compliant** | *påbud* under vandområdeplan, ~2-yr deadline | **40,000–100,000** | 5.4k–13.4k | `sewer_system` — the jätevesi/157-2017 analogue (see §5) |
| **Private well untested** | single-household *enkeltindvinding* (no mandatory testing) | test 2,600–3,000; new ~50m borehole ~130,000 | — | `water_supply=well`. Limits: nitrate ≤50 mg/l, arsenic ≤5 µg/l; simplified test omits pesticides |

**Roof material lifespans** (encode end-of-life by material × recover-year):

| Roof | Life | Risk note |
|---|---|---|
| Tegl (clay tile) | 60–100 yr | outlasts the roof structure/undertag |
| Beton tagsten (concrete tile) | 40–60 yr (~50) | colour fades; heavy |
| Eternit / fibre-cement | 20–40 yr | pre-1990 = asbestos screen; mid-80s→mid-90s = premature-failure flag |
| Tagpap (felt/bitumen) | 20–50 yr (SBS up to 50) | low-quality much shorter |
| Stråtag (thatch) | 40–50 yr N-facing / 20–40 S-facing | high maintenance + fire-insurance loading |
| Metal | 30–60 yr | — |

**Model rule:** you generally **cannot swap a light roof (fibre-cement) for a heavy roof (tile)** without re-engineering rafters → no cheap like-for-like assumption.

**Off-grid sewage detail (§4 ↔ §5):** *renseklasser* SOP/SO/OP/O assigned by kommune per recipient sensitivity; *bundfældningstank* alone removes only ~30% → never standalone. Compliant: nedsivningsanlæg 40–60k (range 25–100k; clay unsuitable; life 10–15yr then re-dig) / minirenseanlæg / sandfilter / pileanlæg / samletank. Distance rules: ≥1m above groundwater, ≥300m to drinking boreholes, ≥25m to watercourse. Annual tank empty ~DKK 1,000.

Sources: MgO https://ing.dk/artikel/aartiers-vaerste-byggeskandale-populaere-vindspaerreplader-har-skadet-tusindvis-af-boliger + https://dagensbyggeri.dk/materialer/mgo-plader-ved-vejs-ende-pa-artiers-byggeskandale/ · 60s/70s defects https://www.bolius.dk/typiske-byggeskader-paa-huse-fra-60erne-og-70erne-17893 · omfangsdræn https://www.bolius.dk/hold-huset-toert-med-et-omfangsdraen-18880 · radon https://www.sst.dk/vidensbase/straalebeskyttelse/fakta-om-ioniserende-straaling/radon-i-boliger/kort-over-radon-i-danmark · asbest 1990 https://workplacedenmark.dk/health-and-safety/building-and-construction/protect-yourselves-against-asbestos · faldstammer https://bygekspert.dk/faldstammerenovering/ · kloak https://boligekspertise.dk/kloakrenovering-pris/ · roof https://www.bolius.dk/udskiftning-af-tagbelaegning-17729 · oil https://www.skrotditoliefyr.dk/regler_oliefyr.cshtml · spildevand https://mst.dk/erhverv/rent-miljoe-og-sikker-forsyning/spildevand/nedsivning-og-spildevandshaandtering-uden-for-kloakerede-omraader · well https://www.syddjurs.dk/borger/miljoe-og-klima/vand/drikkevand/egen-broendboring

## 5. Legal ownership-risk flags (boplikt/strandskydd/etc.)

**A. Bopælspligt** (year-round residence obligation — the DK-specific flag, the NO `boplikt` / SE residence analogue):
- **Trigger:** dwelling registered as **helårsbeboelse** in BBR/planloven carries an obligation it be lived in year-round (~**180 days/yr**, an occupant registered in folkeregisteret at the address). **Imposed per-kommune** in lokalplaner — NOT national-automatic.
- **Why it bites a buyer:** cannot legally buy it purely as a holiday home / leave it empty. If vacant & not for sale, must notify kommune within **6 weeks**; kommune can issue a påbud and ultimately force a tenant. Airbnb/short-let does NOT satisfy it.
- **Exemptions:** sommerhuse (separately, may only be lived in full-time **1 Mar–31 Oct**); homes actively for sale; some first-occupancy/new-build.
- **Flexbolig escape hatch:** kommune can lift bopælspligt → use as helårs OR fritid freely. **Since 2015 the permit follows the property**, BUT **some permits are still personal and lapse on resale** (new owner re-applies) — materially affects resale value. Status noted in BBR (`flexbolig`). Landbrugslov/planlov/lokalplan override.
- **Encode two flags:** (a) is the dwelling helårsbolig under bopælspligt in this kommune? (b) is there a flexbolig permit, and is it property-bound or personal?

**B. Sommerhus foreign-buyer restriction** (buyer-eligibility, surface separately): non-residents face restrictions on buying summerhouses under the sommerhus rules — an eligibility issue distinct from construction risk.

**C. Beach/dune protection** (the SE `strandskydd` analogue): **strandbeskyttelseslinje** (beach-protection), **klitfredning** (dune), **skovbyggelinje** — carried on the Matrikelkort/DAGI (query via Matriklen2 WFS, §7). Restricts building near the coast.

**D. Tilstandsrapport / ejerskifteforsikring system** (due-diligence flag, not a restriction): *Huseftersynsordningen* (standard, not mandatory) — a beskikket bygningssagkyndig produces a **tilstandsrapport** + elinstallationsrapport, enabling the buyer's ejerskifteforsikring and releasing the seller from the default **10-year hidden-defect liability** (only if delivered before the købsaftale; seller stays liable for the plot, e.g. contamination, and for wilfully concealed defects). **Damage grades (since 1 Oct 2020, replaced K0–K3):** 🔴 Red (critical, fails short-term) · 🟡 Yellow (serious, fails long-term) · ⚪ Grey (minor, can still be costly) · ⚫ Black ("should be investigated" — **buyer bears the risk if bought un-investigated; insurance excludes anything named in the report**). A Black/Red item = direct buyer-risk flag. Report valid max 6 mo. Energy label A–G mandatory for homes >60 m².

Sources: bopælspligt https://boligejer.dk/bopaelspligt-hvad-er-boligens-status + https://www.advodan.dk/da/privat/viden-til-dig/bopaelspligt-hvad-er-reglerne/ · flexbolig https://www.bolius.dk/hvad-er-en-flexbolig-36861 · huseftersyn https://www.bolius.dk/huseftersynsordningen-tilstandsrapport-og-elinstallationsrapport-17757 · ejerskifteforsikring https://forbrug.dk/emner/penge-og-forsikring/forsikring/ejerskifteforsikring-hvad-daekker-den

## 6. Listing portals (Plane A: endpoints, params, enum vocabulary)

**Portal duo = Boligsiden (primary) + Boliga (secondary). DBA/home are wrong for DK** (DBA is general classifieds, not housing; home.dk/Nybolig/EDC/Danbolig are single-agency sites that re-syndicate to the two aggregators). Both target APIs serve **open, unauthenticated JSON** and were **live-verified from this residential IP**. DK is the lowest-friction Plane-A of the four countries. **Posture:** undocumented/gray-area — website robots.txt disallows the *website* search paths and blocks AI-training crawlers, but the **separate `api.*` JSON hosts are open** `(UNVERIFIED whether API ToS forbids automated reads; rate limits undocumented)`. Run single-user, conservative pacing, plain desktop Chrome `User-Agent` + `Accept: application/json`, exactly as kontu does for Oikotie/Etuovi. Keep these param/enum maps in D1 `source_config` (they drift).

**PRIMARY — Boligsiden** `GET https://api.boligsiden.dk/search/cases`
Envelope: `{ _links, cases:[...], totalHits }`. Headers: desktop Chrome UA + `Accept: application/json`. No auth/cookie/referer.
Params (verified live): `addressTypes` (slug, repeatable) · `zipCodes` (repeatable) · `municipalities` (slug) · `cities` (slug) · `radius` (m) · `priceMin`/`priceMax` · `areaMin`/`areaMax` · `roomsMin`/`roomsMax` (**server ignores rooms range — filter client-side**) · `sort` (`price`|`timeOnMarket`|`createdAt`|`date`|`random`) · `sortAscending` · `page` (1-indexed) · `per_page` (50). Paginate until `cases.length < per_page`.
Companion: `GET /addresses/{addressID}` → full BBR record (heating/roof/wall, registrations, latestValuation, municipality tax rates). Location autocomplete resolves Danish name → slug.

**`addressTypes` enum — VERIFIED LIVE** (English slugs despite the Danish site; `country house` is NOT valid):

| slug | Danish | live hits | → kontu `property_type` |
|---|---|---|---|
| `villa` | Parcelhus / fritliggende enfamilieshus | 25,851 | `detached` |
| `terraced house` | Række-/kæde-/dobbelthus | 2,342 | `terraced`/`semi` |
| `condo` | Ejerlejlighed | 5,399 | `apartment` (owner) |
| `cooperative` | Andelsbolig | 416 | `apartment` (co-op; `holding_form=andel`) |
| `villa apartment` | Villalejlighed | 268 | `apartment` |
| `holiday house` | Sommerhus / fritidshus | 8,051 | `leisure` |
| `farm` | Landejendom / stuehus | 1,900 | `farm` |
| `hobby farm` | Nedlagt/hobbylandbrug | 625 | `farm` (smallholding) |
| `full year plot` | Helårsgrund | 3,019 | `plot` |
| `holiday plot` | Fritidsgrund | 592 | `plot` (leisure) |
| `houseboat` | Husbåd | 23 | `other` |

**Boligsiden case fields (verified live):** `caseID, slug, caseUrl (broker deep-link), status, addressType, priceCash (DKK), perAreaPrice, monthlyExpense, priceChangePercentage, housingArea, weightedArea, lotArea, basementArea, numberOfRooms, numberOfFloors, numberOfBathrooms, yearBuilt, energyLabel{classification}, coordinates{lat,lon}, hasBalcony/Terrace/Elevator, daysListed, daysOnMarket, timeOnMarket, images[], realtor{}, realEstate{}`.
**Embedded `address` object (the cost+risk goldmine):** `address.buildings[]` straight from **BBR**: `buildingName, yearBuilt, heatingInstallation, supplementaryHeating, roofingMaterial, externalWallMaterial, housingArea/totalArea/basementArea, kitchenCondition/bathroomCondition/toiletCondition`. Plus `address.coordinates, zipCode, cityName, road, houseNumber, bfeNumbers[] (BFE join key), gstkvhx (cadastral), registrations[]{amount,date,perAreaPrice,type} (tx history), latestValuation`, and **`address.municipality{ municipalityCode, slug, councilTaxPercentage, churchTaxPercentage, landValueTaxLevelPerThousand (= grundskyld ‰!) }`** → DK annual-cost model reads tax bands straight off the listing.

**SECONDARY — Boliga** `GET https://api.boliga.dk/api/v2/search/results` (for-sale) · `GET .../api/v2/sold/search/results` (sold; best DK sold-price history — the MML-tilastopalvelu analogue). Headers: `Accept: application/json, text/plain, */*` + Chrome UA. No auth.
For-sale params: `pageSize`(50), `page`, `sort` (`daysForSale-a`,`price-a`,…), `propertyType` (int, below), `priceMin/Max`, `roomsMin/Max`, `sizeMin/Max`, `lotSizeMin/Max`, `basementSizeMin/Max`, `floorMin/Max`, `buildYearMin/Max`, `sqmPriceMin/Max`, `expMin/Max`, `daysForSaleMin/Max`, `energyClassMin/Max`, `priceDevelopment=down`, `q` (free text), `openHouse`, `zipcodes` (comma), `municipality` (int code). Sold params: `+ salesDateMin`(YYYY), `zipcodeFrom/To`, `saleType` (int), `sort=date-d`.
Envelope: `{ meta:{ totalCount, totalPages, pageIndex, pageSize, maxPage }, results:[...] }`. **`meta.maxPage` caps ≈6** regardless of logical pages → **shard by zip/municipality + price band** to page through everything (same sharding as FI).

**Boliga `propertyType` int enum — VERIFIED** (the 10-value list is authoritative; the `tpanum/hjem` 4-value map is stale):

| int | Danish | → kontu |
|---|---|---|
| 1 | Villa (parcelhus) | `detached` |
| 2 | Rækkehus | `terraced` |
| 3 | Ejerlejlighed | `apartment` |
| 4 | Fritidshus (sommerhus) | `leisure` |
| 5 | Landejendom | `farm` |
| 6 | Villalejlighed | `apartment` |
| 7 | Helårsgrund | `plot` |
| 8 | Fritidsgrund | `plot` (leisure) |
| 10 | Tvangsauktion | `foreclosure` (cross-cut flag → `isForeclosure`) |

**Boliga result fields:** `id, guid, latitude, longitude, propertyType, secondaryPropertyType, price, squaremeterPrice, priceChangePercentTotal, rooms, size, lotSize, basementSize, floor, buildYear, energyClass, city, zipCode, municipality(int), daysForSale, createdDate, lastSeen, isForeclosure, isActive, openHouse, selfsale, net, exp (monthly), evaluationPrice, lastSoldDate, lastSoldPrice, images[], agentDisplayName, dawaId/adresseId (→DAWA join), bfeNr (→BBR join), additionalBuildings`.

**Shore/waterfront — NO native filter on either portal** (confirmed). Derive: (1) **geometric** — case `coordinates` → distance to OSM `natural=coastline` + `natural=water`/`waterway` via Overpass, project in **EPSG:25832 (UTM32N)**, flag ≤~150–250 m (the DK analogue of kontu's SYKE proximity); map `oma_ranta`-equiv (own shore) only when BFE cadastral geometry touches water. (2) **text** — scan description for `havudsigt, vandnær, søudsigt, strand, ved vandet` (boost only, not source of truth).

**Cross-join key:** both portals embed `dawaId`/`bfeNumbers`/`bfeNr` → join Boliga ↔ Boligsiden ↔ BBR ↔ DAWA on the same property. **BFE-nummer** (Bestemt Fast Ejendom) is the universal Datafordeler join key (analogous to FI `kiinteistötunnus`).

Sources: Boligsiden CLI wrapper https://github.com/mikkelkrogsholm/skills/blob/main/skills/boligsiden/SKILL.md · Boliga OpenAPI https://github.com/kasperjunge/boligmarkedet · propertyType enum https://github.com/johnwesti/vandkant-boliger · ejendomsdatalisten note "no public documented API" https://www.ejendomsdatalisten.dk/apis/boliga-api

## 7. Open-gov valuation & geodata (Plane B: sources, endpoints, licences)

Ecosystem consolidating onto **Datafordeler** (registers: BBR/Matriklen/DAR) + **Dataforsyningen** (map/geodata, Klimadatastyrelsen/SDFI) + **Danmarks Statistik** (stats) + **Finans Danmark** (sold prices). **Two deadlines to design around: (1) Datafordeler legacy service-users + REST retire end-2026 → use API key + GraphQL/File-download; (2) DAWA geocoder shuts down 2026 (date contested: 1 Jul / 17 Aug / 1 Oct — verify live, do NOT hardcode) → build DAR-on-Datafordeler fallback now.**

**Sold-price / transactions by municipality:**
- **Finans Danmark Boligmarkedsstatistikken** (the MML sold-price analogue) — quarterly avg **DKK/m²** + count of frie handler for parcel-/rækkehuse, ejerlejligheder, fritidshuse, since 1992, to country/region/landsdel/**kommune/postnr**. PX-Web on `rkr.statistikbank.dk`: **BM010** (by område/kommune), **BM011** (by postnr) — `(table IDs UNVERIFIED by name — confirm live before hardcoding)`. Landing https://finansdanmark.dk/tal-og-data/boligstatistik/boligmarkedsstatistikken/ . **Caveats:** method break **1 Jun 2014** (don't splice pre-2014Q1); small geos suppressed (handle nulls); no quality adjustment; ~75–80 day publication lag. Licence: free download, attribution expected `(not formally CC-BY — UNVERIFIED)`.
- **OIS** (Den Offentlige Informationsserver) — canonical per-address sale price (from SKAT at tinglysning, back to 1992; distinguishes fri handel vs familiehandel — **exclude familiehandel from comps**). **NOT a free bulk API — manual single lookups only; bulk = paid distributor. Do not scrape.** https://www.ois.dk/
- **Danmarks Statistik — Sales of real property** (aggregate) via StatBank API; CC-BY.

**Official house-price index — Danmarks Statistik StatBank** (the Tilastokeskus analogue):
- API base `https://api.statbank.dk/v1` (`/subjects /tables /tableinfo /data`). **No auth. Licence: CC 4.0 BY.** Formats JSONSTAT/JSON/CSV/XLSX/PX/TSV. **POST a JSON body** for tables with Danish-char variable codes (`SÆSON`, `OMRÅDE`). Discover vars via `/tableinfo?id={TABLE}&format=JSON&lang=en`.
- Tables (DST rebased Apr 2024 — verify live): **EJ56** (price index, 2022=100) · **EJ67** (by region/category, quarterly) · **EJ121** (seasonally adjusted, 2022=100) · **EJ99** (co-op + owner-occupied) · **EJENEU** (Eurostat HPI 2015=100). **EJ55 DISCONTINUED** (was 2006=100); `EJEN6/EJEN77 not found (UNVERIFIED)`.
- **Granularity caveat:** DST index is **region-only (5 regions)**, NOT kommune → for kommune/postnr **m²-price levels use Finans Danmark BM010/BM011**.

**Geocoding — DAWA** (`https://api.dataforsyningen.dk`, the Pelias analogue): **no key/auth.** `GET /adgangsadresser?vejnavn=…&husnr=…&struktur=mini` · `/adresser` · `/adgangsadresser/autocomplete` · **reverse** `/adgangsadresser/reverse?x={lon}&y={lat}` (srid 4326 default / 25832 ETRS89-UTM32) · **datavask** `/datavask/adgangsadresser` (fuzzy match + confidence) · `/kommuner?navn=Viborg` → `{kode, navn, regionskode}` · `/postnumre`. GeoJSON via `&format=geojson`. Use to resolve free-text kommune → `municipalityCode` (int, for Boliga) + slug (for Boligsiden). Licence: free geographic data + attribution. **DEPRECATED — shutdown 2026 (contested date) → fallback DAR.**
**DAR** (Danmarks Adresseregister, on Datafordeler — DAWA's successor): **GraphQL** (recommended) + File download. **Auth: Datafordeler account at portal.datafordeler.dk → create IT-system → API key (renew every 2 yr).** Gotchas: data-model v1.0 vs DAWA's v0.9; you must filter `status` yourself.

**Flood / shoreline / water layers (the SYKE analogue):**
- **Kystdirektoratet** — EU Floods Directive 2007/60/EF: **14 risk areas / 27 municipalities**. WebGIS "Kystatlas" (flood + erosion + coastal structures). Højvandsstatistik at **69 stations** for **20/50/100/200-yr** return periods; "Kystplanlægger" future-year screening. https://kyst.dk/klimatilpasning . `Public WFS/WMS GetCapabilities for the risk layers not surfaced (UNVERIFIED) — pull OGC endpoints from Miljøportal catalog.`
- **Danmarks Miljøportal — Arealdata** (the SYKE-WFS analogue, >1500 datasets incl. climate/flood): viewer https://danmarksarealinformation.miljoeportal.dk/ , machine catalog (WMS/WFS/WMTS + file extract per dataset) https://arealdata.miljoeportal.dk/ . **Bluespot** (terrain depressions filling with rain) served **WMTS-only**; many klimatilpasning layers WMTS-only.
- **Coastal/dune/beach-protection zones** (`strandbeskyttelseslinje`, `klitfredning`, `skovbyggelinje` — §5C) carried on the **Matrikelkort/DAGI** → query via **Matriklen2 WFS**.

**Building/cadastre registers (feed the risk model):**
- **BBR** (Bygnings- og Boligregistret — the gold-standard building register): per-building **opførelsesår, om-/tilbygningsår, areas, tagdækningsmateriale (roof), varmeinstallation/opvarmningsmiddel (heating), water/drainage, ydervægsmateriale (wall)** — exactly the era/roof/heating/pipe-age signals. Access on Datafordeler: **GraphQL (recommended) / File download JSON-CSV / REST (phased out end-2026) / WFS**. **Auth: company account requiring MitID Erhverv → apply for BBR dataset → API key.** Free but gated. **Note:** notatlinjer NOT exposed on Datafordeler (privacy) — get via OIS. (In practice kontu reads BBR off the Boligsiden listing's `address.buildings[]`, §6 — Datafordeler BBR is the authoritative backstop.)
- **Matriklen2** (cadastre — boundaries + the beach/dune protection zones, updated daily): WFS `https://wfs.datafordeler.dk/MATRIKLEN2/MatGaeldendeOgForeloebigWFS/1.0.0/WFS?apikey=…` (GML 3.2). Legacy `?username=&password=` retires end-2026 → migrate to `?apikey=`. Free basic data.
- **GeoDanmark** (nationwide topographic vector, free open base map): WFS on Datafordeler — older Vektor WFS phased out 2026.

**Broadband + coverage — Tjekditnet.dk** (Energistyrelsen, the Traficom analogue): address-level fixed-line broadband (provider/tech/max speed) + mobile coverage, since 2015. **Open data:** address-level extracts + an API + geoTIFF rasters + history to 2016, on opendata.dk https://www.opendata.dk/andres-data/tjekditnet-dk-kortlaegning-af-fastnetbredbands-og-mobildaekning . **Cadence caveat:** fixed reports ≥1×/yr, mobile 3×/yr → newest rollouts lag. **Mastedatabasen** (daily mast positions API) for distance-to-mast. Licence `(opendata.dk default open/CC — per-dataset UNVERIFIED)`.

**Auth split (encode):** open + keyless = **DST StatBank, DAWA (until shutdown), Finans Danmark statbank**. Key/MitID-gated (free but registered) = **BBR, Matriklen, DAR, GeoDanmark/Dataforsyningen**.

Sources: DST API https://www.dst.dk/en/Statistik/hjaelp-til-statistikbanken/api · DAWA docs https://dawadocs.dataforsyningen.dk/ · DAWA shutdown https://dataforsyningen.dk/data/4924 · DAR https://confluence.sdfi.dk/pages/viewpage.action?pageId=10616849 · BBR https://datafordeler.dk/dataoversigt/bygnings-og-boligregistret-bbr/ · Matriklen2 https://datafordeler.dk/dataoversigt/matriklen2-mat2/ · Kystdirektoratet https://kyst.dk/klimatilpasning · Miljøportal https://arealdata.miljoeportal.dk/ · Finans Danmark https://finansdanmark.dk/tal-og-data/boligstatistik/boligmarkedsstatistikken/ · OIS https://www.ois.dk/

## 8. Local enum vocabulary → kontu normalized enums

**`property_type`** (from Boligsiden `addressType` slug / Boliga `propertyType` int):
`villa`/1 → `detached` · `terraced house`/2 → `terraced` · `condo`/3 → `apartment` · `cooperative` → `apartment` (+`holding_form=andel`) · `villa apartment`/6 → `apartment` · `holiday house`/4 → `leisure` · `farm`/5, `hobby farm` → `farm` · `full year plot`/7 → `plot` · `holiday plot`/8 → `plot` · `houseboat` → `other` · `10` (Tvangsauktion) → cross-cut `foreclosure` flag.

**`holding_form`** (the cost-branch switch, DK analogue of FI kiinteistö/asunto_osake):
`ejerbolig`/`ejerlejlighed` → **`ejer`** (real property → pays 0.6% + DKK 1,850 deed duty) · `andelsbolig` → **`andel`** (co-op share → **DKK 0** deed duty, no skøde). *(Leasehold/lejebolig is rental, not a purchase — exclude.)*

**`heating_type`** (from BBR `heatingInstallation`, check `supplementaryHeating` for fuel):
`Fjernvarme/blokvarme` → **`district`** (kaukolämpö-equiv) · `Varmepumpe` → **`heat_pump`** (maalämpö/ilmalämpö) · `Centralvarme med én fyringsenhed` → **`central_boiler`** (resolve fuel: oil/gas/wood → flag oil for phase-out) · `Elvarme` → **`direct_electric`** · `Ovn til fast og flydende brændsel` → **`stove_solid_liquid`** (wood/oil stove → chimney-sweep + asbestos era) · `Ingen varmeinstallation` → **`none`** (typical sommerhus). `supplementaryHeating` values: `Pejs`/`Ovne…` → fireplace (chimney-sweep cost); `Varmepumpeanlæg`, `Solpaneler`.

**`shore`** (no native filter — derived geometrically, §6):
own shore (BFE cadastral geometry touches water) → **`oma_ranta`-equiv** · within ~150–250 m of OSM coastline/lake → **`rantaoikeus`-equiv / waterfront** · else → **`ei_rantaa`-equiv / none**. Text boosts: `havudsigt, søudsigt, vandnær, strand`.

**`roof_material`** (from BBR `roofingMaterial` — drives roof capex + asbestos flag):
`Tegl` → clay tile (60–100yr) · `Betontagsten` → concrete tile (40–60yr) · `Tagpap med stor/lille hældning` → felt/bitumen (20–50yr) · `Metal` (30–60yr) · **`Fibercement herunder asbest` → ASBESTOS-containing (pre-1988 eternit) — hard asbestos flag** · `Fibercement uden asbest` → fibre-cement (20–40yr; mid-80s–mid-90s = early-failure flag) · `Stråtag` → thatch (fire-insurance loading) · `Plastmaterialer`/`Glas`/`Levende tage` → other.

**`external_wall` / frame** (from BBR `externalWallMaterial`):
`Mursten` → brick · `Træ` → wood (the DK analogue to FI wood-frame valesokkel era) · `Bindingsværk` → half-timber (old) · `Letbetonsten` → aerated concrete (gasbeton — 1960–79 risk) · `Betonelementer` → concrete panel (1960–80 prefab risk) · `Fibercement herunder asbest` → asbestos · `Glas`/`Ingen` → other.

**`condition`** (from tilstandsrapport grades, since 1 Oct 2020):
🔴 Red → **`critical`** (fails short-term) · 🟡 Yellow → **`serious`** (fails long-term) · ⚪ Grey → **`minor`** (cosmetic, can still be costly) · ⚫ Black → **`uninvestigated`** (buyer bears risk; insurer excludes). BBR `kitchenCondition`/`bathroomCondition`/`toiletCondition` → per-room condition. Energy `energyLabel.classification` `a2020|a2015|a2010|b…g` → normalize all `a20xx` → **`A`**, else uppercase letter.

**`water_supply` / `sewer_system`:** grid (`kommunal vand`/`kloak`) vs off-grid (`egen brønd/boring` → well + `nedsivningsanlæg`/`minirenseanlæg`/`samletank`/`bundfældningstank` → septic) — off-grid non-compliance = §4 capex flag.

**`building_type`** (BBR `buildingName`, for primary-dwelling logic): `Fritliggende enfamilieshus (parcelhus)`, `Række- og kædehus`/`Dobbelthus`, `Stuehus til landbrugsejendom`, `Sommerhus`, `Tiloversbleven landbrugsbygning`, `Anneks…` = dwelling; `Carport`/`Garage`/`Udhus`/`Drivhus` = outbuildings (exclude from primary-dwelling logic).

---

**Residual model flags (carry into ops):** (1) family-transfer floor is conditional **80% new-valuation / 85% old §§87–88 valuation** — encode the branch, don't pick one. (2) grundskyld **7.4‰ avg contested** (some ~13‰) — prefer per-kommune lookup off the listing. (3) Finans Danmark **BM010/BM011 table IDs unverified by name** — confirm live. (4) **elafgift 0.8 øre is temporary 2026–2027** — build a revert path to ~70+ øre for 2028+. (5) DAWA shutdown date contested — build DAR fallback. (6) Datafordeler legacy REST/service-users retire end-2026 — use API-key + GraphQL. (7) Market-quote bands (ejerskifteforsikring premium, private-road, realkredit lender line, maintenance %-base) are conventions not tariffs — keep as ranges.

FILE PATHS (kontu conventions this pack maps to): `/home/marcus/Dev/kontu/SPEC.md` (the FI style this mirrors — §2 facts, §6 param model, §7 geodata, §8 price-fairness), `/home/marcus/Dev/kontu/worker/migrations/0007_country.sql` (the `cost_defaults(country,key,…)` and `market_stats(country,…)` tables these defaults populate for `country='DK'`).
