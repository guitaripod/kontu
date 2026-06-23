use chrono::{TimeZone, Utc};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::format::{area_opt, int_opt, money, money_opt, num_opt, ppm2_opt, str_opt};

pub fn draw(app: &mut App, frame: &mut Frame, area: Rect) {
    let detail = match &app.detail {
        Some(d) => d,
        None => {
            frame.render_widget(
                Paragraph::new("Loading…").style(app.theme.dimmed()).alignment(Alignment::Center),
                area,
            );
            return;
        }
    };
    let l = &detail.listing;
    let theme = app.theme.clone();
    let t = &theme;

    let near_water = detail
        .dossier
        .as_ref()
        .and_then(|d| d.get("distance_to_water_m"))
        .and_then(|v| v.as_f64())
        .map(|m| m < 150.0)
        .unwrap_or(false);
    let risk = crate::risk::assess(&l.to_risk_input(near_water), 2026);
    let projection = app.cost.project(&app.defaults);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(l.title(), t.title_style()),
        Span::raw("  "),
        Span::styled(money_opt(l.price_eur), Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(Span::styled(
        format!(
            "{} · {} · {} · {}",
            str_opt(&l.property_type),
            str_opt(&l.municipality),
            str_opt(&l.holding_form),
            l.status
        ),
        t.dimmed(),
    )));
    lines.push(Line::raw(""));

    head(&mut lines, t, "Size & price");
    kv(&mut lines, t, "Living area", area_opt(l.living_area_m2));
    kv(&mut lines, t, "Plot area", area_opt(l.plot_area_m2));
    kv(&mut lines, t, "€/m²", ppm2_opt(l.effective_ppm2()));
    kv(&mut lines, t, "Debt-free price", money_opt(l.debt_free_price_eur));
    kv(&mut lines, t, "Rooms", format!("{}  {}", num_opt(l.room_count), str_opt(&l.room_layout)));

    head(&mut lines, t, "Building & condition");
    kv(&mut lines, t, "Year built", int_opt(l.year_built));
    kv(&mut lines, t, "Condition", str_opt(&l.condition_class));
    kv(&mut lines, t, "Frame / facade", format!("{} / {}", str_opt(&l.frame_material), str_opt(&l.facade_material)));
    kv(&mut lines, t, "Roof", str_opt(&l.roof_material));
    kv(&mut lines, t, "Energy class", str_opt(&l.energy_class));
    kv(&mut lines, t, "Inspection", str_opt(&l.inspection_status));

    head(&mut lines, t, "Plot & water");
    kv(&mut lines, t, "Plot ownership", str_opt(&l.plot_ownership));
    kv(&mut lines, t, "Shore", str_opt(&l.shore));
    kv(&mut lines, t, "Ground rent / yr", money_opt(l.ground_rent_eur_yr));
    kv(&mut lines, t, "Road access", str_opt(&l.road_access));

    head(&mut lines, t, "Heating & utilities");
    kv(&mut lines, t, "Heating", str_opt(&l.heating_type));
    kv(&mut lines, t, "Water / sewer", format!("{} / {}", str_opt(&l.water_supply), str_opt(&l.sewer_system)));
    kv(&mut lines, t, "Broadband", str_opt(&l.broadband));

    head(&mut lines, t, "Cost of ownership (press c to model)");
    kv(&mut lines, t, "Net present cost", money(projection.npv_cost));
    kv(&mut lines, t, "≈ per month", money(projection.equivalent_monthly));
    kv(&mut lines, t, "Transfer tax", money(projection.one_time.transfer_tax));
    kv(&mut lines, t, "Total interest", money(projection.total_loan_interest));

    head(&mut lines, t, "Risk assessment");
    lines.push(Line::from(vec![
        Span::styled("  RiskScore  ", t.dimmed()),
        Span::styled(
            format!("{} ({})", risk.score, risk.band.label()),
            Style::default().fg(t.risk_color(risk.score)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("   deferred capex ~{}", money(risk.deferred_capex_eur)), t.dimmed()),
    ]));
    for flag in &risk.flags {
        let capex = if flag.capex_eur > 0.0 {
            format!("  (~{})", money(flag.capex_eur))
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled("   • ", Style::default().fg(t.warn)),
            Span::styled(flag.label.clone(), t.base()),
            Span::styled(capex, t.dimmed()),
        ]));
    }
    if risk.flags.is_empty() {
        lines.push(Line::from(Span::styled("   no notable risk flags", t.dimmed())));
    }

    head(&mut lines, t, "Personal  (1–5 score · d deal-breaker · n note)");
    let score_str = detail
        .score
        .as_ref()
        .and_then(|s| s.score)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "—".into());
    kv(&mut lines, t, "Your score", score_str);
    if detail.score.as_ref().and_then(|s| s.deal_breaker).unwrap_or(false) {
        lines.push(Line::from(Span::styled("  ✗ deal-breaker", Style::default().fg(t.bad))));
    }
    if !detail.tags.is_empty() {
        kv(&mut lines, t, "Tags", detail.tags.join(", "));
    }
    kv(&mut lines, t, "Note", detail.note.clone().unwrap_or_else(|| "—".into()));

    if !detail.events.is_empty() {
        head(&mut lines, t, "History");
        for e in &detail.events {
            let when = fmt_date(e.observed_at);
            let text = match (e.old_price_eur, e.new_price_eur) {
                (Some(o), Some(n)) => format!("{}: {} → {}", e.kind, money(o as f64), money(n as f64)),
                _ => format!("{}: {}", e.kind, e.new_value.clone().unwrap_or_default()),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {when:<18}"), t.dimmed()),
                Span::styled(text, t.base()),
            ]));
        }
    }

    let photo_note = if detail.photos.is_empty() {
        "no cached photos".to_string()
    } else {
        format!("{} photos cached (press o to open the listing)", detail.photos.len())
    };
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(photo_note, t.dimmed())));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style());

    let text_area = if app.image.is_some() {
        let rows = Layout::vertical([Constraint::Length(16), Constraint::Min(0)]).split(area);
        if let Some(protocol) = app.image.as_mut() {
            frame.render_stateful_widget(ratatui_image::StatefulImage::new(), rows[0], protocol);
        }
        rows[1]
    } else {
        area
    };

    let body_area = if app.note_editing {
        let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(text_area);
        let editor = Block::default()
            .title(" Note — Enter saves · Esc cancels ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(t.accent_style());
        frame.render_widget(
            Paragraph::new(format!("{}▏", app.note_buffer)).block(editor),
            rows[1],
        );
        rows[0]
    } else {
        text_area
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0)),
        body_area,
    );
}

fn head(lines: &mut Vec<Line>, t: &crate::theme::Theme, title: &str) {
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(t.header).add_modifier(Modifier::BOLD),
    )));
}

fn kv(lines: &mut Vec<Line>, t: &crate::theme::Theme, label: &str, value: impl Into<String>) {
    lines.push(Line::from(vec![
        Span::styled(format!("  {label:<18}"), t.dimmed()),
        Span::styled(value.into(), t.base()),
    ]));
}

fn fmt_date(unix: i64) -> String {
    Utc.timestamp_opt(unix, 0)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "—".into())
}
