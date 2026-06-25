-- Public shareable listing pages: one snapshot per listing, addressed by id at
-- /h/:id. The payload is the kontu-CLI-computed page data (facts + cost + risk +
-- hotlink gallery URLs); the Worker never recomputes the models.
CREATE TABLE published_pages (
  listing_id   INTEGER PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,
  tier         TEXT,
  payload_json TEXT NOT NULL,
  published_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
