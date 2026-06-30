#!/usr/bin/env python3
"""Geometric shore detection from the residential IP -> Worker.

The Worker runs the same OSM/Overpass shore detection on its cron, but the public
Overpass API rate-limits Cloudflare's egress under load; this residential machine's
IP is served freely, so it's the reliable place to classify a fresh cross-Nordic
ingest. Pulls listings that have coordinates but no shore yet, asks Overpass whether
each sits on a lake (jarvi) or coastline (meri), and posts the verdicts back via
/api/set-shore. Paced and best-effort: a failed probe leaves the listing pending for
the next run (never written as 'no shore'), mirroring the Worker's failure handling.
"""
import json, os, subprocess, time, random, tomllib

cfg = tomllib.load(open(os.path.expanduser("~/.config/kontu/config.toml"), "rb"))
SERVER, TOKEN = cfg["server_url"], cfg["api_token"]
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
COUNTRIES = ("SE", "NO", "DK", "IS")
# Several public Overpass mirrors — any one throttles under load (and stays blocked
# for hours), so rotate: a query that fails on one is retried on the next. The order
# is shuffled per run so we don't always lean on the same instance first.
OVERPASS = [
    "https://overpass-api.de/api/interpreter",
    "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
    "https://overpass.osm.ch/api/interpreter",
]
random.shuffle(OVERPASS)


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
    json.dump(payload, open("/tmp/shore_post.json", "w"))
    out = subprocess.run(
        ["curl", "-s", "-m", "40", "-X", "POST", "-H", f"Authorization: Bearer {TOKEN}",
         "-H", "Content-Type: application/json", "-H", f"User-Agent: {UA}",
         "--data-binary", "@/tmp/shore_post.json", f"{SERVER}{path}"],
        capture_output=True, text=True, timeout=50,
    ).stdout
    return out


def overpass(q):
    """Run an Overpass query across the mirrors; return elements or None if all fail."""
    for ep in OVERPASS:
        try:
            r = subprocess.run(["curl", "-s", "-m", "30", "-X", "POST", "--data", f"data={q}", ep],
                               capture_output=True, text=True, timeout=40)
            if r.returncode == 0 and r.stdout:
                body = json.loads(r.stdout)
                # Overpass signals a soft failure with a `remark` + empty/absent elements;
                # that is NOT "no water" — fall through to the next mirror, don't poison.
                if "elements" not in body or (body.get("remark") and not body["elements"]):
                    continue
                return body["elements"]
        except Exception:
            continue
    return None


def shore_of(lat, lon):
    """(shore, water_body) or None when every Overpass mirror failed."""
    q = (f"[out:json][timeout:20];("
         f'way["natural"="water"](around:150,{lat},{lon});'
         f'relation["natural"="water"](around:150,{lat},{lon});'
         f'way["natural"="coastline"](around:400,{lat},{lon}););out tags 1;')
    els = overpass(q)
    if els is None:
        return None
    coast = any(e.get("tags", {}).get("natural") == "coastline" for e in els)
    water = next((e for e in els if e.get("tags", {}).get("natural") == "water"), None)
    if water:
        t = water.get("tags", {})
        wb = "meri" if (t.get("water") in ("bay", "lagoon") or coast) else "jarvi"
        return ("oma_ranta", wb)
    if coast:
        return ("oma_ranta", "meri")
    return ("ei_rantaa", None)


lanes = {}
for ctry in COUNTRIES:
    data = api_get(f"/api/listings?country={ctry}&limit=400")
    rows = [l for l in data.get("listings", [])
            if l.get("lat") is not None and l.get("lon") is not None and not l.get("shore")]
    # Freshest within a country first, so a brand-new ingest is classified before its tail.
    rows.sort(key=lambda l: l.get("last_seen") or l.get("id") or 0, reverse=True)
    lanes[ctry] = rows
# Round-robin across countries so no single market's backlog (e.g. Denmark's hundreds)
# starves the smaller ones (Norway's handful) — every market makes steady progress.
pending, its = [], {c: iter(v) for c, v in lanes.items()}
while its:
    for c in [c for c in COUNTRIES if c in its]:
        try:
            pending.append(next(its[c]))
        except StopIteration:
            del its[c]
print(f"listings needing shore: {len(pending)} {{{', '.join(f'{c}:{len(v)}' for c, v in lanes.items())}}}", flush=True)


def flush(batch):
    if batch:
        print("  POST:", api_post("/api/set-shore", {"updates": batch}).strip(), flush=True)


batch, ok, failed = [], 0, 0
for l in pending:
    res = shore_of(l["lat"], l["lon"])
    if res is None:
        failed += 1
    else:
        shore, wb = res
        batch.append({"id": l["id"], "shore": shore, "water_body": wb})
        ok += 1
    if len(batch) >= 25:
        flush(batch)
        batch = []
    time.sleep(1.1 + random.random() * 0.6)
flush(batch)
print(f"classified {ok}, overpass-failed {failed} (left pending)", flush=True)
