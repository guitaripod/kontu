-- Fields populated by per-listing detail-page enrichment (`kontu pull` deep mode):
-- the water-body type (lake vs river — a hard buyer distinction), and renovation
-- years that let the risk model suppress false "overdue" putki/roof flags.
ALTER TABLE listings ADD COLUMN water_body TEXT;            -- jarvi | joki | meri | lampi
ALTER TABLE listings ADD COLUMN roof_year INTEGER;          -- last roof renovation year
ALTER TABLE listings ADD COLUMN pipes_renovated_year INTEGER; -- last plumbing/sewer renovation year
