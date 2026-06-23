//! Rendering. `draw` lays out the chrome (header/footer/overlays) and dispatches
//! the body to the active screen.

pub mod compare;
pub mod costmodel;
pub mod detail;
pub mod filter;
pub mod list;

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::action::Screen;
use crate::app::App;

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    header(app, frame, chunks[0]);
    match app.screen {
        Screen::List => list::draw(app, frame, chunks[1]),
        Screen::Detail => detail::draw(app, frame, chunks[1]),
        Screen::Filter => filter::draw(app, frame, chunks[1]),
        Screen::CostModel => costmodel::draw(app, frame, chunks[1]),
        Screen::Compare => compare::draw(app, frame, chunks[1]),
    }
    footer(app, frame, chunks[2]);

    if let Some((msg, err)) = app.toast.clone() {
        toast(app, frame, &msg, err);
    }
    if app.help_visible {
        help(app, frame);
    }
}

fn header(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let cols = Layout::horizontal([Constraint::Min(0), Constraint::Length(28)]).split(area);

    let mut left = vec![
        Span::styled(" kontu ", Style::default().fg(t.bg).bg(t.accent).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(app.screen.title(), t.title_style()),
    ];
    if let Some(f) = filter_summary(app) {
        left.push(Span::styled("  ·  ", t.dimmed()));
        left.push(Span::styled(f, t.accent_style()));
    }
    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);

    let spin = if app.loading {
        SPINNER[app.spinner % SPINNER.len()]
    } else {
        " "
    };
    let right = Line::from(vec![
        Span::styled(format!("{} ", app.listings.len()), t.base()),
        Span::styled("listings ", t.dimmed()),
        Span::styled(format!("{spin} "), t.accent_style()),
    ])
    .alignment(Alignment::Right);
    frame.render_widget(Paragraph::new(right), cols[1]);
}

fn filter_summary(app: &App) -> Option<String> {
    let f = &app.filter;
    let mut parts = Vec::new();
    if let Some(m) = &f.municipality {
        parts.push(m.clone());
    }
    if let Some(t) = &f.property_type {
        parts.push(t.clone());
    }
    if let Some(p) = f.price_max {
        parts.push(format!("≤{}k", p / 1000));
    }
    if let Some(s) = &f.shore {
        parts.push(s.clone());
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn footer(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let keys: &[(&str, &str)] = match app.screen {
        Screen::List => &[
            ("↑↓", "move"),
            ("⏎", "detail"),
            ("c", "cost"),
            ("/", "filter"),
            ("s", "sort"),
            ("space", "mark"),
            ("v", "compare"),
            ("o", "open"),
            ("y", "sync"),
            ("?", "help"),
            ("q", "quit"),
        ],
        Screen::Detail => &[("↑↓", "scroll"), ("c", "cost"), ("o", "open site"), ("q", "back")],
        Screen::Filter => &[("tab", "field"), ("←→/type", "edit"), ("⏎", "apply"), ("esc", "cancel")],
        Screen::CostModel => &[("↑↓", "field"), ("←→", "adjust"), ("q", "back")],
        Screen::Compare => &[("x", "clear"), ("q", "back")],
    };
    let mut spans = Vec::new();
    for (k, label) in keys {
        spans.push(Span::styled(format!(" {k} "), Style::default().fg(t.bg).bg(t.dim)));
        spans.push(Span::styled(format!(" {label}  "), t.dimmed()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn toast(app: &App, frame: &mut Frame, msg: &str, err: bool) {
    let t = &app.theme;
    let area = frame.area();
    let w = (msg.len() as u16 + 4).min(area.width.saturating_sub(2));
    let rect = Rect {
        x: area.width.saturating_sub(w + 1),
        y: area.height.saturating_sub(3),
        width: w,
        height: 3,
    };
    let color = if err { t.bad } else { t.good };
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));
    frame.render_widget(
        Paragraph::new(msg.to_string())
            .style(Style::default().fg(color))
            .block(block)
            .wrap(Wrap { trim: true }),
        rect,
    );
}

fn help(app: &App, frame: &mut Frame) {
    let t = &app.theme;
    let area = centered_rect(64, 78, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" kontu — help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style())
        .title_style(t.title_style());
    let lines = vec![
        section("Listings"),
        keyline("↑/↓ j/k", "move selection"),
        keyline("g / G", "first / last"),
        keyline("Enter / l", "open detail"),
        keyline("c", "cost-of-ownership model for the selected house"),
        keyline("/ or f", "edit exact-parameter filter"),
        keyline("s", "cycle sort column"),
        keyline("space", "mark/unmark for comparison"),
        keyline("v", "side-by-side compare marked"),
        keyline("o", "open listing on its source site"),
        keyline("r / y", "refresh / trigger a sync crawl"),
        Line::raw(""),
        section("Cost model"),
        keyline("↑/↓", "select an input"),
        keyline("←/→", "decrease / increase it"),
        Line::raw(""),
        section("Filter"),
        keyline("Tab / ↑↓", "move between fields"),
        keyline("type / ←→", "edit text-number / cycle enum"),
        keyline("Enter", "apply and return"),
        Line::raw(""),
        section("Global"),
        keyline("?", "toggle this help"),
        keyline("Ctrl-c / q", "quit"),
    ];
    frame.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: true }), area);
}

fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(Color::Rgb(0x9a, 0xd8, 0xff)).add_modifier(Modifier::BOLD),
    ))
}

fn keyline(keys: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {keys:<12}"), Style::default().fg(Color::Rgb(0x6c, 0xb6, 0xff))),
        Span::styled(desc.to_string(), Style::default().fg(Color::Rgb(0xd8, 0xd8, 0xd8))),
    ])
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}
