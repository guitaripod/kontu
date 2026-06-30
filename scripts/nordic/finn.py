#!/usr/bin/env python3
"""Ingest cheap Norwegian houses from FINN.no (server-rendered HTML cards) -> Worker."""
import re, json, os, subprocess, time, random, tomllib
cfg = tomllib.load(open(os.path.expanduser("~/.config/kontu/config.toml"), "rb"))
SERVER, TOKEN = cfg["server_url"], cfg["api_token"]
FX = 11.2  # NOK per EUR
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"
PTYPE = [("enebolig","detached"),("tomannsbolig","semi"),("vertikaldelt","semi"),("rekkehus","terraced"),
         ("leilighet","apartment"),("gårdsbruk","farm"),("småbruk","farm"),("hytte","leisure"),
         ("fritidsbolig","leisure"),("fritidstomt","plot"),("tomt","plot")]
def ptype(text):
    t=text.lower()
    for k,v in PTYPE:
        if k in t: return v
    return None
def tenure(text):
    t=text.lower()
    if "selveier" in t: return "kiinteisto"
    if "andel" in t or "aksje" in t or "borettslag" in t: return "asunto_osake"
    return None
def fetch(url):
    return subprocess.run(["curl","-sL","-m","25","--http2","-H",f"User-Agent: {UA}",
        "-H","Accept-Language: nb-NO,nb;q=0.9","-H",'sec-ch-ua-platform: "Linux"',url],
        capture_output=True,text=True,timeout=35).stdout
def parse(html):
    out=[]
    for block in re.split(r'<article', html)[1:]:
        mk=re.search(r'finnkode=(\d+)', block)
        if not mk: continue
        fk=mk.group(1)
        text=re.sub(r'\s+',' ',re.sub(r'<[^>]+>',' ',block)).strip()
        ma=re.search(r'(\d+)\s*m²', text)
        mp=re.search(r'(\d[\d  ]{4,})\s*kr', text)
        if not mp: continue
        mimg=re.search(r'https://images\.finncdn\.no/dynamic/[^"\\ ]+\.(?:jpg|jpeg|png|webp)', block)
        price_nok=int(re.sub(r'[\s ]','',mp.group(1)))
        price_eur=round(price_nok/FX)
        if price_eur>100000 or price_eur<2000: continue
        mb=re.search(r'(\d+)\s*soverom', text)
        # place = token sequence right before the m²/price; use heading's last place-ish word
        place=None
        mpl=re.search(r'([A-ZÆØÅ][\wæøåÆØÅ\- ]{2,30}?)\s+\d+\s*m²', text)
        if mpl: place=mpl.group(1).strip().split()[-1]
        out.append({
            "portal":"finn","portal_listing_id":fk,
            "url":f"https://www.finn.no/realestate/leisuresale/ad.html?finnkode={fk}","country":"NO",
            "property_type":ptype(text),"holding_form":tenure(text),
            "municipality":place,"price_eur":price_eur,
            "living_area_m2":int(ma.group(1)) if ma else None,
            "room_count":(int(mb.group(1))+1) if mb else None,
            "raw_json":text[:1500],
            "photo_urls":[mimg.group(0)] if mimg else [],
        })
    return out
cheap=[]
for vert in ("leisuresale","homes"):
    for page in range(1,5):  # radar-light
        html=fetch(f"https://www.finn.no/realestate/{vert}/search.html?page={page}")
        rows=parse(html)
        c=[r for r in rows if r["property_type"] not in ("plot",None)]
        cheap+=c
        print(f"  {vert} p{page}: {len(rows)} cards parsed, {len(c)} cheap houses")
        if not rows: break
        time.sleep(1.5+random.random())
seen={r["portal_listing_id"]:r for r in cheap}; cheap=list(seen.values())
print(f"\nNO cheap (<=100k EUR) houses: {len(cheap)}")

# FINN search cards carry no coordinates; the per-ad detail page embeds them in the
# static-map URL (lat=..&lon=..). Pull them so the Worker's geometric shore detection
# (OSM Overpass on lat/lon) can find Norway's lake/fjord shores — without coords a NO
# listing can never surface as a waterfront candidate. Best-effort + paced to stay
# human; a miss leaves coords null and the COALESCE upsert keeps any prior value.
def detail_coords(url):
    h=fetch(url).replace('\\u0026','&')
    m=re.search(r'lat=(-?\d+\.\d+)&lon=(-?\d+\.\d+)&zoom=', h)
    return (float(m.group(1)), float(m.group(2))) if m else (None, None)
got=0
for r in cheap:
    lat,lon=detail_coords(r["url"])
    if lat is not None:
        r["lat"],r["lon"]=lat,lon; got+=1
    time.sleep(1.0+random.random())
print(f"  geocoded {got}/{len(cheap)} from detail pages")
def post(ls):
    json.dump({"listings":ls},open("/tmp/finn_post.json","w"))
    return subprocess.run(["curl","-s","-m","60","-X","POST","-H",f"Authorization: Bearer {TOKEN}",
        "-H","Content-Type: application/json","-H",f"User-Agent: {UA}",
        "--data-binary","@/tmp/finn_post.json",f"{SERVER}/api/import-normalized"],
        capture_output=True,text=True,timeout=70).stdout
for i in range(0,len(cheap),50): print("  POST:", post(cheap[i:i+50]))
