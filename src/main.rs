mod eve;
mod library;
#[allow(dead_code)]
mod libs;
mod position;
mod ui;

use eve::{Event, Events};
use library::{LibItem, LibKind};
use libs::{update_queue, Data, Library};

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

        if data.tabindex == 2 {
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
                        match data.selected().tag {
                            LibKind::Title => data.options = true,
                            LibKind::Album => data.options = true,
                            _ => {}
                        }
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
                Key::Char(' ') => {
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
                    if data.tabindex == 2 {
                        client
                            .delete(
                                data.queue.state.selected().unwrap_or(0) as u32
                                    + (data.drained + 1) as u32,
                            )
                            .unwrap_or(());
                        update_queue(&mut data, &mut client);
                        data.queue.select_last();
                    }
                }
                Key::Down => data.down(&mut client),
                Key::Up => data.up(&mut client),
                Key::Right => data.nexttab(),
                Key::Left => data.prevtab(),
                Key::Char('a') => data.library.add_to_queue(&data, &mut client),
                Key::Char('.') => {
                    client.next()?;
                    client.pause(true)?;
                    client.play()?;
                    update_queue(&mut data, &mut client);
                }
                Key::Char(',') => {
                    client.prev()?;
                    client.pause(true)?;
                    client.play()?
                }
                Key::Char('b') => {
                    if data.tabindex == 1 {
                        data.library = data.path.up()
                    }
                }
                Key::Char('\n') => {
                    let mut idx = data.library.state.selected().unwrap();

                    if data.tabindex == 2 {
                        client.switch(
                            data.queue.state.selected().unwrap() as u32 + data.drained as u32,
                        )?;
                        client.pause(true)?;
                        client.play()?;
                    }
                    if data.tabindex == 1 && !data.options {
                        idx = data.library.state.selected().unwrap();
                        match data.library.items[idx].tag {
                            LibKind::Home => match data.library.state.selected().unwrap_or(0) {
                                0 => data.library = data.playlists.clone(),
                                1 => data.library = data.artists.clone(),
                                2 => data.library = data.library.get_albums(&mut client),
                                3 => data.library = data.titles.clone(),
                                _ => {}
                            },
                            LibKind::Artist => {
                                let albums: Library = data.library.get_albums(&mut client);
                                data.albums = albums.clone();
                                data.library = albums.clone();
                                data.path.update(albums);
                            }
                            LibKind::Album => {
                                let titles = data.library.get_titles(&mut client);
                                data.path.update(titles.clone());
                                data.library = titles.clone();
                                //data.path.update(titles);
                            }
                            LibKind::Title => {
                                client.clear()?;
                                client
                                    .findadd(&Query::new().and(
                                        Term::Tag(Borrowed("Title")),
                                        Borrowed(data.library.items[idx].content.as_str()),
                                    ))
                                    .unwrap();
                                update_queue(&mut data, &mut client);
                                //client.switch(data.queue.items.len() as u32 + (data.drained + 1) as u32)?;
                                //client.pause(true)?;
                                client.play()?;
                            }
                            LibKind::Playlist => {
                                let p_list = client.playlist("Dance")?;
                                //data.library = 0;
                                for play in p_list {
                                    data.library.push(LibItem::new(
                                        play.title.unwrap_or("".into()),
                                        LibKind::Title,
                                    ))
                                }
                            }
                            LibKind::Option => match data.library.state.selected().unwrap_or(0) {
                                0 => data.library.add_to_queue(&data, &mut client),
                                1 => {}
                                _ => {}
                            },
                            _ => {}
                        }
                    };
                    if data.options {
                        match data.opts.state.selected().unwrap_or(0) {
                            0 => data.library.add_to_queue(&data, &mut client),
                            1 => {}
                            _ => {}
                        }
                        data.options = false;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
