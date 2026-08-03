use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Padding},
};

use crate::{model::RiskLevel, tui::app::AppLogLevel};

pub(super) const SURFACE: Color = Color::Rgb(28, 31, 44);
const SURFACE_RAISED: Color = Color::Rgb(42, 47, 62);
pub(super) const FOOTER_SURFACE: Color = Color::Rgb(22, 25, 35);
pub(super) const BORDER: Color = Color::Rgb(92, 101, 135);
const TEXT: Color = Color::Rgb(206, 212, 236);
const TEXT_MUTED: Color = Color::Rgb(134, 143, 177);
pub(super) const TEXT_STRONG: Color = Color::Rgb(190, 198, 230);
pub(super) const ACCENT: Color = Color::Rgb(112, 208, 178);
pub(super) const ACCENT_SOFT: Color = Color::Rgb(190, 236, 220);
pub(super) const WARNING: Color = Color::Rgb(245, 215, 132);
pub(super) const DANGER: Color = Color::Rgb(239, 112, 138);
const RISK_LOW: Color = Color::Rgb(130, 198, 167);
const RISK_MEDIUM: Color = Color::Rgb(221, 185, 112);
const RISK_HIGH: Color = Color::Rgb(231, 137, 111);

pub(super) fn panel_block(title: &'static str) -> Block<'static> {
    Block::bordered()
        .title(title)
        .title_style(
            Style::default()
                .fg(TEXT_STRONG)
                .add_modifier(Modifier::BOLD),
        )
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(SURFACE))
        .padding(Padding::horizontal(1))
}

pub(super) fn focused_panel_block(title: &'static str) -> Block<'static> {
    panel_block(title).border_style(Style::default().fg(ACCENT))
}

pub(super) fn panel_style() -> Style {
    Style::default().fg(TEXT).bg(SURFACE)
}

pub(super) fn muted_style() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub(super) fn accent_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub(super) fn warning_style() -> Style {
    Style::default().fg(WARNING).add_modifier(Modifier::BOLD)
}

pub(super) fn error_style() -> Style {
    Style::default()
        .fg(DANGER)
        .bg(SURFACE)
        .add_modifier(Modifier::BOLD)
}

pub(super) fn selected_row_style() -> Style {
    Style::default().bg(SURFACE_RAISED)
}

pub(super) fn app_log_level_style(level: AppLogLevel) -> Style {
    match level {
        AppLogLevel::Info => muted_style(),
        AppLogLevel::Warning => warning_style(),
        AppLogLevel::Error => error_style(),
    }
}

pub(super) fn risk_style(risk: &RiskLevel) -> Style {
    match risk {
        RiskLevel::Low => Style::default().fg(RISK_LOW),
        RiskLevel::Medium => Style::default().fg(RISK_MEDIUM),
        RiskLevel::High => Style::default().fg(RISK_HIGH).add_modifier(Modifier::BOLD),
        RiskLevel::Dangerous => Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
    }
}
