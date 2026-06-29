#!/usr/bin/env python3
"""Ingest cheap Icelandic houses from fasteignir.visir.is (open JSON API) -> Worker."""
import json, os, subprocess, time, tomllib
cfg = tomllib.load(open(os.path.expanduser("~/.config/kontu/config.toml"), "rb"))
SERVER, TOKEN = cfg["server_url"], cfg["api_token"]
FX = 144.0  # ISK per EUR
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
PTYPE = {"Einbýlishús": "detached", "Sumarhús": "leisure", "Sumarbústaður": "leisure",
         "Raðhús": "terraced", "Parhús": "semi", "Hæð": "apartment", "Fjölbýlishús": "apartment",
         "Jörð": "farm", "Lóð": "plot"}
# Rural / cheaper Iceland postcodes (outside the Reykjavík capital 101-132)
ZIPS = [200,210,220,225,230,235,240,245,250,260,270,300,310,320,340,355,360,370,380,
        400,410,415,420,425,430,450,460,465,470,510,524,530,540,550,560,565,580,
        600,601,610,620,625,630,640,641,650,660,670,675,680,690,700,710,720,730,735,740,
        750,755,760,765,780,785,800,801,810,815,820,825,840,845,850,860,870,880,900,902]
cheap = []
for z in ZIPS:
    try:
        r = subprocess.run(["curl","-s","-m","20","-H",f"User-Agent: {UA}",
                            f"https://fasteignir.visir.is/api/search?onpage=1000&page=1&zip={z}&stype=sale"],
                           capture_output=True, text=True, timeout=30)
        cards = json.loads(r.stdout)
    except Exception:
        continue
    if not isinstance(cards, list): continue
    for c in cards:
        price = c.get("price") or "0"
        try: price_isk = int(str(price).replace(" ", ""))
        except: continue
        if price_isk <= 0: continue
        price_eur = round(price_isk / FX)
        if price_eur > 100000: continue
        cat = c.get("category")
        try: m2 = float(str(c.get("size","")).replace(",", "."))
        except: m2 = None
        cheap.append({
            "portal":"visir","portal_listing_id":str(c.get("id")),
            "url":f"https://fasteignir.visir.is/property/{c.get('id')}","country":"IS",
            "property_type":PTYPE.get(cat,(cat or '').lower()),
            "address":c.get("street_name"),"municipality":(c.get("zip") or {}).get("town"),
            "postal_code":str((c.get("zip") or {}).get("zip") or z),
            "price_eur":price_eur,"living_area_m2":m2,
            "room_count":(lambda x:int(x) if str(x).isdigit() else None)(c.get("rooms")),
            "lat":(lambda x:float(x) if x else None)(c.get("latitude")),
            "lon":(lambda x:float(x) if x else None)(c.get("longitude")),
            "raw_json":json.dumps(c,ensure_ascii=False)[:3000],
        })
    time.sleep(0.6)
seen={r["portal_listing_id"]:r for r in cheap}; cheap=list(seen.values())
print(f"IS cheap (<=100k EUR) houses: {len(cheap)}")
def post(ls):
    json.dump({"listings":ls}, open("/tmp/visir_post.json","w"))
    return subprocess.run(["curl","-s","-m","60","-X","POST","-H",f"Authorization: Bearer {TOKEN}",
        "-H","Content-Type: application/json","-H",f"User-Agent: {UA}",
        "--data-binary","@/tmp/visir_post.json",f"{SERVER}/api/import-normalized"],
        capture_output=True,text=True,timeout=70).stdout
for i in range(0,len(cheap),50): print("  POST:", post(cheap[i:i+50]))
