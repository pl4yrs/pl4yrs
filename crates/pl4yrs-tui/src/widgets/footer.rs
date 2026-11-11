use pl4yrs_api::playback::PlayerStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::ConnectionStatus;

pub struct FooterWidget<'a> {
    status: Option<&'a PlayerStatus>,
    last_error: Option<&'a String>,
    connection_status: Option<&'a ConnectionStatus>,
}

impl<'a> FooterWidget<'a> {
    pub fn new(
        status: Option<&'a PlayerStatus>,
        last_error: Option<&'a String>,
        connection_status: Option<&'a ConnectionStatus>,
    ) -> Self {
        Self {
            status,
            last_error,
            connection_status,
        }
    }
}

impl Widget for FooterWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [connection_area, status_area] =
            Layout::horizontal([Constraint::Length(20), Constraint::Fill(1)]).areas(area);

        let (connection_color, connection_text) = match &self.connection_status {
            Some(ConnectionStatus::Connecting) => (Color::Yellow, "Connecting..."),
            Some(ConnectionStatus::Connected) => (Color::Green, "Connected"),
            Some(ConnectionStatus::Disconnected(_message)) => (Color::Red, "Disconnected"),
            None => (Color::Red, "Unknown Status"),
        };

        let connection_status = Paragraph::new(connection_text)
            .fg(connection_color)
            .bg(Color::Black)
            .left_aligned();
        connection_status.render(connection_area, buf);

        let footer_text = match (&self.last_error, &self.status) {
            (Some(error), _) => format!("Server error: {error}"),
            (None, Some(_)) => "Status received".to_owned(),
            (None, None) => "Press s to fetch status".to_owned(),
        };

        let footer_status = Paragraph::new(footer_text)
            .fg(Color::Cyan)
            .bg(Color::Black)
            .centered();
        footer_status.render(status_area, buf);
    }
}
