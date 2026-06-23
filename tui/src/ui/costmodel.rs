use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Sparkline};

use crate::app::{App, COST_FIELDS};
use crate::format::money;

pub fn draw(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = app.theme.clone();
    let cols = Layout::horizontal([Constraint::Length(34), Constraint::Min(0)]).split(area);

    inputs(app, &t, frame, cols[0]);
    results(app, &t, frame, cols[1]);
}

fn inputs(app: &App, t: &crate::theme::Theme, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for i in 0..COST_FIELDS {
        let (label, value) = app.cost.field_label(i);
        let selected = i == app.cost.field;
        let (ls, vs) = if selected {
            (
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
            )
        } else {
            (t.dimmed(), t.base())
        };
        let marker = if selected { "›" } else { " " };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} {label:<16}"), ls),
            Span::styled(value, vs),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("↑↓ field · ←→ adjust", t.dimmed())));

    let block = Block::default()
        .title(" Inputs ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style())
        .title_style(t.title_style());
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn results(app: &App, t: &crate::theme::Theme, frame: &mut Frame, area: Rect) {
    let p = app.cost.project(&app.defaults);
    let ot = &p.one_time;

    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(7)]).split(area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Net present cost  ", t.dimmed()),
        Span::styled(
            money(p.npv_cost),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   ≈ {} / mo", money(p.equivalent_monthly)),
            t.base(),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("over {} years, real €, discounted at opportunity cost", app.cost.horizon),
        t.dimmed(),
    )));
    lines.push(Line::raw(""));

    section(&mut lines, t, "Up front");
    kv(&mut lines, t, "Down payment", money(ot.down_payment));
    kv(&mut lines, t, "Varainsiirtovero", money(ot.transfer_tax));
    kv(&mut lines, t, "Registration fees", money(ot.lainhuuto + ot.kaupanvahvistus + ot.kiinnitys));
    kv(&mut lines, t, "Inspection + moving", money(ot.inspection + ot.moving));
    kv(&mut lines, t, "Total up front", money(ot.total()));

    section(&mut lines, t, "Over the horizon");
    kv(&mut lines, t, "Loan interest", money(p.total_loan_interest));
    kv(&mut lines, t, "Loan principal", money(p.loan_principal));
    if let (Some(first), Some(last)) = (p.years.first(), p.years.last()) {
        kv(&mut lines, t, "Year 1 running", money(first.recurring));
        let last_label = format!("Year {} running", last.year);
        kv(&mut lines, t, &last_label, money(last.recurring));
    }
    kv(&mut lines, t, "Terminal equity", money(p.terminal_equity));

    let block = Block::default()
        .title(" Projection ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style())
        .title_style(t.title_style());
    frame.render_widget(Paragraph::new(lines).block(block), rows[0]);

    let data: Vec<u64> = p.years.iter().map(|y| y.total_nominal.max(0.0) as u64).collect();
    let spark = Sparkline::default()
        .block(
            Block::default()
                .title(" Yearly cash outflow ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(t.border_style())
                .title_style(t.dimmed()),
        )
        .data(&data)
        .style(t.accent_style());
    frame.render_widget(spark, rows[1]);
}

fn section(lines: &mut Vec<Line>, t: &crate::theme::Theme, title: &str) {
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(t.header).add_modifier(Modifier::BOLD),
    )));
}

fn kv(lines: &mut Vec<Line>, t: &crate::theme::Theme, label: &str, value: String) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {label:<20}"), t.dimmed()),
        Span::styled(value, t.base()),
    ]));
}
