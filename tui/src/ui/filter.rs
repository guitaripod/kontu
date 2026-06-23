use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::App;
use crate::ui::centered_rect;

pub fn draw(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let rect = centered_rect(60, 80, area);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Exact-parameter filter",
            Style::default().fg(t.header).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];
    for (label, value, selected) in app.filter_form.rows() {
        let label_style = if selected {
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        };
        let cursor = if selected { "▏" } else { " " };
        let shown = if value.is_empty() && !selected {
            "—".to_string()
        } else {
            value.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {label:<22}"), label_style),
            Span::styled(shown, t.base()),
            Span::styled(cursor.to_string(), t.accent_style()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        " Tab/↑↓ move · type to edit · ←/→ cycle options · Enter apply · Esc cancel",
        t.dimmed(),
    )));

    let block = Block::default()
        .title(" Filter ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.border_style())
        .title_style(t.title_style());
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}
