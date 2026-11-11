use pl4yrs_api::playback::{PlaybackSource, PlayerState, PlayerStatus, Progress};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, LineGauge, Padding, Paragraph, Widget, Wrap};
use std::time::Duration;

enum PlayerLayout {
    Compact,
    Normal,
    Detailed,
}

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
        if area.is_empty() {
            return;
        }

        let block = Block::bordered()
            .title("Playback")
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.is_empty() {
            return;
        }

        let Some(status) = self.status else {
            Paragraph::new("Waiting for player status…").render(inner, buf);
            return;
        };

        match layout_for(inner) {
            PlayerLayout::Compact => render_compact(status, inner, buf),
            PlayerLayout::Normal => render_normal(status, inner, buf),
            PlayerLayout::Detailed => render_detailed(status, inner, buf),
        }
    }
}

fn layout_for(area: Rect) -> PlayerLayout {
    if area.height < 4 || area.width < 30 {
        PlayerLayout::Compact
    } else if area.height < 7 || area.width < 50 {
        PlayerLayout::Normal
    } else {
        PlayerLayout::Detailed
    }
}

fn state_label(state: &PlayerState) -> &'static str {
    match state {
        PlayerState::Playing => "Playing",
        PlayerState::Paused => "Paused",
        PlayerState::Stopped => "Stopped",
        PlayerState::Buffering => "Buffering",
        PlayerState::Idle => "Idle",
        PlayerState::Unknown => "Unknown",
    }
}

fn format_time(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn render_progress(progress: Option<&Progress>, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }

    let Some(progress) = progress else {
        Paragraph::new("Progress unavailable")
            .dim()
            .render(area, buf);
        return;
    };

    let position = format_time(progress.position);

    match progress.duration.filter(|duration| !duration.is_zero()) {
        Some(duration) => {
            let label = format!("{position} / {}", format_time(duration),);

            let ratio = (progress.position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);

            if area.width < 30 {
                Paragraph::new(label).render(area, buf);
            } else {
                LineGauge::default()
                    .ratio(ratio)
                    .label(label)
                    .filled_style(Style::default().fg(Color::Cyan))
                    .unfilled_style(Style::default().fg(Color::DarkGray))
                    .render(area, buf);
            }
        }
        None => {
            Paragraph::new(format!("{position} / duration unknown")).render(area, buf);
        }
    }
}

fn render_compact(status: &PlayerStatus, area: Rect, buf: &mut Buffer) {
    let title = status
        .media
        .as_ref()
        .map(|media| media.title.as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or("No media information");

    let line = Line::from(vec![
        Span::styled(
            format!("{} · ", state_label(&status.state)),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(title, Style::default().bold()),
    ]);

    Paragraph::new(line)
        .wrap(Wrap { trim: true })
        .render(area, buf);
}

fn render_normal(status: &PlayerStatus, area: Rect, buf: &mut Buffer) {
    let [state_area, title_area, artist_area, progress_area, _rest] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);

    Paragraph::new(state_label(&status.state)).render(state_area, buf);

    let media = status.media.as_ref();

    let title = media
        .map(|media| media.title.as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or("No media information");

    Paragraph::new(title)
        .bold()
        .wrap(Wrap { trim: true })
        .render(title_area, buf);

    let artist = media
        .and_then(|media| media.artist.as_deref())
        .unwrap_or("");

    Paragraph::new(artist).render(artist_area, buf);

    render_progress(status.progress.as_ref(), progress_area, buf);
}

fn render_detailed(status: &PlayerStatus, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }

    let [details_area, progress_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let [media_area, _gap, source_area] = Layout::horizontal([
        Constraint::Fill(2),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(details_area);

    let media = status.media.as_ref();

    let title = media
        .map(|media| media.title.as_str())
        .filter(|title| !title.is_empty())
        .unwrap_or("No media information");

    let artist = media
        .and_then(|media| media.artist.as_deref())
        .filter(|artist| !artist.is_empty())
        .unwrap_or("Unknown artist");

    let album = media
        .and_then(|media| media.album.as_deref())
        .filter(|album| !album.is_empty())
        .unwrap_or("Unknown album");

    let state_color = match &status.state {
        PlayerState::Playing => Color::Green,
        PlayerState::Paused | PlayerState::Buffering => Color::Yellow,
        _ => Color::Gray,
    };

    let media_lines = vec![
        Line::from(Span::styled(
            state_label(&status.state),
            Style::default().fg(state_color),
        )),
        Line::from(Span::styled(title, Style::default().bold())),
        Line::from(artist),
        Line::from(Span::styled(album, Style::default().fg(Color::DarkGray))),
    ];

    Paragraph::new(media_lines)
        .wrap(Wrap { trim: true })
        .render(media_area, buf);

    let (source_name, server) = match status.source.as_ref() {
        Some(PlaybackSource::Sendspin { server }) => ("SendSpin", Some(server.as_str())),
        Some(PlaybackSource::LocalFile) => ("Local file", None),
        None => ("Not reported", None),
    };

    let mut source_lines = vec![
        Line::from(Span::styled("Source", Style::default().fg(Color::DarkGray))),
        Line::from(source_name),
    ];

    if let Some(server) = server.filter(|server| !server.is_empty()) {
        source_lines.push(Line::from(server));
    }

    Paragraph::new(source_lines)
        .wrap(Wrap { trim: true })
        .render(source_area, buf);

    render_progress(status.progress.as_ref(), progress_area, buf);
}
