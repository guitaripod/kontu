import { describe, it, expect } from "vitest";
import {
  asciiFold,
  contentHash,
  etuoviPhotoUrls,
  extractRiskStructures,
  fingerprint,
  isForeignListing,
  normalizeEtuoviAnnouncement,
  normalizeOikotieCard,
  normalizePropertyType,
  oikotiePhotoUrls,
  toNumber,
} from "../src/normalize";

describe("asciiFold", () => {
  it("strips Finnish diacritics and lowercases", () => {
    expect(asciiFold("Ylämyllyntie")).toBe("ylamyllyntie");
    expect(asciiFold("Pöljä")).toBe("polja");
    expect(asciiFold("ÅLAND")).toBe("aland");
  });

  it("collapses whitespace and is total on null", () => {
    expect(asciiFold("  Kuusikko   tie  ")).toBe("kuusikko tie");
    expect(asciiFold(null)).toBe("");
    expect(asciiFold(undefined)).toBe("");
    expect(asciiFold(123)).toBe("123");
  });
});

describe("fingerprint", () => {
  it("is deterministic for identical input", () => {
    const a = fingerprint("83500", "Kuusikkotie", "12", 118, 4);
    const b = fingerprint("83500", "Kuusikkotie", "12", 118, 4);
    expect(a).toBe(b);
  });

  it("strips diacritics and lowercases components", () => {
    expect(fingerprint("83100", "Ylämyllyntie", "18", 142, 5)).toBe(
      "83100|ylamyllyntie|18|142|5",
    );
  });

  it("rounds m2", () => {
    expect(fingerprint("00100", "Tie", "1", 117.6, 3)).toBe("00100|tie|1|118|3");
    expect(fingerprint("00100", "Tie", "1", 117.4, 3)).toBe("00100|tie|1|117|3");
  });

  it("appends floor only when provided and non-empty", () => {
    expect(fingerprint("00100", "Tie", "1", 50, 2)).toBe("00100|tie|1|50|2");
    expect(fingerprint("00100", "Tie", "1", 50, 2, 4)).toBe("00100|tie|1|50|2|4");
    expect(fingerprint("00100", "Tie", "1", 50, 2, null)).toBe("00100|tie|1|50|2");
    expect(fingerprint("00100", "Tie", "1", 50, 2, "")).toBe("00100|tie|1|50|2");
  });

  it("treats å/ä/ö identically regardless of case", () => {
    expect(fingerprint("X", "Pöljä", "1", 10, 1)).toBe(
      fingerprint("X", "PÖLJÄ", "1", 10, 1),
    );
  });
});

describe("toNumber", () => {
  it("parses Finnish/euro formatted numbers", () => {
    expect(toNumber("142 000 €")).toBe(142000);
    expect(toNumber("1 203,39")).toBe(1203.39);
    expect(toNumber("1.234,56")).toBe(1234.56);
    expect(toNumber(98.5)).toBe(98.5);
  });

  it("returns null on garbage / missing", () => {
    expect(toNumber(null)).toBeNull();
    expect(toNumber("")).toBeNull();
    expect(toNumber("ei tiedossa")).toBeNull();
    expect(toNumber(true)).toBeNull();
  });
});

describe("extractRiskStructures", () => {
  it("finds known tokens in free text", () => {
    const text =
      "Rakennettu 1978, valesokkelirakenne, salaojat uusittava, kosteusvaurio kellarissa.";
    expect(extractRiskStructures(text)).toEqual(["valesokkeli", "kosteusvaurio", "salaoja"]);
  });

  it("dedupes and is order-stable", () => {
    const text = "valesokkeli valesokkeli ja asbesti";
    expect(extractRiskStructures(text)).toEqual(["valesokkeli", "asbesti"]);
  });

  it("returns empty array on no match or missing", () => {
    expect(extractRiskStructures("hyväkuntoinen uudiskohde")).toEqual([]);
    expect(extractRiskStructures(null)).toEqual([]);
    expect(extractRiskStructures(undefined)).toEqual([]);
  });
});

describe("normalizeOikotieCard", () => {
  const card = {
    id: 12345,
    url: "https://asunnot.oikotie.fi/myytavat-asunnot/12345",
    buildingType: "Omakotitalo",
    holdingType: "Kiinteistö",
    address: "Kuusikkotie 12",
    city: "Outokumpu",
    postalCode: "83500",
    price: "142 000 €",
    debtFreePrice: "142 000",
    size: "118",
    rooms: "4",
    roomConfiguration: "4h+k+s",
    buildYear: "1978",
    energyClass: "E",
    heating: "Öljylämmitys",
    waterfront: "Oma ranta",
    lotOwnership: "Oma",
    coordinates: { latitude: 62.7261, longitude: 29.0214 },
    description: "Omalla rannalla, valesokkeli ja salaojat.",
  };

  it("maps core fields", () => {
    const n = normalizeOikotieCard(card);
    expect(n.portal).toBe("oikotie");
    expect(n.portal_listing_id).toBe("12345");
    expect(n.property_type).toBe("omakotitalo");
    expect(n.holding_form).toBe("kiinteisto");
    expect(n.municipality).toBe("Outokumpu");
    expect(n.postal_code).toBe("83500");
    expect(n.price_eur).toBe(142000);
    expect(n.debt_free_price_eur).toBe(142000);
    expect(n.living_area_m2).toBe(118);
    expect(n.room_count).toBe(4);
    expect(n.room_layout).toBe("4h+k+s");
    expect(n.year_built).toBe(1978);
    expect(n.energy_class).toBe("E");
    expect(n.heating_type).toBe("oljy");
    expect(n.shore).toBe("oma_ranta");
    expect(n.plot_ownership).toBe("oma");
    expect(n.lat).toBe(62.7261);
    expect(n.lon).toBe(29.0214);
    expect(n.risk_structures).toEqual(["valesokkeli", "salaoja"]);
    expect(n.status).toBe("active");
  });

  it("never throws on empty/garbage input", () => {
    const empty = normalizeOikotieCard({});
    expect(empty.portal).toBe("oikotie");
    expect(empty.price_eur).toBeNull();
    expect(empty.risk_structures).toEqual([]);
    expect(() => normalizeOikotieCard(null)).not.toThrow();
    expect(() => normalizeOikotieCard("nonsense")).not.toThrow();
    expect(() => normalizeOikotieCard(42)).not.toThrow();
  });

  it("preserves raw payload as JSON", () => {
    const n = normalizeOikotieCard(card);
    expect(JSON.parse(n.raw_json)).toMatchObject({ id: 12345 });
  });
});

describe("normalizePropertyType", () => {
  it("maps Etuovi English propertySubtype enums to Finnish tokens", () => {
    expect(normalizePropertyType("DETACHED_HOUSE")).toBe("omakotitalo");
    expect(normalizePropertyType("SEPARATE_HOUSE")).toBe("erillistalo");
    expect(normalizePropertyType("SEMI_DETACHED_HOUSE")).toBe("paritalo");
    expect(normalizePropertyType("ROW_HOUSE")).toBe("rivitalo");
    expect(normalizePropertyType("COTTAGE")).toBe("mokki");
    expect(normalizePropertyType("APARTMENT_HOUSE")).toBe("kerrostalo");
  });

  it("still maps Finnish tokens and is total on empty", () => {
    expect(normalizePropertyType("Omakotitalo")).toBe("omakotitalo");
    expect(normalizePropertyType("")).toBeNull();
    expect(normalizePropertyType(null)).toBeNull();
  });
});

describe("normalizeEtuoviAnnouncement", () => {
  const announcement = {
    id: 7654321,
    friendlyId: "abc-987",
    propertySubtype: "ROW_HOUSE",
    holdingType: "Asunto-osake",
    addressLine1: "Niskakatu 9",
    addressLine2: "Keskusta Joensuu",
    postalCode: "80100",
    searchPrice: 168000,
    unencumberedSalesPrice: "172000",
    area: 72,
    totalArea: 78,
    roomCount: "3 huonetta",
    roomStructure: "3h+k+s",
    constructionFinishedYear: 2004,
    residentialFloorCount: 2,
    latitude: 62.601,
    longitude: 29.7636,
    energyClass: "C",
    heating: "Kaukolämpö",
    searchListItemText: "Hyväkuntoinen, kaukolämpö, ei rantaa.",
  };

  it("maps the live listpage fields", () => {
    const n = normalizeEtuoviAnnouncement(announcement);
    expect(n.portal).toBe("etuovi");
    expect(n.portal_listing_id).toBe("abc-987");
    expect(n.url).toBe("https://www.etuovi.com/kohde/abc-987");
    expect(n.property_type).toBe("rivitalo");
    expect(n.holding_form).toBe("asunto_osake");
    expect(n.address).toBe("Niskakatu 9");
    expect(n.municipality).toBe("Joensuu");
    expect(n.district).toBe("Keskusta");
    expect(n.postal_code).toBe("80100");
    expect(n.price_eur).toBe(168000);
    expect(n.debt_free_price_eur).toBe(172000);
    expect(n.living_area_m2).toBe(72);
    expect(n.total_area_m2).toBe(78);
    expect(n.room_count).toBe(3);
    expect(n.room_layout).toBe("3h+k+s");
    expect(n.floors).toBe(2);
    expect(n.year_built).toBe(2004);
    expect(n.lat).toBe(62.601);
    expect(n.lon).toBe(29.7636);
    expect(n.energy_class).toBe("C");
    expect(n.heating_type).toBe("kaukolampo");
    expect(n.description).toBe("Hyväkuntoinen, kaukolämpö, ei rantaa.");
  });

  it("uses friendlyId (not the numeric id) for the listing id and url", () => {
    const n = normalizeEtuoviAnnouncement({ id: 999, friendlyId: "x-1" });
    expect(n.portal_listing_id).toBe("x-1");
    expect(n.url).toBe("https://www.etuovi.com/kohde/x-1");
  });

  it("derives municipality from the last token of addressLine2", () => {
    expect(normalizeEtuoviAnnouncement({ friendlyId: "z", addressLine2: "Kilo Espoo" }).municipality).toBe(
      "Espoo",
    );
    expect(normalizeEtuoviAnnouncement({ friendlyId: "z", addressLine2: "Kilo Espoo" }).district).toBe(
      "Kilo",
    );
    const single = normalizeEtuoviAnnouncement({ friendlyId: "z", addressLine2: "Helsinki" });
    expect(single.municipality).toBe("Helsinki");
    expect(single.district).toBeNull();
  });

  it("keeps multi-word kunta suffixes together for municipality", () => {
    expect(
      normalizeEtuoviAnnouncement({ friendlyId: "z", addressLine2: "Sandbacka Pedersören kunta" })
        .municipality,
    ).toBe("Pedersören kunta");
    expect(normalizeEtuoviAnnouncement({ friendlyId: "z", addressLine2: "Koski Tl" }).municipality).toBe(
      "Koski Tl",
    );
  });

  it("maps a price-on-request (searchPrice 0) to null, not 0", () => {
    const n = normalizeEtuoviAnnouncement({ friendlyId: "z", searchPrice: 0, pricePerSquareMeter: 0 });
    expect(n.price_eur).toBeNull();
    expect(n.price_per_m2).toBeNull();
  });

  it("treats a sub-€1000 placeholder price (€1 auction/by-offer) as price-on-request", () => {
    expect(normalizeEtuoviAnnouncement({ friendlyId: "z", searchPrice: 1 }).price_eur).toBeNull();
    expect(normalizeOikotieCard({ id: "z", price: "1 €" }).price_eur).toBeNull();
    expect(normalizeOikotieCard({ id: "z", price: "25 000 €" }).price_eur).toBe(25000);
  });

  it("never throws on empty/garbage input", () => {
    expect(() => normalizeEtuoviAnnouncement(null)).not.toThrow();
    expect(() => normalizeEtuoviAnnouncement({})).not.toThrow();
    expect(() => normalizeEtuoviAnnouncement([])).not.toThrow();
  });
});

describe("oikotie buildingType bitmask", () => {
  const typeOf = (code: number) =>
    normalizeOikotieCard({ id: "x", buildingData: { buildingType: code } }).property_type;
  it("maps the live-verified codes (post-canonicalization)", () => {
    expect(typeOf(1)).toBe("kerrostalo");
    expect(typeOf(2)).toBe("rivitalo");
    expect(typeOf(4)).toBe("omakotitalo");
    // 8 was wrongly 'luhtitalo' before the fix; 64 was unmapped → description fallback
    expect(typeOf(8)).toBe("mokki");
    expect(typeOf(64)).toBe("paritalo");
    // 256 was wrongly 'mökki' before the fix
    expect(typeOf(256)).toBe("luhtitalo");
    // erillistalo canonicalizes to omakotitalo for filtering
    expect(typeOf(32)).toBe("omakotitalo");
  });
});

describe("cover photo extraction", () => {
  it("returns the Oikotie card cover (already absolute https)", () => {
    const u = "https://cdn.asunnot.oikotie.fi/abc/623x464/x.jpg";
    expect(oikotiePhotoUrls({ images: { wide: u } })).toEqual([u]);
    expect(oikotiePhotoUrls({})).toEqual([]);
  });
  it("resolves the Etuovi protocol-relative URI with the size placeholder", () => {
    const out = etuoviPhotoUrls({
      mainImageUri: "//d3ls91xgksobn.cloudfront.net/{imageParameters}/etuovimedia/x/ORIGINAL.jpeg",
    });
    expect(out).toEqual([
      "https://d3ls91xgksobn.cloudfront.net/1600x1066/etuovimedia/x/ORIGINAL.jpeg",
    ]);
    expect(etuoviPhotoUrls({ mainImageUri: "//x/y.jpg", mainImageHidden: true })).toEqual([]);
    expect(etuoviPhotoUrls({})).toEqual([]);
  });
});

describe("isForeignListing", () => {
  it("drops non-Finnish listings, keeps Finnish ones", () => {
    expect(isForeignListing("Viro", null)).toBe(true); // Etuovi Estonian
    expect(isForeignListing("Helsinki", "Philippines")).toBe(true); // Oikotie foreign country
    expect(isForeignListing("Espanja", null)).toBe(true);
    expect(isForeignListing("Outokumpu", "Suomi")).toBe(false);
    expect(isForeignListing("Järvenpää", null)).toBe(false); // Etuovi Finnish, no country
    expect(isForeignListing("Helsinki", "Finnland")).toBe(false); // German spelling of Finland
  });
});

describe("contentHash", () => {
  it("is stable for identical normalized fields", () => {
    const a = normalizeOikotieCard({ id: 1, price: "100000", city: "Kuopio", size: "80", rooms: "3" });
    const b = normalizeOikotieCard({ id: 1, price: "100000", city: "Kuopio", size: "80", rooms: "3" });
    expect(contentHash(a)).toBe(contentHash(b));
  });

  it("changes when a tracked field changes", () => {
    const a = normalizeOikotieCard({ id: 1, price: "100000", city: "Kuopio", size: "80", rooms: "3" });
    const b = normalizeOikotieCard({ id: 1, price: "110000", city: "Kuopio", size: "80", rooms: "3" });
    expect(contentHash(a)).not.toBe(contentHash(b));
  });

  it("ignores untracked fields like url", () => {
    const a = normalizeOikotieCard({ id: 1, price: "100000", url: "https://a", size: "80", rooms: "3", city: "X" });
    const b = normalizeOikotieCard({ id: 1, price: "100000", url: "https://b", size: "80", rooms: "3", city: "X" });
    expect(contentHash(a)).toBe(contentHash(b));
  });

  it("produces an 8-char hex string", () => {
    const n = normalizeOikotieCard({ id: 1, price: "100000" });
    expect(contentHash(n)).toMatch(/^[0-9a-f]{8}$/);
  });
});
