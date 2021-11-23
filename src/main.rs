mod eve;
mod library;
#[allow(dead_code)]
mod libs;
mod position;
mod ui;

use eve::{Event, Events};
use library::{LibItem, LibKind};
use libs::{update_queue, Data};

use mpd::{song::Song, status, Client, Query, Term};
use std::borrow::Cow::Borrowed;
use std::error::Error;
use std::io;
use termion::{event::Key, raw::IntoRawMode};
use tui::{backend::TermionBackend, Terminal};

fn main() -> Result<(), Box<dyn Error>> {
    let mut client = Client::connect("127.0.0.1:6600").unwrap();

    let events = Events::new();

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut data = Data::new(&mut client);

    loop {
        data.status = client.status()?;
        data.current = match client.currentsong() {
            Ok(song) => match song {
                Some(song) => song,
                None => Song::default(),
            },
            Err(_io) => Song::default(),
        };

        if data.tabindex == 3 {
            update_queue(&mut data, &mut client);
        }

        terminal.draw(|f| ui::draw(f, &mut data)).unwrap();

        if let Event::Input(input) = events.next()? {
            match input {
                Key::Char('q') => {
                    break;
                }
                Key::Char('o') => {
                    if data.options {
                        data.options = false
                    } else {
                        data.options = true
                    }
                }
                Key::Char('1') => data.tabindex = 0,
                Key::Char('2') => data.tabindex = 1,
                Key::Char('3') => data.tabindex = 2,
                Key::Char('4') => data.tabindex = 3,
                Key::Char('5') => data.tabindex = 4,
                Key::Char('s') => {
                    client.stop()?;
                }
                Key::Char('+') => {
                    if data.status.volume < 99 {
                        let vol = &data.status.volume + 2;
                        client.volume(vol)?;
                    }
                }
                Key::Char('-') => {
                    if data.status.volume > 1 {
                        let vol = &data.status.volume - 2;
                        client.volume(vol)?;
                    }
                }
                Key::Char('p') => {
                    let status = client.status()?;
                    if status.state == status::State::Play {
                        client.pause(true)?
                    } else {
                        client.play()?;
                    }
                }
                Key::Char('u') => {
                    client.update().unwrap();
                    data.update(&mut client);
                }
                Key::Char('d') => {
                    if data.tabindex == 3 {
                        client
                            .delete(data.queue.state.selected().unwrap_or(0) as u32)
                            .unwrap_or(());
                        update_queue(&mut data, &mut client);
                        data.queue.select_last();
                    }
                }
                Key::Down => data.down(),
                Key::Up => data.up(),
                Key::Right => data.nexttab(),
                Key::Left => data.prevtab(),
                Key::Char('a') => data.library.add_to_queue(&data, &mut client),
                Key::Char('.') => {
                    client.next()?;
                    client.pause(true)?;
                    client.play()?
                }
                Key::Char(',') => {
                    client.prev()?;
                    client.pause(true)?;
                    client.play()?
                }
                Key::Char('\n') => {
                    let idx = data.library.state.selected().unwrap();

                    if data.tabindex == 3 {
                        client.switch(data.queue.state.selected().unwrap() as u32)?;
                        client.pause(true)?;
                        client.play()?;
                    }
                    if data.tabindex == 1 {
                        if data.library.items[idx].tag == LibKind::Artist {
                            let pos = data.library.state.selected().unwrap();
                            data.artists.state.select(Some(pos));
                            let item = data.library.get_albums(&mut client);
                            data.albums.items.clear();
                            data.library.state.select(Some(0));
                            let mut temp = vec![
                                LibItem::new("  [Back]".into(), LibKind::Back),
                                LibItem::new(
                                    data.library.items[pos].content.clone().into(),
                                    LibKind::Artist,
                                ),
                            ];
                            data.albums.items.append(&mut temp);
                            for (i, album) in item.items.iter().enumerate() {
                                data.albums.items.insert(
                                    i + 2,
                                    LibItem::new(album.content.clone(), LibKind::Album),
                                );
                                data.library = data.albums.clone();
                            }
                        } else if data.library.items[idx].tag == LibKind::Album {
                            let pos = data.library.state.selected().unwrap();
                            data.albums.state.select(Some(pos));
                            let item = data.library.get_titles(&mut client);
                            data.titles.items.clear();
                            data.library.state.select(Some(0));
                            let mut temp = vec![
                                LibItem::new("  [Back]".into(), LibKind::Back),
                                LibItem::new(
                                    data.library.items[pos].content.clone().into(),
                                    LibKind::Album,
                                ),
                            ];
                            data.titles.items.append(&mut temp);
                            for (i, album) in item.items.iter().enumerate() {
                                data.titles.items.insert(
                                    i + 2,
                                    LibItem::new(album.content.clone(), LibKind::Title),
                                );
                            }
                            data.library = data.titles.clone();
                        } else if data.library.items[idx].tag == LibKind::Title {
                            client
                                .findadd(&Query::new().and(
                                    Term::Tag(Borrowed("Title")),
                                    Borrowed(data.library.items[idx].content.as_str()),
                                ))
                                .unwrap();
                            update_queue(&mut data, &mut client);
                            client.switch(data.queue.items.len() as u32 - 1)?;
                            client.pause(true)?;
                            client.play()?;
                        } else if data.library.items[idx].tag == LibKind::Back {
                            match data.library.items[idx + 1].tag {
                                LibKind::Artist => {
                                    data.library = data.artists.clone();
                                    data.albums.state.select(Some(0))
                                }
                                LibKind::Album => data.library = data.albums.clone(),
                                _ => {}
                            }
                        };
                    };
                }
                _ => {}
            }
        }
    }
    Ok(())
}
