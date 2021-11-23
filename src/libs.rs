use crate::library::{LibItem, LibKind, LibState};
use mpd::{song::QueuePlace, song::Song, status::Status, Query, Term};
use std::borrow::Cow::Borrowed;
use std::iter::FromIterator;
use tui::{style::Color, style::Style, widgets::ListState};

pub struct Data {
    pub library: Library,
    pub artists: Library,
    pub albums: Library,
    pub titles: Library,
    pub queue: Library,
    pub playlists: Library,
    pub settings: Settings,
    pub status: Status,
    pub colors: ColorScheme,
    pub tabindex: usize,
    pub current: mpd::song::Song,
    pub style: tui::style::Style,
    pub options: bool,
}

impl<'a> Data {
    pub fn new(client: &mut mpd::Client) -> Data {
        let query = Query::new();

        let artists = client.list(&Term::Tag(Borrowed("Artist")), &query);
        let albums = client.list(&Term::Tag(Borrowed("Album")), &query);
        let titles = client.list(&Term::Tag(Borrowed("Title")), &query);
        let playlists = client.playlists().unwrap();
        let queue = client.queue().unwrap();

        let mut artistitems: Vec<String> = vec![];
        let mut albumitems: Vec<String> = vec!["[Back]".into()];
        let mut titleitems: Vec<String> = vec![];
        let mut queueitems: Vec<String> = vec![];
        let mut libraryitems = vec![];
        let mut playitems = vec![];

        for mut artists in artists.unwrap() {
            if artists.is_empty() {
                artists = "[All Albums]".into()
            }
            artistitems.push(artists.clone());
            libraryitems.push(artists);
        }
        //artistitems.remove(0);
        for albums in albums.unwrap() {
            albumitems.push(albums);
        }
        albumitems.remove(0);
        for titles in titles.unwrap() {
            titleitems.push(titles);
        }
        titleitems.remove(0);
        for queue in queue {
            let q = queue.title.unwrap_or("".into());
            queueitems.push(q);
        }
        for play in playlists {
            playitems.push(play.name);
        }

        let library = Library::new(libraryitems, LibKind::Artist);
        let artists = Library::new(artistitems, LibKind::Artist);
        let albums = Library::new(albumitems, LibKind::Album);
        let titles = Library::new(titleitems, LibKind::Title);
        let queue = Library::new(queueitems, LibKind::None);
        let playlists = Library::new(playitems, LibKind::None);

        let tabindex: usize = 0;
        let status = client.status().unwrap();
        Data {
            library,
            artists,
            albums,
            titles,
            playlists,
            queue,
            settings: Settings::new(),
            status,
            tabindex,
            colors: ColorScheme {
                foreground: Color::White,
                background: Color::Black,
                highlight: Color::Blue,
            },
            current: match client.currentsong() {
                Ok(song) => match song {
                    Some(song) => song,
                    None => Song::default(),
                },
                Err(_io) => Song::default(),
            },
            style: tui::style::Style::default()
                .fg(Color::White)
                .bg(Color::Black),
            options: false,
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let tabindex = self.tabindex;
        *self = Self::new(client);
        self.tabindex = tabindex;
    }

    pub fn nexttab(&mut self) {
        self.tabindex = (self.tabindex + 1) % 5;
    }

    pub fn prevtab(&mut self) {
        match self.tabindex {
            0 => self.tabindex = 4,
            _ => self.tabindex -= 1,
        }
    }

    pub fn up(&mut self) {
        match self.tabindex {
            1 => self.library.previous(),
            2 => self.albums.previous(),
            3 => self.queue.previous(),
            4 => self.settings.previous(),
            _ => {}
        }
    }

    pub fn down(&mut self) {
        match self.tabindex {
            1 => self.library.next(),
            2 => self.albums.next(),
            3 => self.queue.next(),
            4 => self.settings.next(),
            _ => {}
        }
    }
}

pub struct ColorScheme {
    pub foreground: Color,
    pub background: Color,
    pub highlight: Color,
}

pub struct ArtistOptions {
    pub items: Vec<String>,
}

#[derive(Clone)]
pub struct Library {
    pub items: Vec<LibItem>,
    pub state: LibState,
}

impl<'a> Library {
    pub fn new(items: Vec<String>, kind: LibKind) -> Library {
        Library {
            state: LibState::default(),
            items: items
                .iter()
                .map(|i| LibItem::new(i.clone(), kind.clone()))
                .collect(),
        }
    }

    pub fn add_to_queue(&self, data: &Data, client: &mut mpd::Client) {
        let mut query = Query::new();
        let index = data.library.state.selected().unwrap();
        match self.items[index].tag {
            LibKind::Artist => {
                let query = query.and(
                    Term::Tag("Artist".into()),
                    self.items[index].content.clone(),
                );
                client.findadd(&query).unwrap();
            }
            LibKind::Album => {
                let query = query.and(Term::Tag("Album".into()), self.items[index].content.clone());
                client.findadd(&query).unwrap();
            }
            LibKind::Title => {
                let query = query.and(Term::Tag("Title".into()), self.items[index].content.clone());
                client.findadd(&query).unwrap();
            }
            _ => {}
        };
    }

    pub fn select_last(&mut self) {
        let len = self.items.len();
        if len > 0 && self.state.selected().unwrap() == len {
            self.state.select(Some(len - 1));
            self.state.offset(0);
        }
    }

    pub fn newlib(items: Vec<LibItem>) -> Library {
        Library {
            state: LibState::default(),
            items,
        }
    }

    pub fn get_albums(&mut self, client: &mut mpd::Client) -> Library {
        let idx: String = self.items[self.state.selected().unwrap()].content.clone();
        let mut query = Query::new();

        let name = idx.as_str();
        let items = client.list(
            &Term::Tag("Album".into()),
            &query.and(Term::Tag("Artist".into()), Borrowed(name)),
        );
        let mut artistalbums: Vec<String> = vec![];
        for albums in items.unwrap() {
            //albums.insert_str(0, "");
            artistalbums.push(albums);
        }

        Library::new(artistalbums, LibKind::Album)
    }

    pub fn get_titles(&mut self, client: &mut mpd::Client) -> Library {
        let idx: String = self.items[self.state.selected().unwrap()].content.clone();
        let artist = self.items[1].content.clone();

        let name: &str = idx.as_str();

        let mut query = Query::new();
        let query = query
            .and(Term::Tag("Album".into()), Borrowed(name))
            .and(Term::Tag("Artist".into()), artist);
        let mut items = client.search(&query, None).unwrap();
        items.sort_by_key(|song| {
            song.tags
                .get("Track")
                .cloned()
                .map(|t| t.parse::<u32>().unwrap())
        });
        let mut albumtitles: Vec<String> = vec![];
        for albums in items {
            albumtitles.push(albums.title.unwrap_or("".into()));
        }

        Library::new(albumtitles, LibKind::Title)
    }

    /*pub fn enter(&mut self, data: &mut Data) -> Library {
        let pos = self.state.selected().unwrap();

        match self.items[pos].tag {
            LibKind::Artist => ,
            LibKind::Album => ,
            LibKind::Title => ,
            LibKind::Back => ,
        }
    }*/

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.items.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl FromIterator<LibItem> for Library {
    fn from_iter<I: IntoIterator<Item = LibItem>>(iter: I) -> Self {
        let mut c = vec![];
        let mut tag: LibKind = LibKind::Artist;
        for i in iter {
            c.push(i.content);
            tag = i.tag;
        }

        Library::new(c, tag)
    }
}

pub struct Artists {
    pub items: Vec<LibItem>,
    pub state: LibState,
    pub options: Vec<String>,
}

impl<'a> Artists {
    pub fn new(items: Vec<String>) -> Artists {
        Artists {
            state: LibState::default(),
            items: items
                .iter()
                .map(|i| LibItem::new(i.clone(), LibKind::Artist))
                .collect(),
            options: vec![String::from("add to queue")],
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let query = Query::new();

        let artists = client.list(&Term::Tag(Borrowed("Artist")), &query);
        let mut items: Vec<LibItem> = vec![];

        for artists in artists.unwrap() {
            //artists.insert_str(0, " ");
            items.push(LibItem::new(artists, LibKind::Title));
        }
        self.items = items
    }

    pub fn get_albums(&mut self, client: &mut mpd::Client) -> Albums {
        let idx: String = self.items[self.state.selected().unwrap()].content.clone();
        let mut query = Query::new();

        let name = idx.as_str();
        let items = client.list(
            &Term::Tag(Borrowed("Album")),
            &query.and(Term::Tag(Borrowed("Artist")), Borrowed(name)),
        );
        let mut artistalbums: Vec<String> = vec![];
        for albums in items.unwrap() {
            artistalbums.push(albums);
        }

        Albums::new(artistalbums)
    }

    pub fn enter(&self) {
        //Albums::new(self.items[self.state.selected()]);
    }

    pub fn options(&self) {}

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct Albums {
    pub items: Vec<LibItem>,
    pub state: ListState,
}

impl<'a> Albums {
    pub fn new(items: Vec<String>) -> Albums {
        Albums {
            state: ListState::default(),
            items: items
                .iter()
                .map(|i| LibItem::new(i.clone(), LibKind::Album))
                .collect(),
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let query = Query::new();

        let albums = client.list(&Term::Tag(Borrowed("Album")), &query);

        self.items.clear();
        for items in albums.unwrap() {
            self.items.push(LibItem::new(items, LibKind::Album))
        }
    }

    pub fn get_titles(&mut self, client: &mut mpd::Client, library: &Library) -> Titles {
        let idx: String = library.items[library.state.selected().unwrap()]
            .content
            .clone();
        let artist = library.items[0].content.clone();

        let name: &str = idx.as_str();

        let mut query = Query::new();
        let query = query
            .and(Term::Tag("Album".into()), Borrowed(name))
            .and(Term::Tag("Artist".into()), artist);
        let mut items = client.search(&query, None).unwrap();
        items.sort_by_key(|song| {
            song.tags
                .get("Track")
                .cloned()
                .map(|t| t.parse::<u32>().unwrap())
        });
        let mut albumtitles: Vec<String> = vec![];
        for albums in items {
            albumtitles.push(albums.title.unwrap_or("".into()));
        }

        Titles::new(albumtitles)
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct Titles {
    pub items: Vec<LibItem>,
    pub state: ListState,
}

impl<'a> Titles {
    pub fn new(items: Vec<String>) -> Titles {
        Titles {
            state: ListState::default(),
            items: items
                .iter()
                .map(|i| LibItem::new(i.clone(), LibKind::Title))
                .collect(),
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let query = Query::new();

        let titles = client.list(&Term::Tag(Borrowed("Title")), &query);
        let mut items: Vec<LibItem> = vec![];

        for titles in titles.unwrap() {
            items.push(LibItem::new(titles, LibKind::Title));
        }
        self.items = items
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub fn update_queue(data: &mut Data, client: &mut mpd::Client) {
    let queue = client.queue().unwrap();
    let current = client.currentsong().unwrap().unwrap_or(Song::default());
    let mut items = vec![];
    for song in queue {
        items.push(LibItem::new(song.title.unwrap_or("".into()), LibKind::None))
    }
    data.queue.items = items;
    if data.queue.items.len() > 0 {
        data.queue.items[current.place.unwrap_or(QueuePlace::default()).pos as usize].style =
            Style::default()
                .fg(Color::Rgb(45, 78, 32))
                .bg(data.colors.background);
    }
}

pub struct Queue {
    pub items: Vec<String>,
    pub state: ListState,
}

impl<'a> Queue {
    pub fn new(items: Vec<String>) -> Queue {
        Queue {
            state: ListState::default(),
            items,
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let queue = client.queue().unwrap();
        let mut items: Vec<String> = vec![];
        for queue in queue {
            let mut q = queue.title.unwrap();
            q.insert_str(0, " ");
            items.push(q);
        }

        self.items = items;
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct Settings {
    pub items: Vec<String>,
    pub state: ListState,
}

impl<'a> Settings {
    pub fn new() -> Settings {
        let settings = vec![" Bluetooth", " Music", " Device", " Other", " Search"];
        Settings {
            state: ListState::default(),
            items: settings.iter().map(|x| String::from(*x)).collect(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}
