-- kontu D1 schema. See SPEC.md §5/§6.
-- Enums kept as plain TEXT (no CHECK) for ingestion resilience against source
-- schema drift; normalization happens in the Worker. Timestamps are unix seconds.

-- Canonical cross-portal property group (dedup target).
CREATE TABLE properties (
  id           INTEGER PRIMARY KEY,
  fingerprint  TEXT NOT NULL,            -- normalize(postal|street|house_no|round(m2)|rooms[|floor])
  postal_code  TEXT,
  municipality TEXT,
  street       TEXT,
  house_no     TEXT,
  lat          REAL,
  lon          REAL,
  first_seen   INTEGER NOT NULL,
  last_seen    INTEGER NOT NULL
);
CREATE UNIQUE INDEX idx_properties_fingerprint ON properties(fingerprint);

-- One row per portal listing: normalized hot/filterable fields + raw payload.
CREATE TABLE listings (
  id                 INTEGER PRIMARY KEY,
  property_id        INTEGER REFERENCES properties(id),
  portal             TEXT NOT NULL,           -- 'oikotie' | 'etuovi'
  portal_listing_id  TEXT NOT NULL,
  url                TEXT NOT NULL,

  -- identity / type
  property_type      TEXT,                    -- omakotitalo | paritalo | rivitalo | mokki | kerrostalo | maatila ...
  holding_form       TEXT,                    -- kiinteisto | asunto_osake | maaraala | hallinnanjako
  kiinteistotunnus   TEXT,
  address            TEXT,
  municipality       TEXT,
  postal_code        TEXT,
  district           TEXT,
  lat                REAL,
  lon                REAL,

  -- price
  price_eur              INTEGER,             -- myyntihinta
  debt_free_price_eur    INTEGER,             -- velaton hinta (varainsiirtovero base)
  debt_share_eur         INTEGER,             -- velkaosuus
  price_per_m2           REAL,
  maintenance_charge_eur INTEGER,             -- hoitovastike (monthly)
  financing_charge_eur   INTEGER,             -- rahoitusvastike (monthly)
  ground_rent_eur_yr     INTEGER,             -- vuokratontti

  -- size / layout
  living_area_m2     REAL,                    -- asuinpinta-ala
  total_area_m2      REAL,
  plot_area_m2       REAL,
  room_count         REAL,
  room_layout        TEXT,                    -- '3h+k+s'
  floors             REAL,

  -- building / condition
  year_built         INTEGER,                 -- rakennusvuosi
  occupancy_year     INTEGER,
  condition_class    TEXT,                    -- hyva | tyydyttava | valttava | huono | uudiskohde
  inspection_status  TEXT,
  frame_material     TEXT,
  facade_material    TEXT,
  roof_material      TEXT,
  energy_class       TEXT,                    -- A..G (+ year)
  e_value            REAL,
  risk_structures    TEXT,                    -- JSON array of flags (valesokkeli, ...)

  -- plot / water
  plot_ownership     TEXT,                    -- oma | vuokra
  lease_end_year     INTEGER,
  shore              TEXT,                    -- oma_ranta | rantaoikeus | ei_rantaa
  shore_sauna        INTEGER,                 -- 0/1

  -- heating / utilities
  heating_type       TEXT,                    -- kaukolampo | maalampo | oljy | sahko | puu | ilmalampopumppu ...
  heat_distribution  TEXT,
  water_supply       TEXT,                    -- kunnallinen | porakaivo | rengaskaivo | kantovesi
  sewer_system       TEXT,                    -- kunnallinen | saostuskaivo | umpisailio | pienpuhdistamo
  broadband          TEXT,
  sauna              TEXT,
  parking            TEXT,
  road_access        TEXT,                    -- yleinen_tie | yksityistie
  intended_use       TEXT,                    -- vakituinen | loma
  zoning_status      TEXT,

  -- status / meta
  status             TEXT NOT NULL DEFAULT 'active',  -- active | reserved | sold | withdrawn | relisted
  raw_json           TEXT NOT NULL,           -- full scraped object (source of truth for sparse fields)
  content_hash       TEXT,                    -- hash of normalized fields, to detect change cheaply
  first_seen         INTEGER NOT NULL,
  last_seen          INTEGER NOT NULL,

  UNIQUE(portal, portal_listing_id)
);
CREATE INDEX idx_listings_property     ON listings(property_id);
CREATE INDEX idx_listings_city_price   ON listings(municipality, price_eur);
CREATE INDEX idx_listings_status_price ON listings(status, price_eur);
CREATE INDEX idx_listings_type_city    ON listings(property_type, municipality);
CREATE INDEX idx_listings_ppm2         ON listings(price_per_m2);
CREATE INDEX idx_listings_shore        ON listings(shore);
CREATE INDEX idx_listings_year         ON listings(year_built);

-- Append-only change history (price drops, status transitions). NEVER UPDATE in place.
CREATE TABLE listing_events (
  id             INTEGER PRIMARY KEY,
  listing_id     INTEGER NOT NULL REFERENCES listings(id),
  kind           TEXT NOT NULL,               -- first_seen | price_change | status_change | relisted
  old_price_eur  INTEGER,
  new_price_eur  INTEGER,
  old_value      TEXT,
  new_value      TEXT,
  observed_at    INTEGER NOT NULL
);
CREATE INDEX idx_events_listing_time ON listing_events(listing_id, observed_at);
CREATE INDEX idx_events_kind_time    ON listing_events(kind, observed_at);

-- Per-listing overrides of cost-engine assumptions (JSON of CostInputs).
CREATE TABLE listing_cost_inputs (
  listing_id  INTEGER PRIMARY KEY REFERENCES listings(id),
  inputs_json TEXT NOT NULL,
  updated_at  INTEGER NOT NULL
);

-- Personal decision layer (single user, no user_id).
CREATE TABLE listing_notes (
  listing_id INTEGER PRIMARY KEY REFERENCES listings(id),
  note       TEXT,
  updated_at INTEGER NOT NULL
);
CREATE TABLE listing_scores (
  listing_id      INTEGER PRIMARY KEY REFERENCES listings(id),
  score           INTEGER,                    -- 0..100 weighted personal score
  criteria_json   TEXT,                       -- per-criterion ratings + weights
  deal_breaker    INTEGER NOT NULL DEFAULT 0,
  rank            INTEGER,
  updated_at      INTEGER NOT NULL
);
CREATE TABLE listing_tags (
  listing_id INTEGER NOT NULL REFERENCES listings(id),
  tag        TEXT NOT NULL,
  PRIMARY KEY(listing_id, tag)
);

-- Photo manifest; R2 keyed by content hash so identical photos dedupe.
CREATE TABLE listing_photos (
  id           INTEGER PRIMARY KEY,
  listing_id   INTEGER NOT NULL REFERENCES listings(id),
  position     INTEGER NOT NULL,
  r2_key       TEXT NOT NULL,                 -- 'photos/<sha256>'
  content_type TEXT,
  source_url   TEXT NOT NULL,
  width        INTEGER,
  height       INTEGER,
  UNIQUE(listing_id, position)
);
CREATE INDEX idx_photos_listing ON listing_photos(listing_id);
CREATE TABLE seen_photo_urls (
  url    TEXT PRIMARY KEY,
  r2_key TEXT NOT NULL
);

-- Per-property location enrichment dossier (Plane B). See SPEC.md §7.
CREATE TABLE location_dossier (
  property_id  INTEGER PRIMARY KEY REFERENCES properties(id),
  dossier_json TEXT NOT NULL,                 -- distances, travel times, flood, zoning, broadband, noise, elevation
  enriched_at  INTEGER NOT NULL
);

-- Cached Plane-B price benchmarks for fairness (StatFin + MML). See SPEC.md §8.
CREATE TABLE market_stats (
  id            INTEGER PRIMARY KEY,
  area_kind     TEXT NOT NULL,                -- municipality | postal_code
  area_code     TEXT NOT NULL,
  metric        TEXT NOT NULL,                -- median_eur_m2 | mean_eur_m2 | tx_count | index
  property_kind TEXT,                         -- okt_kiinteisto | osakeasunto
  period        TEXT,                         -- e.g. '2025' | '2026Q1'
  value         REAL,
  source        TEXT NOT NULL,                -- statfin | mml
  fetched_at    INTEGER NOT NULL,
  UNIQUE(area_kind, area_code, metric, property_kind, period, source)
);
CREATE INDEX idx_market_area ON market_stats(area_kind, area_code);

-- Named exact/range searches (the "exact-parameter filtering").
CREATE TABLE saved_searches (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL,
  params_json TEXT NOT NULL,                  -- {municipality, price_max, m2_min, rooms_min, type, shore, ...}
  is_exact   INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  last_run   INTEGER
);

-- Crawl resume state (one row per portal+search source).
CREATE TABLE crawl_state (
  source      TEXT PRIMARY KEY,              -- 'oikotie:omakotitalo:outokumpu'
  next_page   INTEGER NOT NULL DEFAULT 1,
  total_pages INTEGER,
  cursor      TEXT,
  status      TEXT NOT NULL DEFAULT 'idle',  -- idle | running | done | error
  last_tick   INTEGER,
  last_error  TEXT,
  updated_at  INTEGER NOT NULL
);

-- Volatile per-source param/header maps (NOT hardcoded — they drift). See SPEC.md §5/§12.
CREATE TABLE source_config (
  source     TEXT NOT NULL,                  -- 'oikotie' | 'etuovi'
  key        TEXT NOT NULL,                  -- 'ota_meta_map' | 'building_type_codes' | 'base_url' | 'rate_limit_ms' ...
  value      TEXT NOT NULL,                  -- JSON or scalar
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(source, key)
);

-- Cost-engine seed defaults (2026), overridable. See SPEC.md §2/§3.
CREATE TABLE cost_defaults (
  key        TEXT PRIMARY KEY,
  num_value  REAL,
  text_value TEXT,
  unit       TEXT,
  note       TEXT,
  updated_at INTEGER NOT NULL
);

PRAGMA optimize;
