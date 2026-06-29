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
OVERPASS = "https://overpass-api.de/api/interpreter"


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


def shore_of(lat, lon):
    """(shore, water_body) or None when the Overpass query itself failed."""
    q = (f"[out:json][timeout:20];("
         f'way["natural"="water"](around:150,{lat},{lon});'
         f'relation["natural"="water"](around:150,{lat},{lon});'
         f'way["natural"="coastline"](around:400,{lat},{lon}););out tags 1;')
    r = subprocess.run(["curl", "-s", "-m", "30", "-X", "POST", "--data", f"data={q}", OVERPASS],
                       capture_output=True, text=True, timeout=40)
    if r.returncode != 0 or not r.stdout:
        return None
    try:
        els = json.loads(r.stdout).get("elements", [])
    except Exception:
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


pending = []
for ctry in COUNTRIES:
    data = api_get(f"/api/listings?country={ctry}&limit=400")
    for l in data.get("listings", []):
        if l.get("lat") is not None and l.get("lon") is not None and not l.get("shore"):
            pending.append(l)
# Freshest first: a brand-new ingest gets classified before the long backlog tail, and
# the partial work survives if Overpass starts throttling partway through.
pending.sort(key=lambda l: l.get("last_seen") or l.get("id") or 0, reverse=True)
print(f"listings needing shore (coords, no shore): {len(pending)}", flush=True)


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
