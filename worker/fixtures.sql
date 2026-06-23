-- kontu demo fixtures: ~12 realistic Finnish for-sale listings centred on the
-- user's real search area (Pohjois-Karjala / Pohjois-Savo: Outokumpu, Joensuu,
-- Kuopio, Liperi, Polvijärvi). Lets the whole system be demonstrated WITHOUT live
-- scraping (portals block datacenter IPs). Idempotent-ish: clears demo rows first.
-- Apply with:  wrangler d1 execute kontu --local --file=fixtures.sql

DELETE FROM listing_events WHERE listing_id IN (SELECT id FROM listings WHERE portal_listing_id LIKE 'demo-%');
DELETE FROM listing_photos WHERE listing_id IN (SELECT id FROM listings WHERE portal_listing_id LIKE 'demo-%');
DELETE FROM listings WHERE portal_listing_id LIKE 'demo-%';
DELETE FROM properties WHERE fingerprint LIKE 'demo|%';
DELETE FROM location_dossier WHERE property_id NOT IN (SELECT id FROM properties);
DELETE FROM market_stats WHERE source = 'demo';

-- ---- Canonical properties (cross-portal dedup targets) ----
INSERT INTO properties (id, fingerprint, postal_code, municipality, street, house_no, lat, lon, first_seen, last_seen) VALUES
  (9001, 'demo|83500|kuusikkotie|12|118|4', '83500', 'Outokumpu',  'Kuusikkotie',    '12', 62.7261, 29.0214, unixepoch() - 86400*42, unixepoch()),
  (9002, 'demo|83500|rantakatu|4|96|3',     '83500', 'Outokumpu',  'Rantakatu',      '4',  62.7298, 29.0301, unixepoch() - 86400*60, unixepoch()),
  (9003, 'demo|80100|niskakatu|9|72|3',     '80100', 'Joensuu',    'Niskakatu',      '9',  62.6010, 29.7636, unixepoch() - 86400*15, unixepoch()),
  (9004, 'demo|80140|penttilänkatu|22|54|2','80140', 'Joensuu',    'Penttilankatu',  '22', 62.6082, 29.7901, unixepoch() - 86400*9,  unixepoch()),
  (9005, 'demo|70100|tulliportinkatu|31|88|4','70100','Kuopio',    'Tulliportinkatu','31', 62.8924, 27.6770, unixepoch() - 86400*7,  unixepoch()),
  (9006, 'demo|70200|maljalahdenkatu|6|105|4','70200','Kuopio',    'Maljalahdenkatu','6',  62.8901, 27.6650, unixepoch() - 86400*22, unixepoch()),
  (9007, 'demo|83100|ylamyllyntie|18|142|5','83100', 'Liperi',     'Ylamyllyntie',   '18', 62.5320, 29.3760, unixepoch() - 86400*30, unixepoch()),
  (9008, 'demo|83100|kirkkotie|3|167|6',    '83100', 'Liperi',     'Kirkkotie',      '3',  62.5290, 29.3800, unixepoch() - 86400*5,  unixepoch()),
  (9009, 'demo|83700|kuoringantie|45|198|6','83700', 'Polvijarvi', 'Kuoringantie',   '45', 62.8580, 29.3640, unixepoch() - 86400*70, unixepoch()),
  (9010, 'demo|83700|jokitie|7|64|3',       '83700', 'Polvijarvi', 'Jokitie',        '7',  62.8600, 29.3700, unixepoch() - 86400*3,  unixepoch()),
  (9011, 'demo|83500|sysmajarventie|2|210|6','83500','Outokumpu',  'Sysmajarventie', '2',  62.7000, 28.9800, unixepoch() - 86400*18, unixepoch()),
  (9012, 'demo|80160|noljakantie|14|119|5', '80160', 'Joensuu',    'Noljakantie',    '14', 62.5950, 29.7100, unixepoch() - 86400*12, unixepoch());

-- ---- Listings ----
-- 9001 Outokumpu OKT 1978, oljy, oma ranta + sauna, valesokkeli risk, mid price
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, district, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, e_value, risk_structures, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8001, 9001, 'oikotie', 'demo-ot-1978', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-ot-1978', 'omakotitalo', 'kiinteisto', 'Kuusikkotie 12', 'Outokumpu', '83500', 'Keskusta', 62.7261, 29.0214, 142000, 142000, 1203.39, 118, 140, 2100, 4, '4h+k+s', 1, 1978, 'tyydyttava', 'puu', 'pelti', 'E', 210, '["valesokkeli","salaoja"]', 'oma', 'oma_ranta', 1, 'oljy', 'porakaivo', 'saostuskaivo', 'valokuitu', 'kyllä', 'yksityistie', 'vakituinen', 'active', '{"description":"Rauhallinen omakotitalo omalla rannalla. Rakennettu 1978, valesokkelirakenne, salaojat uusittava. Rantasauna.","images":["https://img.example/demo-ot-1978-1.jpg","https://img.example/demo-ot-1978-2.jpg"]}', 'a1b2c3d4', unixepoch() - 86400*42, unixepoch());

-- 9002 Outokumpu OKT 1962, puu, oma ranta, PRICE DROPPED, cheap
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, risk_structures, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, broadband, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8002, 9002, 'oikotie', 'demo-ot-1962', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-ot-1962', 'omakotitalo', 'kiinteisto', 'Rantakatu 4', 'Outokumpu', '83500', 62.7298, 29.0301, 55000, 55000, 572.92, 96, 3400, 3, '3h+k', 1, 1962, 'valttava', 'puu', 'tiili', 'G', '["kosteusvaurio"]', 'oma', 'oma_ranta', 0, 'puu', 'rengaskaivo', 'umpisailio', 'mokkuverkko', 'yksityistie', 'vakituinen', 'active', '{"description":"Edullinen vanha puutalo järven rannalla. Kosteusvaurio kellarissa, vaatii remonttia. Hinta laskenut.","images":["https://img.example/demo-ot-1962-1.jpg"]}', 'b2c3d4e5', unixepoch() - 86400*60, unixepoch());

-- 9003 Joensuu rivitalo 2004 asunto-osake, kaukolampo, no shore
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, district, lat, lon, price_eur, debt_free_price_eur, debt_share_eur, price_per_m2, maintenance_charge_eur, living_area_m2, room_count, room_layout, floors, year_built, condition_class, energy_class, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, sauna, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8003, 9003, 'etuovi', 'demo-jns-rt-2004', 'https://www.etuovi.com/kohde/demo-jns-rt-2004', 'rivitalo', 'asunto_osake', 'Niskakatu 9', 'Joensuu', '80100', 'Keskusta', 62.6010, 29.7636, 168000, 172000, 4000, 2333.33, 285, 72, 3, '3h+k+s', 1, 2004, 'hyva', 'C', 'oma', 'ei_rantaa', 'kaukolampo', 'kunnallinen', 'kunnallinen', 'valokuitu', 'kyllä', 'vakituinen', 'active', '{"description":"Hyväkuntoinen rivitaloasunto keskustassa. Oma sauna, kaukolämpö.","images":["https://img.example/demo-jns-rt-2004-1.jpg","https://img.example/demo-jns-rt-2004-2.jpg"]}', 'c3d4e5f6', unixepoch() - 86400*15, unixepoch());

-- 9004 Joensuu kerrostalo 2019 asunto-osake, kaukolampo, SOLD
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, district, lat, lon, price_eur, debt_free_price_eur, price_per_m2, maintenance_charge_eur, living_area_m2, room_count, room_layout, floors, year_built, condition_class, energy_class, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8004, 9004, 'etuovi', 'demo-jns-kt-2019', 'https://www.etuovi.com/kohde/demo-jns-kt-2019', 'kerrostalo', 'asunto_osake', 'Penttilankatu 22', 'Joensuu', '80140', 'Penttila', 62.6082, 29.7901, 159000, 159000, 2944.44, 198, 54, 2, '2h+kk', 4, 2019, 'uudiskohde', 'B', 'oma', 'ei_rantaa', 'kaukolampo', 'kunnallinen', 'kunnallinen', 'valokuitu', 'vakituinen', 'sold', '{"description":"Moderni kaksio uudessa kerrostalossa, myyty.","images":["https://img.example/demo-jns-kt-2019-1.jpg"]}', 'd4e5f6a7', unixepoch() - 86400*9, unixepoch());

-- 9005 Kuopio OKT 1985 kiinteisto, maalampo, vuokratontti + ground rent
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, district, lat, lon, price_eur, debt_free_price_eur, price_per_m2, ground_rent_eur_yr, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, lease_end_year, condition_class, frame_material, roof_material, energy_class, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8005, 9005, 'oikotie', 'demo-kuo-1985', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-kuo-1985', 'omakotitalo', 'kiinteisto', 'Tulliportinkatu 31', 'Kuopio', '70100', 'Keskusta', 62.8924, 27.6770, 239000, 239000, 2715.91, 1800, 88, 110, 0, 4, '4h+k+s', 2, 1985, 2048, 'hyva', 'tiili', 'tiili', 'D', 'vuokra', 'ei_rantaa', 'maalampo', 'kunnallinen', 'kunnallinen', 'valokuitu', 'kyllä', 'yleinen_tie', 'vakituinen', 'active', '{"description":"Tiilitalo vuokratontilla, maalämpö asennettu 2018. Vuokratontti, vuokra 1800 e/v.","images":["https://img.example/demo-kuo-1985-1.jpg","https://img.example/demo-kuo-1985-2.jpg","https://img.example/demo-kuo-1985-3.jpg"]}', 'e5f6a7b8', unixepoch() - 86400*7, unixepoch());

-- 9006 Kuopio paritalo 2004 asunto-osake, ilmalampopumppu
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, maintenance_charge_eur, living_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, energy_class, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, sauna, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8006, 9006, 'etuovi', 'demo-kuo-pt-2004', 'https://www.etuovi.com/kohde/demo-kuo-pt-2004', 'paritalo', 'asunto_osake', 'Maljalahdenkatu 6', 'Kuopio', '70200', 62.8901, 27.6650, 198000, 201000, 1885.71, 210, 105, 600, 4, '4h+k+s', 1, 2004, 'hyva', 'C', 'oma', 'ei_rantaa', 'ilmalampopumppu', 'kunnallinen', 'kunnallinen', 'valokuitu', 'kyllä', 'vakituinen', 'active', '{"description":"Tilava paritaloasunto, ilmalämpöpumppu, oma piha.","images":["https://img.example/demo-kuo-pt-2004-1.jpg"]}', 'f6a7b8c9', unixepoch() - 86400*22, unixepoch());

-- 9007 Liperi OKT 1985 kiinteisto, sahko, big plot
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, risk_structures, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8007, 9007, 'oikotie', 'demo-lip-1985', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-lip-1985', 'omakotitalo', 'kiinteisto', 'Ylamyllyntie 18', 'Liperi', '83100', 62.5320, 29.3760, 124000, 124000, 873.24, 142, 165, 5200, 5, '5h+k+s', 2, 1985, 'tyydyttava', 'puu', 'pelti', 'F', '["salaoja"]', 'oma', 'ei_rantaa', 'sahko', 'porakaivo', 'pienpuhdistamo', 'mokkuverkko', 'kyllä', 'yksityistie', 'vakituinen', 'active', '{"description":"Suuri omakotitalo isolla tontilla. Suora sähkölämmitys, salaojat tarkastettava.","images":["https://img.example/demo-lip-1985-1.jpg","https://img.example/demo-lip-1985-2.jpg"]}', 'a7b8c9d0', unixepoch() - 86400*30, unixepoch());

-- 9008 Liperi OKT 2019 kiinteisto, maalampo, oma ranta + sauna, premium
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, e_value, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8008, 9008, 'etuovi', 'demo-lip-2019', 'https://www.etuovi.com/kohde/demo-lip-2019', 'omakotitalo', 'kiinteisto', 'Kirkkotie 3', 'Liperi', '83100', 62.5290, 29.3800, 329000, 329000, 1970.06, 167, 190, 4100, 6, '6h+k+s', 2, 2019, 'uudiskohde', 'puu', 'pelti', 'A', 95, 'oma', 'oma_ranta', 1, 'maalampo', 'kunnallinen', 'kunnallinen', 'valokuitu', 'kyllä', 'yleinen_tie', 'vakituinen', 'active', '{"description":"Upea uudehko rantakohde omalla rannalla, maalämpö, energialuokka A, rantasauna.","images":["https://img.example/demo-lip-2019-1.jpg","https://img.example/demo-lip-2019-2.jpg","https://img.example/demo-lip-2019-3.jpg"]}', 'b8c9d0e1', unixepoch() - 86400*5, unixepoch());

-- 9009 Polvijarvi maatila 1962 kiinteisto, puu, oma ranta, rural farm
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, risk_structures, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8009, 9009, 'oikotie', 'demo-pol-tila-1962', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-pol-tila-1962', 'maatila', 'kiinteisto', 'Kuoringantie 45', 'Polvijarvi', '83700', 62.8580, 29.3640, 185000, 185000, 934.34, 198, 420, 48000, 6, '6h+k+s', 2, 1962, 'valttava', 'puu', 'tiili', 'G', '["valesokkeli","kosteusvaurio","oljysailio"]', 'oma', 'oma_ranta', 1, 'puu', 'porakaivo', 'umpisailio', 'mokkuverkko', 'kyllä', 'yksityistie', 'vakituinen', 'active', '{"description":"Maatila omalla rannalla, peltoa ja metsää 4,8 ha. Päärakennus 1962 valesokkelilla, öljysäiliö maassa, kosteusvaurioita. Rantasauna.","images":["https://img.example/demo-pol-tila-1962-1.jpg","https://img.example/demo-pol-tila-1962-2.jpg"]}', 'c9d0e1f2', unixepoch() - 86400*70, unixepoch());

-- 9010 Polvijarvi mokki 2004, puu, oma ranta + sauna, loma
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8010, 9010, 'etuovi', 'demo-pol-mokki-2004', 'https://www.etuovi.com/kohde/demo-pol-mokki-2004', 'mokki', 'kiinteisto', 'Jokitie 7', 'Polvijarvi', '83700', 62.8600, 29.3700, 89000, 89000, 1390.63, 64, 2800, 3, '2h+k+s', 1, 2004, 'hyva', 'puu', 'pelti', 'E', 'oma', 'oma_ranta', 1, 'puu', 'rengaskaivo', 'saostuskaivo', 'kyllä', 'yksityistie', 'loma', 'active', '{"description":"Hyväkuntoinen vapaa-ajan mökki omalla rannalla, oma sauna.","images":["https://img.example/demo-pol-mokki-2004-1.jpg"]}', 'd0e1f2a3', unixepoch() - 86400*3, unixepoch());

-- 9011 Outokumpu OKT 2004 kiinteisto, maalampo, oma ranta, mid-high
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, e_value, plot_ownership, shore, shore_sauna, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8011, 9011, 'oikotie', 'demo-ot-2004', 'https://asunnot.oikotie.fi/myytavat-asunnot/demo-ot-2004', 'omakotitalo', 'kiinteisto', 'Sysmajarventie 2', 'Outokumpu', '83500', 62.7000, 28.9800, 215000, 215000, 1023.81, 210, 240, 6700, 6, '5h+k+s+khh', 2, 2004, 'hyva', 'puu', 'pelti', 'C', 130, 'oma', 'oma_ranta', 1, 'maalampo', 'porakaivo', 'pienpuhdistamo', 'valokuitu', 'kyllä', 'yksityistie', 'vakituinen', 'active', '{"description":"Tilava 2000-luvun omakotitalo Sysmäjärven rannalla, maalämpö ja oma rantasauna.","images":["https://img.example/demo-ot-2004-1.jpg","https://img.example/demo-ot-2004-2.jpg"]}', 'e1f2a3b4', unixepoch() - 86400*18, unixepoch());

-- 9012 Joensuu OKT 1978 kiinteisto, kaukolampo, suburban
INSERT INTO listings (id, property_id, portal, portal_listing_id, url, property_type, holding_form, address, municipality, postal_code, district, lat, lon, price_eur, debt_free_price_eur, price_per_m2, living_area_m2, total_area_m2, plot_area_m2, room_count, room_layout, floors, year_built, condition_class, frame_material, roof_material, energy_class, risk_structures, plot_ownership, shore, heating_type, water_supply, sewer_system, broadband, sauna, road_access, intended_use, status, raw_json, content_hash, first_seen, last_seen) VALUES
  (8012, 9012, 'etuovi', 'demo-jns-1978', 'https://www.etuovi.com/kohde/demo-jns-1978', 'omakotitalo', 'kiinteisto', 'Noljakantie 14', 'Joensuu', '80160', 'Noljakka', 62.5950, 29.7100, 149000, 149000, 1252.10, 119, 138, 980, 5, '5h+k+s', 1, 1978, 'tyydyttava', 'puu', 'huopa', 'E', '["valesokkeli"]', 'oma', 'ei_rantaa', 'kaukolampo', 'kunnallinen', 'kunnallinen', 'valokuitu', 'kyllä', 'yleinen_tie', 'vakituinen', 'active', '{"description":"Omakotitalo kaupungin lähellä, kaukolämpö. 1978 rakennettu, valesokkeli.","images":["https://img.example/demo-jns-1978-1.jpg"]}', 'f2a3b4c5', unixepoch() - 86400*12, unixepoch());

-- ---- Listing events ----
-- first_seen for every listing
INSERT INTO listing_events (listing_id, kind, new_price_eur, observed_at) VALUES
  (8001, 'first_seen', 142000, unixepoch() - 86400*42),
  (8002, 'first_seen', 69000,  unixepoch() - 86400*60),
  (8003, 'first_seen', 168000, unixepoch() - 86400*15),
  (8004, 'first_seen', 159000, unixepoch() - 86400*9),
  (8005, 'first_seen', 239000, unixepoch() - 86400*7),
  (8006, 'first_seen', 198000, unixepoch() - 86400*22),
  (8007, 'first_seen', 129000, unixepoch() - 86400*30),
  (8008, 'first_seen', 329000, unixepoch() - 86400*5),
  (8009, 'first_seen', 185000, unixepoch() - 86400*70),
  (8010, 'first_seen', 89000,  unixepoch() - 86400*3),
  (8011, 'first_seen', 215000, unixepoch() - 86400*18),
  (8012, 'first_seen', 149000, unixepoch() - 86400*12);

-- price drops (8002 dropped twice, 8007 dropped once)
INSERT INTO listing_events (listing_id, kind, old_price_eur, new_price_eur, observed_at) VALUES
  (8002, 'price_change', 69000, 62000, unixepoch() - 86400*30),
  (8002, 'price_change', 62000, 55000, unixepoch() - 86400*8),
  (8007, 'price_change', 129000, 124000, unixepoch() - 86400*10);

-- a status change → sold for 8004
INSERT INTO listing_events (listing_id, kind, old_value, new_value, observed_at) VALUES
  (8004, 'status_change', 'active', 'sold', unixepoch() - 86400*1);

-- ---- Photos (placeholder R2 keys; bytes not present in R2 → /api/photos 404s gracefully) ----
INSERT INTO listing_photos (listing_id, position, r2_key, content_type, source_url) VALUES
  (8001, 1, 'photos/demo0000000000000000000000000000000000000000000000000000000001', 'image/jpeg', 'https://img.example/demo-ot-1978-1.jpg'),
  (8001, 2, 'photos/demo0000000000000000000000000000000000000000000000000000000002', 'image/jpeg', 'https://img.example/demo-ot-1978-2.jpg'),
  (8008, 1, 'photos/demo0000000000000000000000000000000000000000000000000000000003', 'image/jpeg', 'https://img.example/demo-lip-2019-1.jpg'),
  (8008, 2, 'photos/demo0000000000000000000000000000000000000000000000000000000004', 'image/jpeg', 'https://img.example/demo-lip-2019-2.jpg');

-- ---- A couple of market_stats benchmarks (demo source) ----
INSERT INTO market_stats (area_kind, area_code, metric, property_kind, period, value, source, fetched_at) VALUES
  ('municipality', 'Outokumpu', 'median_eur_m2', 'okt_kiinteisto', '2025', 980,  'demo', unixepoch()),
  ('municipality', 'Joensuu',   'median_eur_m2', 'osakeasunto',    '2025', 2150, 'demo', unixepoch()),
  ('municipality', 'Kuopio',    'median_eur_m2', 'osakeasunto',    '2025', 2480, 'demo', unixepoch()),
  ('municipality', 'Liperi',    'median_eur_m2', 'okt_kiinteisto', '2025', 1050, 'demo', unixepoch());

-- ---- One precomputed dossier for property 9008 (Liperi rantakohde) ----
INSERT INTO location_dossier (property_id, dossier_json, enriched_at) VALUES
  (9008, '{"lat":62.529,"lon":29.38,"distance_to_water_m":35,"nearest_services":{"grocery":{"name":"K-Market Liperi","distance_m":2100,"lat":62.5325,"lon":29.3712},"school":{"name":"Liperin koulu","distance_m":2600,"lat":62.5331,"lon":29.3705},"health":{"name":"Liperin terveysasema","distance_m":2800,"lat":62.5340,"lon":29.3690},"town":{"name":"Joensuu","distance_m":31000,"lat":62.601,"lon":29.7636}},"broadband":{"fibre":true,"min_100mbit":true,"min_1gbit":true},"flood_risk":{"in_zone":false,"depth_class":null,"return_period_years":null},"travel_times":null,"partial":false}', unixepoch());
