#!/usr/bin/env python3
"""Enrich Icelandic listings with build year + coordinates from visir detail pages.

The visir search card (what the Worker cron ingests) omits Byggt and, for some rows,
coordinates — so IS risk had no era flag (the ASR 1961–79 band is era-only) and some
houses had no shore detection. The open /property/<id> page carries both. This pulls
them and posts to /api/enrich-listing, which only FILLS null fields (never overwrites
source data); the upsert COALESCEs year_built/lat/lon so a card-only re-crawl can't
wipe them. Paced + best-effort; visir is an open API, no bot gate. Wired into radar.sh.
"""
import json, os, subprocess, re, time, random, tomllib

cfg = tomllib.load(open(os.path.expanduser("~/.config/kontu/config.toml"), "rb"))
SERVER, TOKEN = cfg["server_url"], cfg["api_token"]
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"


def api_get(path):
    out = subprocess.run(
        ["curl", "-s", "-m", "30", "-H", f"Authorization: Bearer {TOKEN}", "-H", f"User-Agent: {UA}", f"{SERVER}{path}"],
        capture_output=True, text=True, timeout=40,
    ).stdout
    try:
        return json.loads(out)
    except Exception:
        return {}


def api_post(path, payload):
    json.dump(payload, open("/tmp/visir_enrich.json", "w"))
    return subprocess.run(
        ["curl", "-s", "-m", "40", "-X", "POST", "-H", f"Authorization: Bearer {TOKEN}",
         "-H", "Content-Type: application/json", "-H", f"User-Agent: {UA}",
         "--data-binary", "@/tmp/visir_enrich.json", f"{SERVER}{path}"],
        capture_output=True, text=True, timeout=50,
    ).stdout.strip()


def detail(url):
    raw = subprocess.run(
        ["curl", "-sL", "-m", "25", "-H", f"User-Agent: {UA}", "-H", "Accept-Language: is", url],
        capture_output=True, timeout=35,
    ).stdout.decode("utf-8", "replace")
    out = {}
    my = re.search(r"Byggt\s+((?:18|19|20)\d{2})", re.sub(r"<[^>]+>", " ", raw))
    if my:
        out["year_built"] = int(my.group(1))
    mc = re.search(r"lat=(-?\d+\.\d+)&lon=(-?\d+\.\d+)", raw)
    if mc:
        out["lat"], out["lon"] = float(mc.group(1)), float(mc.group(2))
    return out


# Only enrich real dwellings — visir's IS feed is dominated by farms (Jörð), bare plots
# and commercial premises that the matcher drops anyway, so detail-fetching them wastes
# the run. Capped per run; the daily radar drains the rest incrementally.
DWELLINGS = {"detached_house", "detached", "apartment", "cottage", "semi_detached", "terraced_house", "leisure"}
# visir rate-limits rapid sequential detail fetches from a residential IP (responses
# slow to the curl timeout), so pace gently and cap per run — the daily radar drains
# the rest over a few days, like the shore pass. Newest first via the API's default order.
CAP = 40
data = api_get("/api/listings?country=IS&limit=400")
pending = [
    l for l in data.get("listings", [])
    if l.get("year_built") is None and l.get("url") and l.get("property_type") in DWELLINGS
][:CAP]
print(f"IS dwellings needing build year: {len(pending)}", flush=True)

batch, ok, miss = [], 0, 0
for l in pending:
    d = detail(l["url"])
    if d.get("year_built") is not None or d.get("lat") is not None:
        batch.append({"id": l["id"], **d})
        ok += 1
    else:
        miss += 1
    if len(batch) >= 10:
        print("  POST:", api_post("/api/enrich-listing", {"updates": batch}), flush=True)
        batch = []
    time.sleep(2.5 + random.random() * 1.5)
if batch:
    print("  POST:", api_post("/api/enrich-listing", {"updates": batch}), flush=True)
print(f"enriched {ok}, no detail {miss}", flush=True)
