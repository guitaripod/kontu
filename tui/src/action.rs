//! Messages that drive state changes. Key handling lives in the screens (which
//! may mutate `App` directly for screen-local state); cross-cutting effects —
//! navigation, async results, global commands — flow through [`Action`].

use crate::cost::CostDefaults;
use crate::models::{ListingDetail, Listing};

/// The top-level screen currently in focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    List,
    Detail,
    Filter,
    CostModel,
    Compare,
}

impl Screen {
    pub fn title(&self) -> &'static str {
        match self {
            Screen::List => "Listings",
            Screen::Detail => "Detail",
            Screen::Filter => "Filter",
            Screen::CostModel => "Cost of ownership",
            Screen::Compare => "Compare",
        }
    }
}

#[derive(Debug)]
pub enum Action {
    Quit,
    Render,
    Tick,
    Refresh,
    Sync,
    Navigate(Screen),
    OpenDetail(i64),
    ListingsLoaded(Vec<Listing>),
    DetailLoaded(Box<ListingDetail>),
    PhotoLoaded(Vec<u8>),
    CostDefaultsLoaded(Box<CostDefaults>),
    Toast(String),
    Error(String),
}
