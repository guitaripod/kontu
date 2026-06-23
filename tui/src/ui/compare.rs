use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table};

use crate::app::{App, CostState};
use crate::format::{area_opt, int_opt, money, money_opt, num_opt, ppm2_opt, str_opt};
use crate::models::Listing;

const MAX_COLS: usize = 4;

pub fn draw(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let listings = app.compared_listings();
    if listings.is_empty() {
        frame.render_widget(
            Paragraph::new("Nothing marked. Press space on listings, then v.")
                .style(t.dimmed())
                .alignment(Alignment::Center),
            area,
        );
        return;
    }
    let shown: Vec<&Listing> = listings.into_iter().take(MAX_COLS).collect();

    let modelled: Vec<(f64, f64, u32)> = shown
        .iter()
        .map(|l| {
            let mut cs = CostState::from_defaults(&app.defaults);
            let risk = app.risk_for(l);
            cs.apply_listing(l, &risk, &app.defaults);
            let p = cs.project(&app.defaults);
            (p.npv_cost, p.equivalent_monthly, risk.score)
        })
        .collect();

    let attrs: Vec<(&str, Box<dyn Fn(usize, &Listing) -> String>)> = vec![
        ("Place", Box::new(|_, l: &Listing| l.title())),
        ("Type", Box::new(|_, l: &Listing| str_opt(&l.property_type))),
        ("Price", Box::new(|_, l: &Listing| money_opt(l.price_eur))),
        ("€/m²", Box::new(|_, l: &Listing| ppm2_opt(l.effective_ppm2()))),
        ("m²", Box::new(|_, l: &Listing| area_opt(l.living_area_m2))),
        ("Rooms", Box::new(|_, l: &Listing| num_opt(l.room_count))),
        ("Year", Box::new(|_, l: &Listing| int_opt(l.year_built))),
        ("Energy", Box::new(|_, l: &Listing| str_opt(&l.energy_class))),
        ("Heating", Box::new(|_, l: &Listing| str_opt(&l.heating_type))),
        ("Shore", Box::new(|_, l: &Listing| str_opt(&l.shore))),
        ("Plot", Box::new(|_, l: &Listing| str_opt(&l.plot_ownership))),
    ];

    let mut rows: Vec<Row> = Vec::new();
    for (label, getter) in &attrs {
        let mut cells = vec![Cell::from(Span::styled(*label, t.dimmed()))];
        for (i, l) in shown.iter().enumerate() {
            cells.push(Cell::from(getter(i, l)));
        }
        rows.push(Row::new(cells));
    }

    rows.push(Row::new(vec![Cell::from("")]));
    let mut risk_cells = vec![Cell::from(Span::styled("RiskScore", t.dimmed()))];
    for (_, _, score) in &modelled {
        risk_cells.push(Cell::from(Span::styled(
            score.to_string(),
            Style::default().fg(t.risk_color(*score)),
        )));
    }
    rows.push(Row::new(risk_cells));

    let mut npv_cells = vec![Cell::from(Span::styled("Modelled NPV", t.dimmed()))];
    for (npv, _, _) in &modelled {
        npv_cells.push(Cell::from(Span::styled(
            money(*npv),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )));
    }
    rows.push(Row::new(npv_cells));

    let mut mo_cells = vec![Cell::from(Span::styled("≈ / month", t.dimmed()))];
    for (_, mo, _) in &modelled {
        mo_cells.push(Cell::from(money(*mo)));
    }
    rows.push(Row::new(mo_cells));

    let mut widths = vec![Constraint::Length(14)];
    widths.extend(std::iter::repeat_n(Constraint::Fill(1), shown.len()));

    let mut header_cells = vec![Cell::from("")];
    for (i, _) in shown.iter().enumerate() {
        header_cells.push(Cell::from(Span::styled(format!("#{}", i + 1), t.title_style())));
    }

    let block = Block::default()
        .title(format!(" Compare ({} marked) ", app.compare.len()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style())
        .title_style(t.title_style());

    let table = Table::new(rows, widths).header(Row::new(header_cells)).block(block);
    frame.render_widget(table, area);
}
