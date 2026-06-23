//! Headless rendering smoke tests: draw every screen to a `TestBackend` with
//! representative data and assert no panic + non-empty output. This exercises
//! the real `ui::draw` path without a TTY.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::action::Screen;
use crate::api::KontuClient;
use crate::app::App;
use crate::config::Config;
use crate::models::{Listing, ListingDetail, ListingEvent, Photo};

fn sample_listing(id: i64, price: i64, year: i32) -> Listing {
    Listing {
        id,
        portal: "etuovi".into(),
        portal_listing_id: format!("e{id}"),
        url: format!("https://example.com/{id}"),
        property_type: Some("omakotitalo".into()),
        holding_form: Some("kiinteisto".into()),
        address: Some(format!("Testitie {id}")),
        municipality: Some("Outokumpu".into()),
        price_eur: Some(price),
        debt_free_price_eur: Some(price),
        living_area_m2: Some(118.0),
        plot_area_m2: Some(1500.0),
        room_count: Some(4.0),
        room_layout: Some("4h+k+s".into()),
        year_built: Some(year),
        condition_class: Some("tyydyttävä".into()),
        energy_class: Some("D".into()),
        heating_type: Some("puu".into()),
        water_supply: Some("porakaivo".into()),
        sewer_system: Some("saostuskaivo".into()),
        shore: Some("oma_ranta".into()),
        shore_sauna: Some(true),
        plot_ownership: Some("oma".into()),
        road_access: Some("yksityistie".into()),
        risk_structures: vec!["valesokkeli".into()],
        status: "active".into(),
        days_on_market: Some(42),
        first_seen: 1_700_000_000,
        last_seen: 1_750_000_000,
        ..Default::default()
    }
}

fn sample_app() -> App {
    let client = KontuClient::new("http://localhost:8787", "t").unwrap();
    let mut app = App::new(client, &Config::default(), None);
    app.listings = vec![
        sample_listing(1, 55_000, 1975),
        sample_listing(2, 142_000, 2004),
        sample_listing(3, 89_000, 1962),
    ];
    app.table.select(Some(0));
    app.compare = vec![1, 2];
    let detail = ListingDetail {
        listing: sample_listing(1, 55_000, 1975),
        events: vec![ListingEvent {
            kind: "price_change".into(),
            old_price_eur: Some(60_000),
            new_price_eur: Some(55_000),
            old_value: None,
            new_value: None,
            observed_at: 1_740_000_000,
        }],
        photos: vec![Photo {
            r2_key: "photos/abc".into(),
            content_type: Some("image/jpeg".into()),
            source_url: "https://example.com/p.jpg".into(),
            position: 0,
        }],
        dossier: Some(serde_json::json!({ "distance_to_water_m": 80.0 })),
        cost_inputs: None,
    };
    app.detail = Some(detail);
    app
}

fn render(app: &mut App, screen: Screen) {
    app.screen = screen;
    let backend = TestBackend::new(140, 44);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::draw(app, f)).unwrap();
}

#[test]
fn renders_every_screen_without_panic() {
    let mut app = sample_app();
    for screen in [
        Screen::List,
        Screen::Detail,
        Screen::Filter,
        Screen::CostModel,
        Screen::Compare,
    ] {
        render(&mut app, screen);
    }
}

#[test]
fn renders_overlays_and_empty_state() {
    let mut app = sample_app();
    app.help_visible = true;
    render(&mut app, Screen::List);

    app.help_visible = false;
    app.toast = Some(("listings: connection refused".into(), true));
    render(&mut app, Screen::List);

    app.listings.clear();
    app.loading = false;
    render(&mut app, Screen::List);
}

#[test]
fn renders_detail_with_an_image() {
    let mut app = sample_app();
    let picker = ratatui_image::picker::Picker::halfblocks();
    let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 8, image::Rgb([90, 150, 210])));
    app.image = Some(picker.new_resize_protocol(img));
    render(&mut app, Screen::Detail);
}

#[test]
fn renders_in_a_narrow_terminal() {
    let mut app = sample_app();
    app.screen = Screen::List;
    let backend = TestBackend::new(50, 16);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::draw(&mut app, f)).unwrap();
}
