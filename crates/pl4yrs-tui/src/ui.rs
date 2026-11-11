use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::app::{App, ConnectionStatus};
use crate::widgets::PlayerStatusWidget;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [help_area, player_area, footer_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        let [connection_area, status_area] =
            Layout::horizontal([Constraint::Length(20), Constraint::Fill(1)]).areas(footer_area);

        let block = Block::bordered()
            .title("pl4yrs-tui")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let text = "Welcome to Pl4yrs.\n\
                Press `Esc`, `Ctrl-C` or `q` to stop running.\n\
                Press `s` to fetch Pl4yrs status.\n";

        let paragraph = Paragraph::new(text)
            .block(block)
            .fg(Color::Cyan)
            .bg(Color::Black)
            .centered();

        paragraph.render(help_area, buf);

        PlayerStatusWidget::new(self.status.as_ref()).render(player_area, buf);

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
