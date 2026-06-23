//! Color palette. A single dark theme for now; the `theme` config key selects it.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub accent_alt: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
    pub selection_bg: Color,
    pub border: Color,
    pub header: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::Rgb(0xd8, 0xd8, 0xd8),
            dim: Color::Rgb(0x80, 0x80, 0x88),
            accent: Color::Rgb(0x6c, 0xb6, 0xff),
            accent_alt: Color::Rgb(0xc9, 0x8a, 0xff),
            good: Color::Rgb(0x7d, 0xd6, 0x7d),
            warn: Color::Rgb(0xe6, 0xc3, 0x6a),
            bad: Color::Rgb(0xf2, 0x6d, 0x6d),
            selection_bg: Color::Rgb(0x26, 0x3a, 0x52),
            border: Color::Rgb(0x3a, 0x3a, 0x44),
            header: Color::Rgb(0x9a, 0xd8, 0xff),
        }
    }
}

impl Theme {
    pub fn base(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn dimmed(&self) -> Style {
        Style::default().fg(self.dim)
    }

    pub fn title_style(&self) -> Style {
        Style::default().fg(self.header).add_modifier(Modifier::BOLD)
    }

    pub fn selected(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn accent_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    /// Color for a risk band score 0..100.
    pub fn risk_color(&self, score: u32) -> Color {
        match score {
            0..=24 => self.good,
            25..=49 => self.warn,
            _ => self.bad,
        }
    }
}
