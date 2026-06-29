//! The Nordic country dimension. kontu is the abstraction over the whole Nordic
//! region; each country is one implementation behind it (a [`crate::cost`]
//! jurisdiction, a risk model, a listing portal, a market-data and geo source).
//! Finland is one member of this set, not a privileged base.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A supported Nordic market, by ISO-3166-1 alpha-2 code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Country {
    /// Finland (EU + euro).
    #[serde(rename = "FI")]
    Fi,
    /// Sweden (EU, SEK).
    #[serde(rename = "SE")]
    Se,
    /// Norway (EEA, NOK).
    #[serde(rename = "NO")]
    No,
    /// Denmark (EU, DKK).
    #[serde(rename = "DK")]
    Dk,
    /// Iceland (EEA, ISK).
    #[serde(rename = "IS")]
    Is,
}

impl Default for Country {
    fn default() -> Self {
        Country::Fi
    }
}

impl Country {
    /// Every country kontu covers, in display order.
    pub const ALL: [Country; 5] = [
        Country::Fi,
        Country::Se,
        Country::No,
        Country::Dk,
        Country::Is,
    ];

    /// The ISO-3166-1 alpha-2 code (`FI`, `SE`, `NO`, `DK`, `IS`).
    pub fn code(self) -> &'static str {
        match self {
            Country::Fi => "FI",
            Country::Se => "SE",
            Country::No => "NO",
            Country::Dk => "DK",
            Country::Is => "IS",
        }
    }

    /// English display name.
    pub fn name(self) -> &'static str {
        match self {
            Country::Fi => "Finland",
            Country::Se => "Sweden",
            Country::No => "Norway",
            Country::Dk => "Denmark",
            Country::Is => "Iceland",
        }
    }

    /// ISO-4217 currency code of the local money the source portals quote in.
    pub fn currency(self) -> &'static str {
        match self {
            Country::Fi => "EUR",
            Country::Se => "SEK",
            Country::No => "NOK",
            Country::Dk => "DKK",
            Country::Is => "ISK",
        }
    }

    /// Whether the local currency is the euro (drives whether prices need FX
    /// conversion for cross-country comparison).
    pub fn is_eurozone(self) -> bool {
        matches!(self, Country::Fi)
    }

    /// Whether the cost & risk models are calibrated with verified figures for
    /// this country yet. The cost/risk commands refuse uncalibrated countries
    /// rather than show a Finland-derived fallback as if it were local truth.
    pub fn cost_calibrated(self) -> bool {
        // All five Nordic markets are calibrated from docs/expansion/<c>.md. The
        // gate stays so a future, uncalibrated market can be added safely.
        matches!(
            self,
            Country::Fi | Country::Se | Country::No | Country::Dk | Country::Is
        )
    }

    /// Parse a country from an ISO code or English/endonym name (case-insensitive).
    /// Accepts the forms a user or a listing source is likely to supply.
    pub fn parse(s: &str) -> Option<Country> {
        match s.trim().to_lowercase().as_str() {
            "fi" | "fin" | "finland" | "suomi" => Some(Country::Fi),
            "se" | "swe" | "sweden" | "sverige" | "ruotsi" => Some(Country::Se),
            "no" | "nor" | "norway" | "norge" | "norja" => Some(Country::No),
            "dk" | "dnk" | "denmark" | "danmark" | "tanska" => Some(Country::Dk),
            "is" | "isl" | "iceland" | "ísland" | "island" | "islanti" => Some(Country::Is),
            _ => None,
        }
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_and_parse_round_trip() {
        for c in Country::ALL {
            assert_eq!(Country::parse(c.code()), Some(c));
            assert_eq!(Country::parse(c.name()), Some(c));
        }
    }

    #[test]
    fn parses_endonyms_and_finnish_exonyms() {
        assert_eq!(Country::parse("Sverige"), Some(Country::Se));
        assert_eq!(Country::parse("ruotsi"), Some(Country::Se));
        assert_eq!(Country::parse("Norge"), Some(Country::No));
        assert_eq!(Country::parse("ísland"), Some(Country::Is));
        assert_eq!(Country::parse("nonsense"), None);
    }

    #[test]
    fn serde_uses_iso_code() {
        let j = serde_json::to_string(&Country::Se).unwrap();
        assert_eq!(j, "\"SE\"");
        let c: Country = serde_json::from_str("\"NO\"").unwrap();
        assert_eq!(c, Country::No);
    }

    #[test]
    fn default_is_finland() {
        assert_eq!(Country::default(), Country::Fi);
    }
}
