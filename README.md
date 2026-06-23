# kontu

A single-user terminal app to find and decide on a house to buy in Finland.

- **`tui/`** — Rust + ratatui terminal UI: exact-parameter filtering, side-by-side
  comparison, and a total-cost-of-ownership model over time. Opens any listing on
  its source site (Etuovi/Oikotie) in the browser.
- **`worker/`** — Cloudflare Worker: scrapes Etuovi + Oikotie on a Cron Trigger,
  normalizes both into one parameter model, stores in D1 (+ R2 for photos), and
  serves the API the TUI consumes.

Personal tool. Not affiliated with any listing portal.
