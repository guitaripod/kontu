use ratatui::prelude::*;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use crate::app::App;
use crate::format::{area_opt, money_opt, num_opt, ppm2_opt};

pub fn draw(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.listings.is_empty() {
        let msg = if app.loading {
            "Loading listings…"
        } else {
            "No listings.  Press  y  to run a sync crawl, or  /  to adjust the filter."
        };
        frame.render_widget(
            Paragraph::new(msg)
                .style(app.theme.dimmed())
                .alignment(Alignment::Center),
            center_v(area),
        );
        return;
    }

    let t = app.theme.clone();
    let header = Row::new(
        [
            "", "Place", "Type", "Price", "€/m²", "m²", "rooms", "year", "risk", "dom",
        ]
        .into_iter()
        .map(|h| Cell::from(Span::styled(h, t.title_style()))),
    )
    .height(1);

    let compare = app.compare.clone();
    let rows: Vec<Row> = app
        .listings
        .iter()
        .map(|l| {
            let risk = app.risk_for(l);
            let marked = compare.contains(&l.id);
            let mark = if marked { "▌" } else { " " };
            let place = l.title();
            let kind = l.property_type.clone().unwrap_or_default();
            let dom = l
                .days_on_market
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".into());
            Row::new(vec![
                Cell::from(Span::styled(mark, t.accent_style())),
                Cell::from(truncate(&place, 26)),
                Cell::from(truncate(&kind, 11)),
                Cell::from(Line::from(money_opt(l.price_eur)).alignment(Alignment::Right)),
                Cell::from(Line::from(ppm2_opt(l.effective_ppm2())).alignment(Alignment::Right)),
                Cell::from(Line::from(area_opt(l.living_area_m2)).alignment(Alignment::Right)),
                Cell::from(Line::from(num_opt(l.room_count)).alignment(Alignment::Right)),
                Cell::from(Line::from(int_opt_year(l.year_built)).alignment(Alignment::Right)),
                Cell::from(
                    Line::from(Span::styled(
                        format!("{:>3}", risk.score),
                        Style::default().fg(t.risk_color(risk.score)),
                    ))
                    .alignment(Alignment::Right),
                ),
                Cell::from(Line::from(dom).alignment(Alignment::Right)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(5),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(t.selected())
        .highlight_symbol("");

    frame.render_stateful_widget(table, area, &mut app.table);
}

fn int_opt_year(y: Option<i32>) -> String {
    y.map(|y| y.to_string()).unwrap_or_else(|| "—".into())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

fn center_v(area: Rect) -> Rect {
    Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area)[1]
}
