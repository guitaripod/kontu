#!/usr/bin/env python3
"""Ingest cheap Swedish houses from Booli (residential IP) -> Worker.
Parses Booli's __NEXT_DATA__ Apollo cache, normalizes, filters <=100k EUR, POSTs."""
import re, json, os, time, subprocess, tomllib, random

cfg = tomllib.load(open(os.path.expanduser("~/.config/kontu/config.toml"), "rb"))
SERVER, TOKEN = cfg["server_url"], cfg["api_token"]
FX = 11.3  # SEK per EUR
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"

PTYPE = {"Villa": "detached", "Fritidshus": "leisure", "Radhus": "terraced", "Parhus": "semi",
         "Kedjehus": "terraced", "Lägenhet": "apartment", "Gård": "farm", "Tomt": "plot"}
TENURE = {"Äganderätt": "kiinteisto", "Bostadsrätt": "asunto_osake", "Tomträtt": "kiinteisto"}

def fetch(url):
    r = subprocess.run(["curl", "-s", "-m", "25", "--http2", "-H", f"User-Agent: {UA}",
                        "-H", "Accept-Language: sv-SE,sv;q=0.9", "-H", "Accept: text/html",
                        "-H", 'sec-ch-ua: "Chromium";v="126"', "-H", 'sec-ch-ua-platform: "Linux"', url],
                       capture_output=True, text=True, timeout=35)
    return r.stdout

def num_from(txt, unit):
    m = re.search(r"([\d\s]+)\s*" + unit, txt or "")
    return int(re.sub(r"\s", "", m.group(1))) if m else None

def parse_page(html):
    m = re.search(r'<script id="__NEXT_DATA__"[^>]*>(.*?)</script>', html, re.S)
    if not m: return [], 0
    apollo = json.loads(m.group(1))["props"]["pageProps"]["__APOLLO_STATE__"]
    sfs = next((v for k, v in apollo["ROOT_QUERY"].items() if k.startswith("searchForSale(")), None)
    if not sfs: return [], 0
    total = sfs.get("totalCount", 0)
    out = []
    for ref in sfs.get("result", []):
        L = apollo.get(ref.get("__ref")) if isinstance(ref, dict) else None
        if not L or not isinstance(L.get("listPrice"), dict): continue
        raw = L["listPrice"].get("raw")
        if not raw: continue
        price_eur = round(raw / FX)
        da = next((v for k, v in L.items() if k.startswith("displayAttributes")), {})
        pts = " · ".join(p.get("value", {}).get("plainText", "") for p in da.get("dataPoints", []))
        muni = (L.get("location") or {}).get("region", {}).get("municipalityName")
        sqm = (L.get("listSqmPrice") or {}).get("formatted")
        ppm2 = num_from(sqm, "kr") if sqm else None
        url = L.get("url") or ""
        if url.startswith("/"): url = "https://www.booli.se" + url
        out.append({
            "portal": "booli", "portal_listing_id": str(L.get("booliId") or L.get("id")),
            "url": url, "country": "SE",
            "property_type": PTYPE.get(L.get("objectType"), (L.get("objectType") or "").lower()),
            "holding_form": TENURE.get(L.get("tenureForm")),
            "address": L.get("streetAddress"), "municipality": muni,
            "district": L.get("descriptiveAreaName"),
            "price_eur": price_eur, "price_per_m2": round(ppm2 / FX) if ppm2 else None,
            "living_area_m2": num_from(pts, "m²"), "room_count": num_from(pts, "rum"),
            "plot_area_m2": num_from(pts.split("tomt")[0] if "tomt" in pts else "", "m²"),
            "lat": L.get("latitude"), "lon": L.get("longitude"),
            "raw_json": json.dumps(L, ensure_ascii=False)[:4000],
        })
    return out, total

def post(listings):
    with open("/tmp/booli_post.json", "w") as f:
        json.dump({"listings": listings}, f)
    r = subprocess.run(["curl", "-s", "-m", "60", "-X", "POST", "-H", f"Authorization: Bearer {TOKEN}",
                        "-H", "Content-Type: application/json", "-H", f"User-Agent: {UA}",
                        "--data-binary", "@/tmp/booli_post.json", f"{SERVER}/api/import-normalized"],
                       capture_output=True, text=True, timeout=70)
    return r.stdout

cheap = []
for otype in ("Fritidshus", "Villa"):
    for page in range(1, 6):  # radar-light: newest pages
        html = fetch(f"https://www.booli.se/sok/till-salu?objectType={otype}&page={page}")
        rows, total = parse_page(html)
        if not rows:
            print(f"  {otype} p{page}: 0 rows (blocked or empty) — stop"); break
        c = [r for r in rows if r["price_eur"] and r["price_eur"] <= 100000]
        cheap += c
        print(f"  {otype} p{page}: {len(rows)} rows, {len(c)} <=100k (total {total})")
        time.sleep(1.5 + random.random())
# dedup by id
seen = {}
for r in cheap: seen[r["portal_listing_id"]] = r
cheap = list(seen.values())
print(f"\nTotal cheap (<=100k EUR) SE listings: {len(cheap)}")
if cheap:
    for i in range(0, len(cheap), 50):
        print("  POST:", post(cheap[i:i+50]))
