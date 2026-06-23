-- kontu seed data: 2026 cost defaults (verified, see SPEC.md §2/§3) + volatile
-- source param/header maps. Idempotent (INSERT OR REPLACE). Apply with:
--   wrangler d1 execute kontu --local  --file=seed.sql
--   wrangler d1 execute kontu --remote --file=seed.sql

-- ---- Cost defaults (2026) ----
INSERT OR REPLACE INTO cost_defaults (key, num_value, text_value, unit, note, updated_at) VALUES
  ('transfer_tax_kiinteisto',          0.03,    NULL, 'fraction', 'varainsiirtovero, real property; base = debt-free price', unixepoch()),
  ('transfer_tax_osake',               0.015,   NULL, 'fraction', 'varainsiirtovero, housing-company shares', unixepoch()),
  ('lainhuuto_eur',                    172,     NULL, 'eur',      'title registration (kiinteisto)', unixepoch()),
  ('kaupanvahvistus_eur',              143,     NULL, 'eur',      'deed attestation; 0 via e-conveyance', unixepoch()),
  ('kaupanvahvistus_econveyance_eur',  0,       NULL, 'eur',      'Kiinteistovaihdannan palvelu', unixepoch()),
  ('kiinnitys_eur',                    47,      NULL, 'eur',      'per mortgage deed', unixepoch()),
  ('kuntotarkastus_eur',               1450,    NULL, 'eur',      'condition inspection (range 1250-1650)', unixepoch()),

  ('euribor_12m',                      0.02809, NULL, 'fraction', '12-mo Euribor 22 Jun 2026', unixepoch()),
  ('mortgage_margin',                  0.0052,  NULL, 'fraction', 'avg new-loan margin', unixepoch()),
  ('ltv_max',                          0.90,    NULL, 'fraction', 'lainakatto', unixepoch()),
  ('ltv_first_home',                   0.95,    NULL, 'fraction', 'lainakatto first-home', unixepoch()),
  ('loan_term_years',                  25,      NULL, 'years',    'typical term', unixepoch()),

  ('kvero_building_permanent_min',     0.0041,  NULL, 'fraction', 'kiinteistovero permanent residence building band', unixepoch()),
  ('kvero_building_permanent_max',     0.0100,  NULL, 'fraction', NULL, unixepoch()),
  ('kvero_land_min',                   0.0130,  NULL, 'fraction', 'general land band', unixepoch()),
  ('kvero_land_max',                   0.0200,  NULL, 'fraction', NULL, unixepoch()),

  ('insurance_eur_yr',                 450,     NULL, 'eur/yr',   'kotivakuutus (range 240-650)', unixepoch()),
  ('heating_maalampo_eur_yr',          900,     NULL, 'eur/yr',   NULL, unixepoch()),
  ('heating_kaukolampo_eur_yr',        2200,    NULL, 'eur/yr',   NULL, unixepoch()),
  ('heating_ilmalampopumppu_eur_yr',   1400,    NULL, 'eur/yr',   'ilmavesilampopumppu', unixepoch()),
  ('heating_oljy_eur_yr',              3100,    NULL, 'eur/yr',   NULL, unixepoch()),
  ('heating_sahko_eur_yr',             4000,    NULL, 'eur/yr',   'suora sahkolammitys', unixepoch()),
  ('heating_puu_eur_yr',               1200,    NULL, 'eur/yr',   NULL, unixepoch()),
  ('electricity_eur_yr',               900,     NULL, 'eur/yr',   'non-heating household', unixepoch()),
  ('water_municipal_eur_yr',           850,     NULL, 'eur/yr',   NULL, unixepoch()),
  ('water_well_eur_yr',                200,     NULL, 'eur/yr',   'well + septic cash upkeep', unixepoch()),
  ('waste_eur_yr',                     300,     NULL, 'eur/yr',   'jatehuolto', unixepoch()),
  ('nuohous_eur_yr',                   110,     NULL, 'eur/yr',   'if fireplace', unixepoch()),
  ('tiekunta_eur_yr',                  400,     NULL, 'eur/yr',   'private road', unixepoch()),
  ('broadband_eur_yr',                 500,     NULL, 'eur/yr',   NULL, unixepoch()),
  ('maintenance_reserve_pct',          0.015,   NULL, 'fraction', 'of building rebuild value / yr', unixepoch()),

  ('discount_rate_real',               0.03,    NULL, 'fraction', 'opportunity cost (real, after-tax portfolio return)', unixepoch()),
  ('general_inflation',                0.02,    NULL, 'fraction', NULL, unixepoch()),
  ('energy_inflation',                 0.04,    NULL, 'fraction', 'energy escalates faster than CPI', unixepoch()),
  ('resale_real_growth',               0.00,    NULL, 'fraction', 'rural/lakeside often flat-to-declining', unixepoch()),
  ('seller_commission_pct',            0.035,   NULL, 'fraction', 'on future resale, incl VAT', unixepoch());

-- ---- Source config (volatile; verify codes against live DevTools, see SPEC.md §12) ----
INSERT OR REPLACE INTO source_config (source, key, value, updated_at) VALUES
  ('oikotie', 'cards_url',        'https://asunnot.oikotie.fi/api/cards', unixepoch()),
  ('oikotie', 'search_page_url',  'https://asunnot.oikotie.fi/myytavat-asunnot', unixepoch()),
  ('oikotie', 'ota_meta_map',     '{"api-token":"OTA-token","cuid":"OTA-cuid","loaded":"OTA-loaded"}', unixepoch()),
  ('oikotie', 'card_type_for_sale','100', unixepoch()),
  ('oikotie', 'building_type_codes','{}', unixepoch()),   -- VERIFY LIVE: integer codes drift
  ('oikotie', 'rate_limit_ms',    '2000', unixepoch()),
  ('oikotie', 'user_agent',       'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36', unixepoch()),
  ('etuovi',  'search_url',       'https://www.etuovi.com/api/v3/announcements/search/listpage', unixepoch()),
  ('etuovi',  'property_type_codes','{}', unixepoch()),   -- VERIFY LIVE
  ('etuovi',  'rate_limit_ms',    '800', unixepoch()),
  ('etuovi',  'user_agent',       'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36', unixepoch());
