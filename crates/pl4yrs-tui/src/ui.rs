use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::widgets::PlayerStatusWidget;
use crate::{app::App, widgets::FooterWidget};

const PLAYER_MAX_WIDTH: u16 = 100;
const PLAYER_MAX_HEIGHT: u16 = 10;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let help_height = if area.height >= 14 { 5 } else { 0 };

        let [help_area, player_area, footer_area] = Layout::vertical([
            Constraint::Length(help_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        let block = Block::bordered()
            .title("pl4yrs-tui")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let text = "Welcome to Pl4yrs.\n\
                Press `Esc`, `Ctrl-C` or `q` to stop running.\n";

        let paragraph = Paragraph::new(text)
            .block(block)
            .fg(Color::Cyan)
            .bg(Color::Black)
            .centered();

        paragraph.render(help_area, buf);

        let [player_column] =
            Layout::horizontal([Constraint::Length(player_area.width.min(PLAYER_MAX_WIDTH))])
                .flex(Flex::Center)
                .areas(player_area);

        let [player_panel, _remaining] = Layout::vertical([
            Constraint::Length(player_column.height.min(PLAYER_MAX_HEIGHT)),
            Constraint::Min(0),
        ])
        .areas(player_column);

        PlayerStatusWidget::new(self.status.as_ref()).render(player_panel, buf);

        FooterWidget::new(
            self.status.as_ref(),
            self.last_error.as_ref(),
            self.connection_status.as_ref(),
        )
        .render(footer_area, buf);
    }
}
