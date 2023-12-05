use crate::library::{Tree, LibKind};
pub use crate::libs::Data;
use crate::position::PositionWidget;

use mpd::status::{State, Status};
use mpd::song::QueuePlace;

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Modifier},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, data: &mut Data) {
    let block = Block::default().borders(Borders::NONE).style(data.style);
    f.render_widget(block, f.size());

    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .horizontal_margin(0)
        .split(f.size());
    let top = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(5)].as_ref())
        .direction(Direction::Horizontal)
        .horizontal_margin(0)
        .split(chunks[0]);
    let tabtitles = [status_icon(&data.status), " ", " ", " "]
        .iter()
        .cloned()
        .map(|t| Spans::from(Span::styled(t, data.style)))
        .collect();
    let tabs = Tabs::new(tabtitles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(data.tabindex)
        .style(data.style)
        .highlight_style(Style::default().fg(data.colors.highlight))
        .divider("");
    f.render_widget(tabs, top[0]);

    draw_volume(f, data, top[1]);

    match data.tabindex {
        0 => draw_current(f, data, chunks[1]),
        1 => draw_library(f, data, chunks[1]),
        2 => draw_queue(f, data, chunks[1]),
        3 => draw_settings(f, data, chunks[1]),
        5 => draw_options(f, data, chunks[1]),
        _ => {}
    }
    if data.options {
        draw_options(f, data, chunks[1]);
    }        

    draw_position(f, &data, chunks[2]);

    draw_status_bar(f, &data, chunks[3]);
}

fn draw_volume<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    let volume = data.status.volume.to_string();
    let text = vec![Spans::from(vec![Span::from(volume), Span::from("% ")])];
    let volume = Paragraph::new(text)
        .block(Block::default().style(data.style).borders(Borders::BOTTOM))
        .alignment(Alignment::Left);
    f.render_widget(volume, area);
}

fn draw_current<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    let area = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .horizontal_margin(1)
        .split(area);
    let default_artist = String::from("Unknown Artist");
    let default_album = String::from("Uknown Album");
    let artist_text = data.current.tags.get("Artist").unwrap_or(&default_artist);
    let album_text = data.current.tags.get("Album").unwrap_or(&default_album);
    let text = vec![
        Spans::from(Span::styled(
            " Artist:",
            Style::default().fg(data.colors.highlight),
        )),
        Spans::from(vec![Span::from(""), Span::from(artist_text.clone())]),
        Spans::from(Span::styled(
            " Album:",
            Style::default().fg(data.colors.highlight),
        )),
        Spans::from(vec![Span::from(""), Span::from(album_text.clone())]),
        //Spans::from(repeat_shuffle(&data.status)),
    ];
    let paragraph = Paragraph::new(text)
        .block(Block::default().style(data.style))
        .alignment(Alignment::Center);
    f.render_widget(paragraph, area[0]);

    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(5),
                Constraint::Min(4),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .direction(Direction::Horizontal)
        .split(area[1]);

    let time = time::Duration::seconds(0);
    let status = &data.status;
    let elapsedmin = status.elapsed.unwrap_or(time).num_minutes();
    let elapsedsec = status.elapsed.unwrap_or(time).num_seconds() % 60;
    let durationmin = status.duration.unwrap_or(time).num_minutes();
    let durationsec = status.duration.unwrap_or(time).num_seconds() % 60;

    let elapsed = format!("{:0>2}:{:0>2}", elapsedmin, elapsedsec);
    let duration = format!("{:0>2}:{:0>2} ", durationmin, durationsec);
    let text = Paragraph::new(elapsed)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .alignment(Alignment::Left);
    f.render_widget(text, chunks[0]);

    let text = Paragraph::new(duration)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .alignment(Alignment::Right);
    f.render_widget(text, chunks[2]);

}

fn draw_library<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([
            Constraint::Min(0),
            Constraint::Length(4)
            ].as_ref())
        .direction(Direction::Horizontal)
        .split(area);
    let mut n = 0;
    for item in &mut data.library.items {
        if &item.content == &data.current.title.clone().unwrap_or("".into()) { data.library.state.set_playing(Some(n)) }
        n += 1;
    }
    let list = Tree::new(&data.library.items)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .highlight_style(Style::default().bg(data.colors.selected))//fg(data.colors.highlight))
        .playing_symbol(" 󰄨  ")
        .playing_style(Style::default().fg(data.colors.highlight));
    f.render_stateful_widget(list, chunks[0], &mut data.library.state);

    let sidelist = Tree::new(&data.sidelist)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .highlight_style(Style::default().bg(data.colors.selected))//fg(data.colors.highlight))
        .playing_style(Style::default().fg(data.colors.highlight));
    f.render_stateful_widget(sidelist, chunks[1], &mut data.library.state);
}

fn draw_queue<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    /*let mut i = 0;
    for song in data.queue.items.iter() {
        if data.status.song.unwrap_or(QueuePlace::default()).pos as usize > 0 {
            i = data.status.song.unwrap_or(QueuePlace::default()).pos as usize;
        }
    }
    data.drained = i;
    data.queue.items.drain(0..i);*/

    let list = Tree::new(&data.queue.items)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .highlight_style(Style::default().bg(data.colors.selected))
        .playing_style(Style::default().fg(data.colors.highlight))
        .playing_symbol(" 󰄨  ");
    f.render_stateful_widget(list, area, &mut data.queue.state);
}

fn draw_settings<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    let layout = Layout::default()
        .constraints([Constraint::Percentage(100)])
        .horizontal_margin(3)
        .split(area);
    let block = Block::default().style(data.style).borders(Borders::NONE);
    let items: Vec<ListItem> = data
        .settings
        .items
        .iter()
        .map(|i| ListItem::new(Spans::from(i.clone())))
        .collect();
    let list = List::new(items)
        .block(Block::default().style(data.style).borders(Borders::ALL))
        .highlight_style(Style::default().fg(data.colors.highlight));
    f.render_widget(block, area);
    f.render_stateful_widget(list, layout[0], &mut data.settings.state)
}

fn draw_position<B>(f: &mut Frame<B>, data: &Data, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(100)])
        .horizontal_margin(0)
        .split(area);
    let status = &data.status;
    let pos = match status.elapsed {
        Some(p) => p.num_seconds(),
        None => 0,
    };
    let full = match status.duration {
        Some(f) => f.num_seconds(),
        None => 1,
    };

    let gauge = PositionWidget::default()
        .block(Block::default().borders(Borders::NONE))
        .style(data.style)
        .ratio(pos as f64 / full as f64);
    f.render_widget(gauge, chunks[0]);
}

fn draw_status_bar<B>(f: &mut Frame<B>, data: &Data, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(4),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .direction(Direction::Horizontal)
        .split(area);

    let time = time::Duration::seconds(0);
    let status = &data.status;
    let elapsedmin = status.elapsed.unwrap_or(time).num_minutes();
    let elapsedsec = status.elapsed.unwrap_or(time).num_seconds() % 60;
    let durationmin = status.duration.unwrap_or(time).num_minutes();
    let durationsec = status.duration.unwrap_or(time).num_seconds() % 60;

    let elapsed = format!(" {:0>2}:{:0>2}", elapsedmin, elapsedsec);
    let duration = format!("{:0>2}:{:0>2} ", durationmin, durationsec);
    let text = Paragraph::new(elapsed)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .alignment(Alignment::Left);
    //f.render_widget(text, chunks[0]);

    let text = Paragraph::new(duration)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .alignment(Alignment::Right);
    //f.render_widget(text, chunks[2]);

    let current = match data.current.title.clone() {
        Some(song) => String::from(song),
        None => String::from(""),
    };
    let text = Paragraph::new(current)
        .block(Block::default().style(data.style).borders(Borders::NONE))
        .alignment(Alignment::Center);
    f.render_widget(text, chunks[1]);
}

fn draw_options<B>(f: &mut Frame<B>, data: &mut Data, area: Rect)
where
    B: Backend,
{
    let layout = Layout::default()
        .constraints([Constraint::Percentage(100)])
        .vertical_margin(4)
        .horizontal_margin(6)
        .split(area);
    let block = Block::default().style(data.style).borders(Borders::NONE);
    let list = Tree::new(&data.opts.items)
        .block(Block::default().style(data.style).borders(Borders::ALL).border_type(tui::widgets::BorderType::Rounded))
        .highlight_style(Style::default().bg(data.colors.selected));
    f.render_widget(block, layout[0]);
    f.render_stateful_widget(list, layout[0], &mut data.opts.state)
}

fn status_icon<'a>(status: &Status) -> &'a str {
    let currently: &str;
    if status.state == State::Play {
        currently = " ";
        &currently[..]
    } else {
        currently = " ";
        &currently[..]
    }
}

fn repeat_shuffle<'a>(status: &Status) -> Vec<Span> {
    let mut out: &str = "";
    if status.repeat {
        out = "凌"
    } else if !status.single {
        out = " 綾"
    }
    vec![Span::from(out)]
}
