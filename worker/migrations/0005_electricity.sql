-- Actual average yearly electricity spend ("Keskimääräinen sähkönkulutus") from
-- the listing detail page. For rural electric/wood-heated houses this is the real
-- energy bill, so the cost model uses it as the total energy cost (heating + power)
-- instead of a default, making the cost-of-living figure accurate.
ALTER TABLE listings ADD COLUMN electricity_eur_yr INTEGER;
