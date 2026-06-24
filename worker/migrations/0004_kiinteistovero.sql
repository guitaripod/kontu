-- Actual annual property tax (kiinteistövero) from the listing detail page, a
-- concrete recurring ownership cost shown on the ownership one-pager.
ALTER TABLE listings ADD COLUMN kiinteistovero_eur_yr INTEGER;
