-- Normalized free-text listing description, used by `kontu match` for keyword
-- scoring (fiber, shore, EV charging, water/sewer, privacy …).
ALTER TABLE listings ADD COLUMN description TEXT;
