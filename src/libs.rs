use crate::library::{LibItem, LibKind, LibState};
use mpd::{client::Client,song::QueuePlace, song::Song, status::Status, Query, Term};
use std::{borrow::Cow::Borrowed, io::Seek};
use std::iter::FromIterator;
use std::str::FromStr;
use tui::{style::Color, style::Style, widgets::ListState};

pub struct Data {
    pub library: Library,
    pub sidelist: Vec<LibItem>,
    pub artists: Library,
    pub albums: Library,
    pub titles: Library,
    pub queue: Library,
    pub playlists: Library,
    pub settings: Settings,
    pub status: Status,
    pub colors: ColorScheme,
    pub drained: usize,
    pub tabindex: usize,
    pub current: mpd::song::Song,
    pub style: tui::style::Style,
    pub options: bool,
    pub opts: Options,
    pub path: Path,
    pub side_path: Path,
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
        
        let mut n_artists = 0;
        let mut n_albums = 0;
        let mut n_titles = 0;
        let mut n_plylists = 0;

        for mut artists in artists.unwrap() {
            if artists.is_empty() {
                artists = "-Uknown Artist-".into();
            }
            artistitems.push(artists.clone());
            libraryitems.push(artists);
            n_artists += 1;
        }
        for albums in albums.unwrap() {
            albumitems.push(albums);
            n_albums += 1;
        }
        albumitems.remove(0);
        for titles in titles.unwrap() {
            titleitems.push(titles);
            n_titles += 1;
        }
        if titleitems.len() > 0 {titleitems.remove(0);}
        for queue in queue {
            let q = queue.title.unwrap_or("".into());
            queueitems.push(q);
        }
        for play in playlists {
            playitems.push(play.name);
            n_plylists += 1;
        }

        if n_titles > 999 {
            n_titles /= 1000;
        }

        let library = Library::default();//new(libraryitems, LibKind::Artist);
        let artists = Library::new(artistitems, LibKind::Artist);
        let albums = Library::new(albumitems, LibKind::Album);
        let titles = Library::new(titleitems, LibKind::Title);
        let queue = Library::new(queueitems, LibKind::Title);
        let playlists = Library::new(playitems, LibKind::Playlist);

        let tabindex: usize = 0;
        let status = client.status().unwrap();
        let mut path = Path::new();
        path.update(artists.clone());
        let sidelist = vec![
            LibItem::new(n_plylists.to_string(), LibKind::None), 
            LibItem::new(n_artists.to_string(), LibKind::None),
            LibItem::new(n_albums.to_string(), LibKind::None),
            LibItem::new(n_titles.to_string(), LibKind::None)];

        Data {
            library,
            sidelist,
            artists,
            albums,
            titles,
            playlists,
            queue,
            settings: Settings::new(),
            status,
            tabindex,
            colors: ColorScheme {
                foreground: Color::Rgb(145, 130, 116),
                background: Color::Black,
                highlight: Color::Rgb(211, 189, 151),
                selected: Color::Rgb(79, 76, 75),
            },
            current: match client.currentsong() {
                Ok(song) => match song {
                    Some(song) => song,
                    None => Song::default(),
                },
                Err(_io) => Song::default(),
            },
            style: tui::style::Style::default()
                .fg(Color::Rgb(145, 131, 116))
                .bg(Color::Rgb(40, 40, 40)),
            options: false,
            opts: Options::new(),
            drained: 0,
            path,
            side_path: Path::new(),
        }
    }

    pub fn update(&mut self, client: &mut mpd::Client) {
        let tabindex = self.tabindex;
        *self = Self::new(client);
        self.tabindex = tabindex;
    }

    pub fn nexttab(&mut self) {
        self.tabindex = (self.tabindex + 1) % 4;
    }

    pub fn prevtab(&mut self) {
        match self.tabindex {
            0 => self.tabindex = 3,
            _ => self.tabindex -= 1,
        }
    }

    pub fn up(&mut self, client: &mut Client) {
        //let pos = client.status().unwrap().elapsed.unwrap().num_seconds();
        if self.options {
            self.opts.previous()
        } else {
        match self.tabindex {
            0 => client.rewind(5).unwrap(),
            1 => { self.library.previous(); self.path.update(self.library.clone()) },
            2 => self.queue.previous(),
            3 => self.settings.previous(),
            _ => {}
        }
        }
    }

    pub fn down(&mut self, client: &mut Client) {
        if self.options {
            self.opts.next()
        } else {
        match self.tabindex {
            0 => client.rewind(-5).unwrap(),
            1 => { self.library.next(); self.path.update(self.library.clone()) },
            2 => self.queue.next(),
            3 => self.settings.next(),
            _ => {}
        }
        }
    }

    pub fn index(&self) -> usize {
        self.library.state.selected().unwrap_or(0)
    }

    pub fn selected(&self) -> LibItem {
        self.library.items[self.index()].clone()
    }
}

pub struct ColorScheme {
    pub foreground: Color,
    pub background: Color,
    pub highlight: Color,
    pub selected: Color,
}

pub struct ArtistOptions {
    pub items: Vec<String>,
}

#[derive(Clone)]
pub struct Library {
    pub items: Vec<LibItem>,
    pub sidelist: Vec<LibItem>,
    pub state: LibState,
}

impl Default for Library {
    fn default() -> Self {
        Library {
            items: vec![LibItem::new("󰟄  Playlists".into(), LibKind::Home),
                        LibItem::new("󰀉  Artists".into(), LibKind::Home),
                        LibItem::new("󰀥  Albums".into(), LibKind::Home),
                        LibItem::new("󰎆  Titles".into(), LibKind::Home)],
            sidelist: vec![],
            state: LibState::default(),
        }
    }
}

impl<'a> Library {
    pub fn new(items: Vec<String>, kind: LibKind) -> Library {
        Library {
            state: LibState::default(),
            items: items
                .iter()
                .map(|i| LibItem::new(i.clone(), kind.clone()))
                .collect(),
            sidelist: vec![],
        }
    }

    pub fn push(&mut self, item: LibItem) {
        self.items.push(item);
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
        if len > 0 && self.state.selected().unwrap() > len {
            self.state.select(Some(len - 1));
            self.state.offset(0);
        }
    }

    pub fn newlib(items: Vec<LibItem>) -> Library {
        Library {
            state: LibState::default(),
            items,
            sidelist: vec![],
        }
    }

    pub fn get_albums(&mut self, client: &mut mpd::Client) -> Library {
        let idx = self.items[self.state.selected().unwrap()].clone();
        let mut query = Query::new();
        
        let items: Result<Vec<String>, mpd::error::Error>;
        if idx.tag == LibKind::Home {
            items = client.list(&Term::Tag(Borrowed("Album")), &query);
        } else {
            items = client.list(
            &Term::Tag("Album".into()),
            &query.and(Term::Tag("Artist".into()), Borrowed(idx.content.as_str())),
        );}
        let mut artistalbums: Vec<String> = vec![];
        for albums in items.unwrap() {
            artistalbums.push(albums);
        }

        Library::new(artistalbums, LibKind::Album)
    }

    pub fn get_titles(&mut self, client: &mut mpd::Client) -> Library {
        let idx: String = self.items[self.state.selected().unwrap()].content.clone();

        let name: &str = idx.as_str();

        let mut query = Query::new();
        let query = query
            .and(Term::Tag("Album".into()), Borrowed(name));
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

pub struct Path {
    artists: Option<Library>,
    albums: Option<Library>,
    titles: Option<Library>,
    home: Library,
}

impl Path {
    pub fn new() -> Path {
        Path {
            artists: None,
            albums: None,
            titles: None,
            home: Library::default(),
        }
    }

    pub fn update(&mut self, level: Library) {
        match level.items[0].tag {
            LibKind::Artist => self.artists = Some(level),
            LibKind::Album => self.albums = Some(level),
            LibKind::Title => self.titles = Some(level),
            LibKind::Home => self.home = level,
            _ => {},
        }
    }

    pub fn up(&mut self) -> Library {
        let mut out = Some(self.home.clone());
        match &self.titles {
            Some(_) => { out = self.albums.clone(); self.titles = None; },
            None => match &self.albums {
                Some(_) => { out = self.artists.clone(); self.albums = None; },
                None => match &self.artists {
                    Some(_) => { out = Some(self.home.clone()); self.artists = None; },
                    None => {},
                }
            }
        }
        match out {
            Some(o) => o,
            None => self.home.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Artists {
    pub items: Vec<LibItem>,
    pub state: LibState,
    pub options: Vec<String>,
}

impl From<Library> for Artists {
    fn from(l: Library) -> Artists {
        Artists {
            items: l.items,
            state: l.state,
            options: vec![],
        }
    }
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

#[derive(Clone)]
pub struct Albums {
    pub items: Vec<LibItem>,
    pub state: LibState,
}

impl From<Library> for Albums {
    fn from(l: Library) -> Albums {
        Albums {
            items: l.items,
            state: l.state,
        }
    }
}

impl<'a> Albums {
    pub fn new(items: Vec<String>) -> Albums {
        Albums {
            state: LibState::default(),
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

#[derive(Clone)]
pub struct Titles {
    pub items: Vec<LibItem>,
    pub state: LibState,
}

impl From<Library> for Titles {
    fn from(l: Library) -> Titles {
        Titles {
            items: l.items,
            state: l.state,
        }
    }
}

//impl Copy for Titles {}

//impl Clone for Titles {
//    fn clone(&self) -> Self {
//        *self
//    }
//}

impl<'a> Titles {
    pub fn new(items: Vec<String>) -> Titles {
        Titles {
            state: LibState::default(),
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
        items.push(LibItem::new(song.title.unwrap_or("".into()).clone(), LibKind::Title));
    }
    data.queue.items = items;
    data.queue.state.set_playing(Some(current.place.unwrap_or(QueuePlace::default()).pos as usize));
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
        let settings = vec![" ➕ Add to Queue", " ➕ Add to Playlist", " 󰀉  View Artist", " 󰀥  View Album"];
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

pub struct Options {
    pub items: Vec<LibItem>,
    pub state: LibState,
}

impl Options {
    pub fn new() -> Options {
        let opt = vec![
            LibItem::new("   Add to Queue".into(), LibKind::Option),
            LibItem::new(" 󰟄  Add to Playlist".into(), LibKind::Option)
        ];
        Options {
            items: opt,
            state: LibState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    i
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
                    0//self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}
