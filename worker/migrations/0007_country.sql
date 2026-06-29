-- Nordic expansion: make every data plane country-scoped. Finland is the
-- existing data, so backfill 'FI' as the default for all current rows. Country
-- is an ISO-3166-1 alpha-2 code: FI | SE | NO | DK | IS.

ALTER TABLE listings   ADD COLUMN country TEXT NOT NULL DEFAULT 'FI';
ALTER TABLE properties ADD COLUMN country TEXT NOT NULL DEFAULT 'FI';
ALTER TABLE crawl_state ADD COLUMN country TEXT NOT NULL DEFAULT 'FI';

-- Country-qualified indexes: every hot listing query now filters on country
-- first, so lead with it to keep D1 rows-read low.
CREATE INDEX idx_listings_country_city_price   ON listings(country, municipality, price_eur);
CREATE INDEX idx_listings_country_status_price ON listings(country, status, price_eur);
CREATE INDEX idx_listings_country_type_city    ON listings(country, property_type, municipality);

-- The cross-portal dedup fingerprint must not merge a Finnish and a Swedish
-- address that happen to normalize alike, so the property fingerprint is now
-- country-prefixed in the Worker. Existing fingerprints are Finnish; prefix them.
UPDATE properties SET fingerprint = 'FI|' || fingerprint WHERE fingerprint NOT LIKE '%|%' OR fingerprint NOT GLOB '[A-Z][A-Z]|*';

-- market_stats: rebuild with country in the natural key (it was UNIQUE on the
-- area tuple alone, which would now collide across countries sharing a name).
CREATE TABLE market_stats_v2 (
  id            INTEGER PRIMARY KEY,
  country       TEXT NOT NULL DEFAULT 'FI',
  area_kind     TEXT NOT NULL,
  area_code     TEXT NOT NULL,
  metric        TEXT NOT NULL,
  property_kind TEXT,
  period        TEXT,
  value         REAL,
  source        TEXT NOT NULL,
  fetched_at    INTEGER NOT NULL,
  UNIQUE(country, area_kind, area_code, metric, property_kind, period, source)
);
INSERT INTO market_stats_v2 (id, country, area_kind, area_code, metric, property_kind, period, value, source, fetched_at)
  SELECT id, 'FI', area_kind, area_code, metric, property_kind, period, value, source, fetched_at FROM market_stats;
DROP TABLE market_stats;
ALTER TABLE market_stats_v2 RENAME TO market_stats;
CREATE INDEX idx_market_area ON market_stats(country, area_kind, area_code);

-- cost_defaults: scope overrides per country (PRIMARY KEY was key alone).
CREATE TABLE cost_defaults_v2 (
  country    TEXT NOT NULL DEFAULT 'FI',
  key        TEXT NOT NULL,
  num_value  REAL,
  text_value TEXT,
  unit       TEXT,
  note       TEXT,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(country, key)
);
INSERT INTO cost_defaults_v2 (country, key, num_value, text_value, unit, note, updated_at)
  SELECT 'FI', key, num_value, text_value, unit, note, updated_at FROM cost_defaults;
DROP TABLE cost_defaults;
ALTER TABLE cost_defaults_v2 RENAME TO cost_defaults;

PRAGMA optimize;
