//! The application state machine and async run loop.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::action::{Action, Screen};
use crate::api::KontuClient;
use crate::config::Config;
use crate::cost::{
    CostDefaults, HeatingType, HoldingForm, Projection, RepaymentType, WaterSupply,
};
use crate::models::{FilterState, Listing, ListingDetail};
use crate::risk::{self, RiskAssessment};
use crate::theme::Theme;
use crate::tui::{Event, Tui};
use crate::{cost, ui};

/// Interactive cost-of-ownership model state (adjustable on the Cost screen).
#[derive(Debug, Clone)]
pub struct CostState {
    pub listing_id: Option<i64>,
    pub price: f64,
    pub debt_free_price: f64,
    pub holding_form: HoldingForm,
    pub ltv: f64,
    pub term_years: u32,
    pub euribor: f64,
    pub margin: f64,
    pub repayment: RepaymentType,
    pub heating: HeatingType,
    pub water: WaterSupply,
    pub building_value: Option<f64>,
    pub fireplace: bool,
    pub private_road: bool,
    pub ground_rent: f64,
    pub horizon: u32,
    pub real_discount: f64,
    pub general_inflation: f64,
    pub energy_inflation: f64,
    pub resale_growth: f64,
    pub capex: Vec<(u32, f64)>,
    pub field: usize,
}

pub const COST_FIELDS: usize = 9;

impl CostState {
    pub fn from_defaults(d: &CostDefaults) -> Self {
        Self {
            listing_id: None,
            price: 150_000.0,
            debt_free_price: 150_000.0,
            holding_form: HoldingForm::Kiinteisto,
            ltv: d.ltv_max,
            term_years: d.loan_term_years,
            euribor: d.euribor_12m,
            margin: d.mortgage_margin,
            repayment: RepaymentType::Annuiteetti,
            heating: HeatingType::Kaukolampo,
            water: WaterSupply::Municipal,
            building_value: None,
            fireplace: false,
            private_road: false,
            ground_rent: 0.0,
            horizon: 20,
            real_discount: d.discount_rate_real,
            general_inflation: d.general_inflation,
            energy_inflation: d.energy_inflation,
            resale_growth: d.resale_real_growth,
            capex: Vec::new(),
            field: 0,
        }
    }

    /// Seed the model from a listing and its risk assessment.
    pub fn apply_listing(&mut self, l: &Listing, risk: &RiskAssessment, d: &CostDefaults) {
        self.listing_id = Some(l.id);
        if let Some(p) = l.price_eur {
            self.price = p as f64;
        }
        self.debt_free_price = l.debt_free_price_eur.map(|v| v as f64).unwrap_or(self.price);
        self.holding_form = match l.holding_form.as_deref() {
            Some(h) if h.contains("osake") => HoldingForm::AsuntoOsake,
            _ => HoldingForm::Kiinteisto,
        };
        self.heating = l.heating_enum();
        self.water = match l.water_supply.as_deref() {
            Some(w) if w.contains("kaivo") || w.contains("kanto") => WaterSupply::Well,
            _ => WaterSupply::Municipal,
        };
        self.ground_rent = l.ground_rent_eur_yr.map(|v| v as f64).unwrap_or(0.0);
        self.private_road = l
            .road_access
            .as_deref()
            .map(|r| r.contains("yksityis"))
            .unwrap_or(false);
        self.capex = risk.capex_events();
        let _ = d;
    }

    pub fn project(&self, d: &CostDefaults) -> Projection {
        let purchase = cost::PurchaseInputs {
            price_eur: self.price,
            debt_free_price_eur: self.debt_free_price,
            holding_form: self.holding_form,
            ltv: self.ltv,
            term_years: self.term_years,
            interest_rate: self.euribor + self.margin,
            repayment: self.repayment,
            rate_path: None,
            arrangement_fee_eur: 0.0,
            moving_eur: 1000.0,
            inspection_eur: d.kuntotarkastus_eur,
            mortgage_deeds: 1,
            e_conveyance: false,
        };
        let property = cost::PropertyInputs {
            heating: self.heating,
            water: self.water,
            building_value_eur: self.building_value,
            fireplace: self.fireplace,
            private_road: self.private_road,
            ground_rent_eur_yr: self.ground_rent,
            vastike_eur_mo: 0.0,
            kiinteistovero_eur_yr: None,
            insurance_eur_yr: None,
            electricity_eur_yr: None,
            capex: self.capex.clone(),
        };
        let model = cost::ModelInputs {
            horizon_years: self.horizon,
            real_discount_rate: self.real_discount,
            general_inflation: self.general_inflation,
            energy_inflation: self.energy_inflation,
            resale_real_growth: self.resale_growth,
            seller_commission_pct: d.seller_commission_pct,
        };
        cost::project(&purchase, &property, &model, d)
    }

    pub fn field_label(&self, i: usize) -> (&'static str, String) {
        use crate::format::{money, pct};
        match i {
            0 => ("Price", money(self.price)),
            1 => ("Loan-to-value", pct(self.ltv)),
            2 => ("Term", format!("{} yr", self.term_years)),
            3 => ("Euribor (12 mo)", pct(self.euribor)),
            4 => ("Margin", pct(self.margin)),
            5 => ("Repayment", self.repayment_label().into()),
            6 => ("Heating", self.heating_label().into()),
            7 => ("Horizon", format!("{} yr", self.horizon)),
            8 => ("Discount (real)", pct(self.real_discount)),
            _ => ("", String::new()),
        }
    }

    fn repayment_label(&self) -> &'static str {
        match self.repayment {
            RepaymentType::Tasalyhennys => "tasalyhennys",
            RepaymentType::Annuiteetti => "annuiteetti",
            RepaymentType::KiinteaTasaera => "kiinteä tasaerä",
        }
    }

    fn heating_label(&self) -> &'static str {
        match self.heating {
            HeatingType::Kaukolampo => "kaukolämpö",
            HeatingType::Maalampo => "maalämpö",
            HeatingType::Oljy => "öljy",
            HeatingType::Sahko => "sähkö",
            HeatingType::Puu => "puu",
            HeatingType::IlmaLampopumppu => "ilmalämpöpumppu",
        }
    }

    /// Nudge the selected field. `dir` is +1 / -1.
    pub fn adjust(&mut self, dir: f64) {
        match self.field {
            0 => self.price = (self.price + dir * 5_000.0).max(10_000.0),
            1 => self.ltv = (self.ltv + dir * 0.05).clamp(0.0, 0.95),
            2 => self.term_years = ((self.term_years as i64 + dir as i64).clamp(5, 40)) as u32,
            3 => self.euribor = (self.euribor + dir * 0.0025).clamp(0.0, 0.15),
            4 => self.margin = (self.margin + dir * 0.001).clamp(0.0, 0.05),
            5 => self.repayment = cycle_repayment(self.repayment, dir > 0.0),
            6 => self.heating = cycle_heating(self.heating, dir > 0.0),
            7 => self.horizon = ((self.horizon as i64 + dir as i64).clamp(1, 40)) as u32,
            8 => self.real_discount = (self.real_discount + dir * 0.005).clamp(0.0, 0.12),
            _ => {}
        }
    }
}

fn cycle_repayment(r: RepaymentType, fwd: bool) -> RepaymentType {
    let order = [
        RepaymentType::Annuiteetti,
        RepaymentType::Tasalyhennys,
        RepaymentType::KiinteaTasaera,
    ];
    cycle(&order, r, fwd)
}

fn cycle_heating(h: HeatingType, fwd: bool) -> HeatingType {
    let order = [
        HeatingType::Kaukolampo,
        HeatingType::Maalampo,
        HeatingType::IlmaLampopumppu,
        HeatingType::Puu,
        HeatingType::Oljy,
        HeatingType::Sahko,
    ];
    cycle(&order, h, fwd)
}

fn cycle<T: Copy + PartialEq>(order: &[T], cur: T, fwd: bool) -> T {
    let i = order.iter().position(|x| *x == cur).unwrap_or(0);
    let n = order.len();
    let next = if fwd { (i + 1) % n } else { (i + n - 1) % n };
    order[next]
}

pub struct App {
    pub client: KontuClient,
    pub defaults: CostDefaults,
    pub theme: Theme,
    pub screen: Screen,
    pub help_visible: bool,
    pub should_quit: bool,

    pub listings: Vec<Listing>,
    pub table: TableState,
    pub loading: bool,
    pub spinner: usize,

    pub filter: FilterState,
    pub filter_form: FilterForm,
    pub sort: crate::models::SortColumn,
    pub sort_desc: bool,

    pub detail: Option<ListingDetail>,
    pub detail_scroll: u16,

    pub compare: Vec<i64>,
    pub cost: CostState,

    pub toast: Option<(String, bool)>,
    pub toast_ticks: u8,

    action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
}

impl App {
    pub fn new(client: KontuClient, _config: &Config) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let defaults = CostDefaults::default();
        let mut table = TableState::default();
        table.select(Some(0));
        Self {
            client,
            cost: CostState::from_defaults(&defaults),
            defaults,
            theme: Theme::default(),
            screen: Screen::List,
            help_visible: false,
            should_quit: false,
            listings: Vec::new(),
            table,
            loading: true,
            spinner: 0,
            filter: FilterState::default(),
            filter_form: FilterForm::new(),
            sort: crate::models::SortColumn::Price,
            sort_desc: false,
            detail: None,
            detail_scroll: 0,
            compare: Vec::new(),
            toast: None,
            toast_ticks: 0,
            action_tx,
            action_rx,
        }
    }

    pub async fn run(&mut self, tui: &mut Tui) -> Result<()> {
        self.spawn_defaults();
        self.send(Action::Refresh);
        while !self.should_quit {
            let Some(event) = tui.next().await else { break };
            match event {
                Event::Render | Event::Resize(_, _) => self.draw(tui)?,
                Event::Tick => self.tick(),
                Event::Key(key) => {
                    if let Some(action) = self.on_key(key) {
                        self.send(action);
                    }
                }
            }
            while let Ok(action) = self.action_rx.try_recv() {
                self.update(action);
            }
        }
        Ok(())
    }

    fn send(&self, action: Action) {
        let _ = self.action_tx.send(action);
    }

    fn draw(&mut self, tui: &mut Tui) -> Result<()> {
        tui.terminal.draw(|frame| ui::draw(self, frame))?;
        Ok(())
    }

    fn tick(&mut self) {
        if self.loading {
            self.spinner = self.spinner.wrapping_add(1);
        }
        if self.toast_ticks > 0 {
            self.toast_ticks -= 1;
            if self.toast_ticks == 0 {
                self.toast = None;
            }
        }
    }

    fn toast(&mut self, msg: impl Into<String>, error: bool) {
        self.toast = Some((msg.into(), error));
        self.toast_ticks = 12;
    }

    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Render | Action::Tick => {}
            Action::Refresh => {
                self.loading = true;
                self.spawn_listings();
            }
            Action::Sync => {
                self.toast("Triggering sync…", false);
                self.spawn_sync();
            }
            Action::Navigate(screen) => self.screen = screen,
            Action::OpenDetail(id) => {
                self.screen = Screen::Detail;
                self.detail = None;
                self.detail_scroll = 0;
                self.spawn_detail(id);
            }
            Action::ListingsLoaded(listings) => {
                self.listings = listings;
                self.loading = false;
                let sel = self.table.selected().unwrap_or(0);
                if sel >= self.listings.len() {
                    self.table
                        .select((!self.listings.is_empty()).then(|| self.listings.len() - 1));
                } else if self.table.selected().is_none() && !self.listings.is_empty() {
                    self.table.select(Some(0));
                }
            }
            Action::DetailLoaded(detail) => {
                self.cost_from_detail(&detail);
                self.detail = Some(*detail);
            }
            Action::CostDefaultsLoaded(d) => {
                self.defaults = *d;
            }
            Action::Toast(m) => self.toast(m, false),
            Action::Error(e) => self.toast(e, true),
        }
    }

    fn cost_from_detail(&mut self, detail: &ListingDetail) {
        let near_water = detail
            .dossier
            .as_ref()
            .and_then(|d| d.get("distance_to_water_m"))
            .and_then(|v| v.as_f64())
            .map(|m| m < 150.0)
            .unwrap_or(false);
        let assessment = risk::assess(&detail.listing.to_risk_input(near_water), 2026);
        self.cost.apply_listing(&detail.listing, &assessment, &self.defaults);
    }

    pub fn selected_listing(&self) -> Option<&Listing> {
        self.table.selected().and_then(|i| self.listings.get(i))
    }

    pub fn risk_for(&self, listing: &Listing) -> RiskAssessment {
        risk::assess(&listing.to_risk_input(false), 2026)
    }

    fn spawn_listings(&self) {
        let client = self.client.clone();
        let tx = self.action_tx.clone();
        let filter = self.filter.clone();
        let sort = self.sort;
        let desc = self.sort_desc;
        tokio::spawn(async move {
            match client.list_listings(&filter, sort, desc, 300, 0).await {
                Ok(page) => {
                    let _ = tx.send(Action::ListingsLoaded(page.listings));
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("listings: {e}")));
                }
            }
        });
    }

    fn spawn_detail(&self, id: i64) {
        let client = self.client.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match client.get_listing(id).await {
                Ok(d) => {
                    let _ = tx.send(Action::DetailLoaded(Box::new(d)));
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("detail: {e}")));
                }
            }
        });
    }

    fn spawn_defaults(&self) {
        let client = self.client.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            if let Ok(d) = client.cost_defaults().await {
                let _ = tx.send(Action::CostDefaultsLoaded(Box::new(d)));
            }
        });
    }

    fn spawn_sync(&self) {
        let client = self.client.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            match client.trigger_sync().await {
                Ok(_) => {
                    let _ = tx.send(Action::Toast("Sync done".into()));
                    let _ = tx.send(Action::Refresh);
                }
                Err(e) => {
                    let _ = tx.send(Action::Error(format!("sync: {e}")));
                }
            }
        });
    }

    fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }
        if self.help_visible {
            self.help_visible = false;
            return None;
        }
        if key.code == KeyCode::Char('?') {
            self.help_visible = true;
            return None;
        }
        match self.screen {
            Screen::List => self.on_key_list(key),
            Screen::Detail => self.on_key_detail(key),
            Screen::Filter => self.on_key_filter(key),
            Screen::CostModel => self.on_key_cost(key),
            Screen::Compare => self.on_key_compare(key),
        }
    }

    fn on_key_list(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.table.select((!self.listings.is_empty()).then_some(0));
                None
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.table
                    .select((!self.listings.is_empty()).then(|| self.listings.len() - 1));
                None
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                self.selected_listing().map(|l| Action::OpenDetail(l.id))
            }
            KeyCode::Char('/') | KeyCode::Char('f') => {
                self.filter_form = FilterForm::from_filter(&self.filter);
                Some(Action::Navigate(Screen::Filter))
            }
            KeyCode::Char('c') => {
                if let Some(l) = self.selected_listing() {
                    let risk = self.risk_for(l);
                    let l = l.clone();
                    self.cost.apply_listing(&l, &risk, &self.defaults);
                }
                Some(Action::Navigate(Screen::CostModel))
            }
            KeyCode::Char(' ') => {
                self.toggle_compare();
                None
            }
            KeyCode::Char('v') => {
                if self.compare.is_empty() {
                    self.toast("Mark listings with <space> first", true);
                    None
                } else {
                    Some(Action::Navigate(Screen::Compare))
                }
            }
            KeyCode::Char('s') => {
                self.cycle_sort();
                Some(Action::Refresh)
            }
            KeyCode::Char('r') => Some(Action::Refresh),
            KeyCode::Char('y') => Some(Action::Sync),
            KeyCode::Char('o') => {
                self.open_selected_in_browser();
                None
            }
            _ => None,
        }
    }

    fn on_key_detail(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Backspace => {
                Some(Action::Navigate(Screen::List))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
                None
            }
            KeyCode::Char('c') => Some(Action::Navigate(Screen::CostModel)),
            KeyCode::Char('o') => {
                if let Some(d) = &self.detail {
                    let _ = open::that_detached(&d.listing.url);
                }
                None
            }
            _ => None,
        }
    }

    fn on_key_filter(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => Some(Action::Navigate(Screen::List)),
            KeyCode::Enter => {
                self.filter = self.filter_form.to_filter();
                self.send(Action::Refresh);
                Some(Action::Navigate(Screen::List))
            }
            KeyCode::Tab | KeyCode::Down => {
                self.filter_form.next();
                None
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.filter_form.prev();
                None
            }
            _ => {
                self.filter_form.input(key);
                None
            }
        }
    }

    fn on_key_cost(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                Some(Action::Navigate(Screen::List))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.cost.field = (self.cost.field + 1) % COST_FIELDS;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cost.field = (self.cost.field + COST_FIELDS - 1) % COST_FIELDS;
                None
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') => {
                self.cost.adjust(1.0);
                None
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
                self.cost.adjust(-1.0);
                None
            }
            _ => None,
        }
    }

    fn on_key_compare(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                Some(Action::Navigate(Screen::List))
            }
            KeyCode::Char('x') => {
                self.compare.clear();
                Some(Action::Navigate(Screen::List))
            }
            _ => None,
        }
    }

    fn move_selection(&mut self, delta: i64) {
        if self.listings.is_empty() {
            return;
        }
        let len = self.listings.len() as i64;
        let cur = self.table.selected().unwrap_or(0) as i64;
        let next = (cur + delta).rem_euclid(len);
        self.table.select(Some(next as usize));
    }

    fn toggle_compare(&mut self) {
        if let Some(l) = self.selected_listing() {
            let id = l.id;
            if let Some(pos) = self.compare.iter().position(|&x| x == id) {
                self.compare.remove(pos);
            } else {
                self.compare.push(id);
            }
        }
    }

    fn cycle_sort(&mut self) {
        use crate::models::SortColumn::*;
        let order = [Price, PricePerM2, SizeM2, YearBuilt, DaysOnMarket, RiskScore];
        let i = order.iter().position(|s| *s == self.sort).unwrap_or(0);
        self.sort = order[(i + 1) % order.len()];
    }

    fn open_selected_in_browser(&mut self) {
        if let Some(l) = self.selected_listing() {
            let url = l.url.clone();
            match open::that_detached(&url) {
                Ok(_) => self.toast("Opened in browser", false),
                Err(e) => self.toast(format!("open failed: {e}"), true),
            }
        }
    }

    pub fn compared_listings(&self) -> Vec<&Listing> {
        self.compare
            .iter()
            .filter_map(|id| self.listings.iter().find(|l| l.id == *id))
            .collect()
    }
}

// ---- Filter form ----

#[derive(Debug, Clone)]
enum FieldKind {
    Text,
    Number,
    Enum(&'static [&'static str]),
}

#[derive(Debug, Clone)]
struct FilterField {
    label: &'static str,
    key: &'static str,
    value: String,
    kind: FieldKind,
}

#[derive(Debug, Clone)]
pub struct FilterForm {
    fields: Vec<FilterField>,
    pub index: usize,
}

impl FilterForm {
    pub fn new() -> Self {
        Self {
            index: 0,
            fields: vec![
                FilterField { label: "Municipality", key: "municipality", value: String::new(), kind: FieldKind::Text },
                FilterField { label: "Type", key: "property_type", value: String::new(), kind: FieldKind::Enum(&["", "omakotitalo", "paritalo", "rivitalo", "kerrostalo", "mökki"]) },
                FilterField { label: "Holding", key: "holding_form", value: String::new(), kind: FieldKind::Enum(&["", "kiinteisto", "asunto_osake"]) },
                FilterField { label: "Price min", key: "price_min", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "Price max", key: "price_max", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "m² min", key: "m2_min", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "Rooms min", key: "rooms_min", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "Year min", key: "year_min", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "Shore", key: "shore", value: String::new(), kind: FieldKind::Enum(&["", "oma_ranta", "rantaoikeus", "ei_rantaa"]) },
                FilterField { label: "Heating", key: "heating_type", value: String::new(), kind: FieldKind::Text },
                FilterField { label: "Plot", key: "plot_ownership", value: String::new(), kind: FieldKind::Enum(&["", "oma", "vuokra"]) },
                FilterField { label: "Max days on market", key: "max_days_on_market", value: String::new(), kind: FieldKind::Number },
                FilterField { label: "Text search", key: "text", value: String::new(), kind: FieldKind::Text },
            ],
        }
    }

    pub fn from_filter(f: &FilterState) -> Self {
        let mut form = Self::new();
        let set = |form: &mut Self, key: &str, v: String| {
            if let Some(field) = form.fields.iter_mut().find(|x| x.key == key) {
                field.value = v;
            }
        };
        if let Some(v) = &f.municipality {
            set(&mut form, "municipality", v.clone());
        }
        if let Some(v) = &f.property_type {
            set(&mut form, "property_type", v.clone());
        }
        if let Some(v) = f.price_max {
            set(&mut form, "price_max", v.to_string());
        }
        if let Some(v) = &f.shore {
            set(&mut form, "shore", v.clone());
        }
        form
    }

    pub fn rows(&self) -> impl Iterator<Item = (&'static str, &str, bool)> + '_ {
        self.fields
            .iter()
            .enumerate()
            .map(move |(i, f)| (f.label, f.value.as_str(), i == self.index))
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.fields.len();
    }

    pub fn prev(&mut self) {
        self.index = (self.index + self.fields.len() - 1) % self.fields.len();
    }

    fn input(&mut self, key: KeyEvent) {
        let field = &mut self.fields[self.index];
        match (&field.kind, key.code) {
            (FieldKind::Enum(opts), KeyCode::Left) | (FieldKind::Enum(opts), KeyCode::Char('h')) => {
                let cur = opts.iter().position(|o| *o == field.value).unwrap_or(0);
                field.value = opts[(cur + opts.len() - 1) % opts.len()].to_string();
            }
            (FieldKind::Enum(opts), KeyCode::Right)
            | (FieldKind::Enum(opts), KeyCode::Char('l'))
            | (FieldKind::Enum(opts), KeyCode::Char(' ')) => {
                let cur = opts.iter().position(|o| *o == field.value).unwrap_or(0);
                field.value = opts[(cur + 1) % opts.len()].to_string();
            }
            (FieldKind::Number, KeyCode::Char(c)) if c.is_ascii_digit() => field.value.push(c),
            (FieldKind::Text, KeyCode::Char(c)) => field.value.push(c),
            (_, KeyCode::Backspace) => {
                field.value.pop();
            }
            _ => {}
        }
    }

    pub fn to_filter(&self) -> FilterState {
        let mut f = FilterState::default();
        for field in &self.fields {
            let v = field.value.trim();
            if v.is_empty() {
                continue;
            }
            match field.key {
                "municipality" => f.municipality = Some(v.to_string()),
                "property_type" => f.property_type = Some(v.to_string()),
                "holding_form" => f.holding_form = Some(v.to_string()),
                "price_min" => f.price_min = v.parse().ok(),
                "price_max" => f.price_max = v.parse().ok(),
                "m2_min" => f.m2_min = v.parse().ok(),
                "rooms_min" => f.rooms_min = v.parse().ok(),
                "year_min" => f.year_min = v.parse().ok(),
                "shore" => f.shore = Some(v.to_string()),
                "heating_type" => f.heating_type = Some(v.to_string()),
                "plot_ownership" => f.plot_ownership = Some(v.to_string()),
                "max_days_on_market" => f.max_days_on_market = v.parse().ok(),
                "text" => f.text = Some(v.to_string()),
                _ => {}
            }
        }
        f
    }
}

impl Default for FilterForm {
    fn default() -> Self {
        Self::new()
    }
}
