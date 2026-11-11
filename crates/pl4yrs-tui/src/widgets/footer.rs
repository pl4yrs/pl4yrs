use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget};

use crate::app::ConnectionStatus;

pub struct FooterWidget<'a> {
    connection: Option<&'a ConnectionStatus>,
    has_status: bool,
    error: Option<&'a str>,
}

impl<'a> FooterWidget<'a> {
    pub fn new(
        connection: Option<&'a ConnectionStatus>,
        has_status: bool,
        error: Option<&'a str>,
    ) -> Self {
        Self {
            connection,
            has_status,
            error,
        }
    }

    fn connection_badge(&self) -> (&'static str, Color) {
        match self.connection {
            Some(ConnectionStatus::Connected) => (" CONNECTED ", Color::Green),
            Some(ConnectionStatus::Connecting) if self.has_status => (" RETRYING ", Color::Yellow),
            Some(ConnectionStatus::Connecting) => (" CONNECTING ", Color::Yellow),
            Some(ConnectionStatus::Disconnected(_)) => (" OFFLINE ", Color::Red),
            None => (" STARTING ", Color::Gray),
        }
    }

    fn detail(&self) -> Line<'_> {
        let muted = Style::default().fg(Color::Gray);

        match self.connection {
            Some(ConnectionStatus::Disconnected(message)) => {
                let reason = if message.trim().is_empty() {
                    "Connection lost"
                } else {
                    message.as_str()
                };

                Line::styled(reason, Style::default().fg(Color::LightRed))
            }
            Some(ConnectionStatus::Connecting) => {
                if self.has_status {
                    Line::styled("Reconnecting...", muted)
                } else {
                    Line::styled("Connecting to backend...", muted)
                }
            }
            Some(ConnectionStatus::Connected) => {
                if let Some(error) = self.error.filter(|error| !error.trim().is_empty()) {
                    Line::from(vec![
                        Span::styled("Warning: ", Style::default().fg(Color::Yellow)),
                        Span::raw(error),
                    ])
                } else if !self.has_status {
                    Line::styled("Waiting for player status...", muted)
                } else {
                    Line::default()
                }
            }
            None => Line::styled("Starting backend client...", muted),
        }
    }
}

impl Widget for FooterWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        let area = Rect::new(area.x, area.y, area.width, 1);
        let bar_style = Style::default().fg(Color::White).bg(Color::DarkGray);

        Block::default().style(bar_style).render(area, buf);

        let (badge_text, badge_color) = self.connection_badge();
        let badge = Line::from(badge_text);
        let badge_width = badge.width() as u16;
        let badge_style = Style::default().fg(Color::Black).bg(badge_color).bold();

        if area.width < 32 {
            Paragraph::new(badge).style(badge_style).render(area, buf);
            return;
        }

        let hints = Line::from(vec![
            Span::styled("q", Style::default().fg(Color::White).bold()),
            Span::styled(" Quit ", Style::default().fg(Color::Gray)),
        ]);

        let hints_width = if area.width >= 60 {
            hints.width() as u16
        } else {
            0
        };

        let [badge_area, gap_area, detail_area, hints_area] = Layout::horizontal([
            Constraint::Length(badge_width),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(hints_width),
        ])
        .areas(area);

        Paragraph::new(badge)
            .style(badge_style)
            .render(badge_area, buf);

        Paragraph::new(" ").style(bar_style).render(gap_area, buf);

        Paragraph::new(self.detail())
            .style(bar_style)
            .render(detail_area, buf);

        if hints_width > 0 {
            Paragraph::new(hints)
                .style(bar_style)
                .right_aligned()
                .render(hints_area, buf);
        }
    }
}
