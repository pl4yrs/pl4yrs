use pl4yrs_api::playback::PlayerStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

pub struct PlayerStatusWidget<'a> {
    status: Option<&'a PlayerStatus>,
}

impl<'a> PlayerStatusWidget<'a> {
    pub fn new(status: Option<&'a PlayerStatus>) -> Self {
        Self { status }
    }
}

impl Widget for PlayerStatusWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = self
            .status
            .as_ref()
            .and_then(|status| status.media.as_ref())
            .map(|media| media.title.as_str())
            .unwrap_or("No media information");

        title.render(area, buf);
    }
}
