#![cfg_attr(windows, windows_subsystem = "windows")]

mod icons;
mod media;
mod model;
mod output;
mod remote;
#[cfg(windows)]
mod session;
#[cfg(not(windows))]
#[path = "session_stub.rs"]
mod session;
#[cfg(windows)]
mod discord;
#[cfg(not(windows))]
#[path = "discord_stub.rs"]
mod discord;
mod press;
mod songfile;
mod style;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;

use iced::alignment::{Horizontal, Vertical};
use iced::animation::Easing;
use iced::widget::image::Handle;
use iced::widget::{button, column, container, mouse_area, row, scrollable, slider, stack, text, text_input};
use iced::{Animation, ContentFit, Element, Fill, Font, Point, Rectangle, Subscription, Task, Theme, Vector};

const DRAWER: f32 = 220.0;
const QUEUE: f32 = 320.0;
use model::{format_bytes, format_clock, format_duration, greeting_now, Card, Collection, Home, Kind, Library, Look, Resume, Search, Source, Track};
use press::press;
use output::Output;

fn main() -> iced::Result {
    #[cfg(windows)]
    let _ = unsafe { windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(windows::core::w!("ValDex")) };
    let mut window = iced::window::Settings::default();
    window.size = iced::Size::new(1180.0, 760.0);
    window.min_size = Some(iced::Size::new(960.0, 640.0));
    window.decorations = false;
    #[cfg(windows)]
    {
        window.platform_specific.corner_preference = iced::window::settings::platform::CornerPreference::Round;
    }
    window.icon = iced::window::icon::from_file_data(include_bytes!("../../../valdex-icon.png"), None).ok();
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(|app: &App| if app.look == Look::Light { Theme::Light } else { Theme::Dark })
        .subscription(App::subscription)
        .default_font(Font::with_name(if cfg!(windows) { "Segoe UI" } else { "Helvetica Neue" }))
        .window(window)
        .run()
}

struct App {
    catalog: Source,
    mood: String,
    query: String,
    generation: u64,
    screen: Screen,
    tab: SearchTab,
    page_status: String,
    page_error: String,
    playback_error: String,
    busy: String,
    home: Option<Home>,
    results: Option<Search>,
    collection: Option<Collection>,
    pending_card: Option<Card>,
    queue: Vec<Track>,
    index: usize,
    now: Option<Track>,
    playing: bool,
    shuffle: bool,
    repeat: Repeat,
    queue_open: bool,
    position: f32,
    seek_target: Option<f32>,
    volume: f32,
    muted: bool,
    library: Library,
    menu: Option<Track>,
    menu_anchor: Point,
    download_key: Option<String>,
    notice: String,
    audio: Option<Output>,
    covers: HashMap<String, Handle>,
    pictures: HashMap<String, Vec<u8>>,
    pending: HashSet<String>,
    seek_hold: Instant,
    session: Option<session::Session>,
    discord: discord::Presence,
    look: Look,
    pending_seek: Option<f32>,
    restore_once: bool,
    resume_saved: Instant,
    drawer: Animation<bool>,
    queue_anim: Animation<bool>,
    reveal: Animation<bool>,
    above_anim: Animation<bool>,
    above: bool,
    frame: Instant,
    window_id: Option<iced::window::Id>,
    maximized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Search,
    Library,
    Liked,
    Downloads,
    Recents,
    Collection,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchTab {
    All,
    Songs,
    Albums,
    Artists,
    Playlists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repeat {
    Off,
    All,
    One,
}

impl Repeat {
    fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Catalog(Source),
    Mood(String),
    Query(String),
    Submit,
    Searched(u64, Result<Search, String>),
    HomeLoaded(Result<Home, String>),
    Go(Screen),
    Tab(SearchTab),
    Open(Card),
    PlayCard(Card),
    Opened(Result<Collection, String>),
    PlayCollection(Result<Collection, String>),
    PlayTrack(Vec<Track>, usize),
    Audio(Track, Result<(Vec<u8>, f32), String>),
    Toggle,
    Next,
    Previous,
    Seek(f32),
    SeekRelease,
    Volume(f32),
    Mute,
    Shuffle,
    CycleRepeat,
    Jump(usize),
    Remove(usize),
    ClearUpcoming,
    Queue(bool),
    Like(Track),
    Menu(Track),
    Dismiss,
    PlayNext(Track),
    Enqueue(Track),
    Download(Track),
    Stored(Track, Result<(usize, Option<Vec<u8>>), String>),
    Media,
    Retry,
    Tick,
    Cover(String, Result<Vec<u8>, String>),
    Kept,
    Theme(Look),
    OpenFolder,
    ToggleAbove,
    ClearSearches,
    ToggleSidebar,
    Frame(Instant),
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
    Maximized(bool),
    Zoom,
    Resize(iced::window::Direction),
    Window(iced::window::Id),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let library = remote::load_library();
        let (covers, pictures) = saved_covers(&library);
        let catalog = library.catalog.unwrap_or(Source::Youtube);
        let look = library.look;
        let resume = library.resume.clone();
        let drawer_open = library.drawer;
        let above = library.above;
        let volume = if library.volume.is_finite() { library.volume.clamp(0.0, 1.0) } else { 0.8 };
        style::apply(look);
        let mut app = Self {
            catalog,
            mood: String::new(),
            query: String::new(),
            generation: 0,
            screen: Screen::Home,
            tab: SearchTab::All,
            page_status: format!("Loading {}…", catalog.label()),
            page_error: String::new(),
            playback_error: String::new(),
            busy: String::new(),
            home: None,
            results: None,
            collection: None,
            pending_card: None,
            queue: Vec::new(),
            index: 0,
            now: None,
            playing: false,
            shuffle: false,
            repeat: Repeat::Off,
            queue_open: false,
            position: 0.0,
            seek_target: None,
            volume,
            muted: false,
            library,
            menu: None,
            menu_anchor: Point::ORIGIN,
            download_key: None,
            notice: String::new(),
            audio: None,
            covers,
            pictures,
            pending: HashSet::new(),
            seek_hold: Instant::now(),
            session: None,
            discord: discord::Presence::start(),
            look,
            pending_seek: None,
            restore_once: false,
            resume_saved: Instant::now(),
            drawer: slide(drawer_open, 360),
            queue_anim: slide(false, 360),
            reveal: slide(true, 280),
            above_anim: slide(above, 220),
            above,
            frame: Instant::now(),
            window_id: None,
            maximized: false,
        };
        let resume_task = match resume {
            Some(saved) if !saved.queue.is_empty() => {
                app.index = saved.index.min(saved.queue.len() - 1);
                app.queue = saved.queue;
                let at = saved.position;
                app.pending_seek = if at.is_finite() && at > 1.0 { Some(at) } else { None };
                app.restore_once = true;
                app.launch()
            }
            _ => Task::none(),
        };
        let home = app.load_home();
        let covers = app.cover_task();
        let saved = app.library.downloads.clone();
        let upgrade = Task::perform(
            async move {
                for track in &saved {
                    let _ = remote::upgrade_download(track);
                }
            },
            |_| Message::Kept,
        );
        (app, Task::batch([home, covers, upgrade, resume_task]))
    }

    fn title(&self) -> String {
        match &self.now {
            Some(track) => format!("{} · ValDex", track.title),
            None => "ValDex".into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keys = iced::event::listen_with(on_key);
        let pointer = iced::event::listen_raw(track_pointer);
        let media = iced::time::every(std::time::Duration::from_millis(150)).map(|_| Message::Media);
        let window = iced::window::open_events().map(Message::Window);
        let zoom = iced::window::resize_events().map(|_| Message::Zoom);
        let mut parts = vec![keys, pointer, media, window, zoom];
        if self.audio.is_some() {
            parts.push(iced::time::every(std::time::Duration::from_millis(400)).map(|_| Message::Tick));
        }
        if self.moving() {
            parts.push(iced::window::frames().map(Message::Frame));
        }
        Subscription::batch(parts)
    }

    fn open_menu(&mut self, track: Track) {
        self.menu_anchor = pointer();
        self.menu = Some(track);
        MENU_OPEN.store(true, Ordering::Release);
    }

    fn hide_menu(&mut self) {
        self.menu = None;
        MENU_OPEN.store(false, Ordering::Release);
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Catalog(source) => {
                self.catalog = source;
                self.mood.clear();
                self.query.clear();
                self.results = None;
                self.home = None;
                self.screen = Screen::Home;
                self.nudge();
                self.library.catalog = Some(source);
                let _ = remote::save_library(&self.library);
                self.load_home()
            }
            Message::Mood(filter) => {
                self.mood = if self.mood == filter { String::new() } else { filter };
                self.screen = Screen::Home;
                self.nudge();
                self.load_home()
            }
            Message::Query(query) => {
                self.query = query;
                self.screen = Screen::Search;
                self.arm_search()
            }
            Message::Submit => {
                self.screen = Screen::Search;
                self.arm_search()
            }
            Message::Searched(generation, result) => {
                if generation != self.generation {
                    return Task::none();
                }
                self.page_status.clear();
                match result {
                    Ok(page) => {
                        self.remember_search();
                        self.page_error.clear();
                        self.results = Some(page);
                        self.cover_task()
                    }
                    Err(error) => {
                        self.page_error = error;
                        Task::none()
                    }
                }
            }
            Message::HomeLoaded(result) => {
                self.page_status.clear();
                match result {
                    Ok(page) => {
                        self.page_error.clear();
                        self.home = Some(page);
                        self.cover_task()
                    }
                    Err(error) => {
                        self.page_error = error;
                        Task::none()
                    }
                }
            }
            Message::Go(screen) => {
                self.screen = screen;
                self.nudge();
                self.hide_menu();
                self.notice.clear();
                self.page_error.clear();
                let task = if screen == Screen::Home && self.home.is_none() { self.load_home() } else { Task::none() };
                if screen == Screen::Downloads { Task::batch([task, self.cover_task()]) } else { task }
            }
            Message::Tab(tab) => {
                self.tab = tab;
                Task::none()
            }
            Message::Open(card) => self.open_card(card, false),
            Message::PlayCard(card) => self.open_card(card, true),
            Message::Opened(result) => self.finish_collection(result, false),
            Message::PlayCollection(result) => self.finish_collection(result, true),
            Message::PlayTrack(tracks, index) => self.begin(tracks, index),
            Message::Audio(track, Ok((bytes, gain))) => {
                if self.now.as_ref().is_some_and(|current| !current.same(&track)) {
                    return Task::none();
                }
                let bytes = match media::playable(bytes) {
                    Ok(ready) => ready,
                    Err(error) => {
                        self.busy.clear();
                        self.playing = false;
                        self.playback_error = error;
                        return Task::none();
                    }
                };
                let measured = media::length_seconds(&bytes).map(u64::from);
                self.busy.clear();
                match Output::start(bytes, gain) {
                    Ok(output) => {
                        if let Some(seconds) = measured {
                            self.keep_length(&track, seconds);
                        }
                        output.set_volume(self.audible());
                        if let Some(at) = self.pending_seek.take() {
                            output.seek(at);
                            self.position = at;
                            self.seek_hold = Instant::now();
                        }
                        self.audio = Some(output);
                        self.note_resume();
                        self.playing = true;
                        self.playback_error.clear();
                        self.publish_session();
                    }
                    Err(error) => {
                        self.audio = None;
                        self.playing = false;
                        self.playback_error = error;
                    }
                }
                Task::none()
            }
            Message::Audio(track, Err(error)) => {
                if self.now.as_ref().is_some_and(|current| current.same(&track)) {
                    self.busy.clear();
                    self.playing = false;
                    self.playback_error = error;
                }
                Task::none()
            }
            Message::Toggle => {
                let task = self.toggle();
                self.publish_session();
                task
            }
            Message::Next => self.next_track(),
            Message::Previous => self.previous(),
            Message::Seek(seconds) => {
                self.position = seconds;
                self.seek_target = Some(seconds);
                self.seek_hold = Instant::now();
                Task::none()
            }
            Message::SeekRelease => {
                self.seek_hold = Instant::now();
                if let Some(seconds) = self.seek_target.take() {
                    if let Some(audio) = &self.audio {
                        audio.seek(seconds);
                    }
                }
                self.note_resume();
                self.publish_session();
                Task::none()
            }
            Message::Volume(volume) => {
                self.volume = volume;
                self.muted = false;
                self.library.volume = volume;
                let _ = remote::save_library(&self.library);
                if let Some(audio) = &self.audio {
                    audio.set_volume(volume);
                }
                Task::none()
            }
            Message::Mute => {
                self.muted = !self.muted && self.volume > 0.0;
                if self.volume == 0.0 {
                    self.volume = 0.8;
                    self.muted = false;
                }
                if let Some(audio) = &self.audio {
                    audio.set_volume(self.audible());
                }
                Task::none()
            }
            Message::Shuffle => {
                self.shuffle = !self.shuffle;
                Task::none()
            }
            Message::CycleRepeat => {
                self.repeat = self.repeat.cycle();
                Task::none()
            }
            Message::Jump(index) => {
                if index < self.queue.len() {
                    self.index = index;
                    self.launch()
                } else {
                    Task::none()
                }
            }
            Message::Remove(index) => self.remove_at(index),
            Message::ClearUpcoming => {
                if self.index + 1 < self.queue.len() {
                    self.queue.truncate(self.index + 1);
                }
                Task::none()
            }
            Message::Queue(open) => {
                self.queue_open = open;
                let at = Instant::now();
                self.queue_anim.go_mut(open, at);
                self.frame = at;
                Task::none()
            }
            Message::Like(track) => {
                if let Some(index) = self.library.likes.iter().position(|liked| liked.same(&track)) {
                    self.library.likes.remove(index);
                } else {
                    self.library.likes.insert(0, track);
                }
                if let Err(error) = remote::save_library(&self.library) {
                    self.notice = error;
                }
                Task::none()
            }
            Message::Menu(track) => {
                if self.menu.as_ref().is_some_and(|open| open.same(&track)) {
                    self.hide_menu();
                } else {
                    self.open_menu(track);
                }
                Task::none()
            }
            Message::Dismiss => {
                self.hide_menu();
                Task::none()
            }
            Message::PlayNext(track) => {
                self.hide_menu();
                self.play_next(track)
            }
            Message::Enqueue(track) => {
                self.hide_menu();
                if self.queue.is_empty() || self.now.is_none() {
                    self.begin(vec![track], 0)
                } else {
                    self.queue.push(track);
                    Task::none()
                }
            }
            Message::Download(track) => self.download(track),
            Message::Stored(track, result) => {
                if self.download_key.as_ref().is_some_and(|key| key == &track_key(&track)) {
                    self.download_key = None;
                }
                match result {
                    Ok((size, picture)) => {
                        self.library.downloads.retain(|item| !item.same(&track));
                        self.library.downloads.insert(0, track.clone());
                        if let Some(picture) = picture {
                            self.keep_picture(&track, &picture);
                        }
                        self.notice = format!("Saved · {}", format_bytes(size as u64));
                        if let Err(error) = remote::save_library(&self.library) {
                            self.notice = error;
                        }
                    }
                    Err(error) => self.notice = error,
                }
                Task::none()
            }
            Message::Media => {
                let commands = self.session.as_ref().map(|session| session.poll()).unwrap_or_default();
                let mut task = Task::none();
                for command in commands {
                    task = Task::batch([task, self.media(command)]);
                }
                task
            }
            Message::Retry => match self.screen {
                Screen::Search => self.arm_search(),
                Screen::Collection => {
                    if let Some(card) = self.pending_card.clone() { self.open_card(card, false) } else { self.load_home() }
                }
                _ => self.load_home(),
            },
            Message::Tick => {
                if !self.playing {
                    return Task::none();
                }
                let Some((position, finished)) = self.audio.as_ref().map(|output| (output.position(), output.finished())) else { return Task::none() };
                if self.seek_hold.elapsed() > std::time::Duration::from_millis(350) {
                    self.position = position;
                }
                if self.resume_saved.elapsed() > std::time::Duration::from_secs(5) {
                    self.note_resume();
                }
                let duration = self.now.as_ref().and_then(|track| track.duration).unwrap_or(0) as f32;
                if output::song_ended(finished, position, duration) {
                    self.next_track()
                } else {
                    self.publish_session();
                    Task::none()
                }
            }
            Message::Cover(url, Ok(bytes)) => {
                self.keep_picture_url(&url, &bytes);
                self.covers.insert(url, Handle::from_bytes(bytes));
                self.publish_session();
                Task::none()
            }
            Message::Cover(_, Err(_)) => Task::none(),
            Message::Kept => Task::none(),
            Message::Theme(look) => {
                self.look = look;
                self.library.look = look;
                style::apply(look);
                self.nudge();
                let _ = remote::save_library(&self.library);
                Task::none()
            }
            Message::OpenFolder => {
                if let Err(error) = remote::open_downloads() {
                    self.notice = error;
                }
                Task::none()
            }
            Message::ToggleAbove => {
                self.above = !self.above;
                let at = Instant::now();
                self.above_anim.go_mut(self.above, at);
                self.frame = at;
                self.library.above = self.above;
                let _ = remote::save_library(&self.library);
                self.place()
            }
            Message::ClearSearches => {
                self.library.searches.clear();
                self.notice = match remote::save_library(&self.library) {
                    Ok(()) => "Search history cleared.".into(),
                    Err(error) => error,
                };
                Task::none()
            }
            Message::ToggleSidebar => {
                let at = Instant::now();
                self.drawer.go_mut(!self.drawer.value(), at);
                self.frame = at;
                self.library.drawer = self.drawer.value();
                let _ = remote::save_library(&self.library);
                Task::none()
            }
            Message::Frame(at) => {
                self.frame = at;
                Task::none()
            }
            Message::Drag => self.command(iced::window::drag),
            Message::Minimize => self.command(|id| iced::window::minimize(id, true)),
            Message::ToggleMaximize => self.command(|id| iced::window::toggle_maximize(id).chain(iced::window::is_maximized(id).map(Message::Maximized))),
            Message::Close => iced::exit(),
            Message::Maximized(value) => {
                self.maximized = value;
                Task::none()
            }
            Message::Zoom => self.command(|id| iced::window::is_maximized(id).map(Message::Maximized)),
            Message::Resize(direction) => self.command(move |id| iced::window::drag_resize(id, direction)),
            Message::Window(id) => {
                self.window_id = Some(id);
                iced::window::is_maximized(id).map(Message::Maximized).chain(self.place())
            }
        }
    }

    fn moving(&self) -> bool {
        let at = self.frame;
        self.drawer.is_animating(at) || self.queue_anim.is_animating(at) || self.reveal.is_animating(at) || self.above_anim.is_animating(at)
    }

    fn nudge(&mut self) {
        let at = Instant::now();
        self.reveal = slide(false, 280).go(true, at);
        self.frame = at;
    }

    fn place(&self) -> Task<Message> {
        let level = if self.above { iced::window::Level::AlwaysOnTop } else { iced::window::Level::Normal };
        self.command(move |id| iced::window::set_level(id, level))
    }

    fn command(&self, apply: impl Fn(iced::window::Id) -> Task<Message> + Send + 'static) -> Task<Message> {
        match self.window_id {
            Some(id) => apply(id),
            None => iced::window::latest().and_then(apply),
        }
    }

    fn drawer_width(&self) -> f32 {
        self.drawer.interpolate(0.0, DRAWER, self.frame).clamp(0.0, DRAWER)
    }

    fn load_home(&mut self) -> Task<Message> {
        self.page_error.clear();
        self.page_status = format!("Loading {}…", self.catalog.label());
        let source = self.catalog;
        let mood = self.mood.clone();
        Task::perform(async move { remote::home(source, &mood).await }, Message::HomeLoaded)
    }

    fn arm_search(&mut self) -> Task<Message> {
        self.generation += 1;
        self.tab = SearchTab::All;
        let generation = self.generation;
        let query = self.query.trim().to_string();
        if query.len() < 2 {
            self.results = None;
            self.page_status.clear();
            self.page_error.clear();
            return Task::none();
        }
        self.page_error.clear();
        self.page_status = format!("Searching {}…", self.catalog.label());
        let source = self.catalog;
        Task::perform(
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(280)).await;
                (generation, remote::search(source, &query).await)
            },
            |(generation, result)| Message::Searched(generation, result),
        )
    }

    fn open_card(&mut self, card: Card, play: bool) -> Task<Message> {
        if matches!(card.kind, Kind::Song | Kind::Video) {
            let Some(track) = card.track.clone() else {
                self.playback_error = "That song has no recording.".into();
                return Task::none();
            };
            let tracks = self.siblings(&track);
            let index = tracks.iter().position(|item| item.same(&track)).unwrap_or(0);
            return self.begin(tracks, index);
        }
        let Some(kind) = card.kind.api() else {
            self.page_error = "That page can't be opened.".into();
            return Task::none();
        };
        self.pending_card = Some(card.clone());
        self.screen = Screen::Collection;
        self.nudge();
        self.collection = None;
        self.page_error.clear();
        self.page_status = format!("Opening {}…", card.title);
        let source = card.source();
        let id = card.id.clone();
        Task::perform(async move { remote::collection(kind, source, &id).await }, move |result| {
            if play { Message::PlayCollection(result) } else { Message::Opened(result) }
        })
    }

    fn finish_collection(&mut self, result: Result<Collection, String>, play: bool) -> Task<Message> {
        self.page_status.clear();
        match result {
            Ok(page) => {
                let tracks = page.playable();
                self.page_error.clear();
                self.collection = Some(page);
                let covers = self.cover_task();
                if play {
                    if tracks.is_empty() {
                        self.page_error = "That page has no playable songs.".into();
                        covers
                    } else {
                        Task::batch([covers, self.begin(tracks, 0)])
                    }
                } else {
                    covers
                }
            }
            Err(error) => {
                self.page_error = error;
                Task::none()
            }
        }
    }

    fn siblings(&self, track: &Track) -> Vec<Track> {
        if let Some(home) = &self.home {
            for shelf in &home.shelves {
                let songs = shelf.songs();
                if songs.iter().any(|item| item.same(track)) {
                    return songs;
                }
            }
        }
        if let Some(results) = &self.results {
            let songs = results.tracks();
            if songs.iter().any(|item| item.same(track)) {
                return songs;
            }
        }
        if let Some(collection) = &self.collection {
            let songs = collection.playable();
            if songs.iter().any(|item| item.same(track)) {
                return songs;
            }
        }
        if self.screen == Screen::Liked && self.library.likes.iter().any(|item| item.same(track)) {
            return self.library.likes.clone();
        }
        if self.screen == Screen::Recents && self.library.recents.iter().any(|item| item.same(track)) {
            return self.library.recents.clone();
        }
        if self.screen == Screen::Downloads && self.library.downloads.iter().any(|item| item.same(track)) {
            return self.library.downloads.clone();
        }
        vec![track.clone()]
    }

    fn begin(&mut self, mut tracks: Vec<Track>, mut index: usize) -> Task<Message> {
        if tracks.is_empty() {
            return Task::none();
        }
        index = index.min(tracks.len() - 1);
        if self.shuffle && tracks.len() > 1 {
            let chosen = tracks.remove(index);
            shuffle(&mut tracks);
            tracks.insert(0, chosen);
            index = 0;
        }
        self.queue = tracks;
        self.index = index;
        self.launch()
    }

    fn launch(&mut self) -> Task<Message> {
        let Some(track) = self.queue.get(self.index).cloned() else { return Task::none() };
        let restore = self.restore_once;
        self.restore_once = false;
        if !restore {
            self.pending_seek = None;
        }
        self.now = Some(track.clone());
        self.audio = None;
        self.playing = false;
        self.position = 0.0;
        self.seek_target = None;
        self.playback_error.clear();
        self.busy = format!("Starting {}…", track.title);
        self.remember(&track);
        let requested = track.clone();
        if remote::has_download(&track) {
            return Task::perform(async move { with_level(remote::read_download(&requested).ok_or_else(|| "That download is missing.".to_string())).await }, move |result| Message::Audio(track, result));
        }
        Task::perform(async move { with_level(remote::audio(&requested).await).await }, move |result| Message::Audio(track, result))
    }

    fn play_next(&mut self, track: Track) -> Task<Message> {
        if self.queue.is_empty() || self.now.is_none() {
            return self.begin(vec![track], 0);
        }
        let at = (self.index + 1).min(self.queue.len());
        self.queue.insert(at, track);
        Task::none()
    }

    fn download(&mut self, track: Track) -> Task<Message> {
        self.hide_menu();
        if self.download_key.is_some() {
            return Task::none();
        }
        if remote::has_download(&track) {
            remote::delete_download(&track);
            self.covers.remove(&remote::cover_key(&track));
            self.pictures.remove(&remote::cover_key(&track));
            self.library.downloads.retain(|item| !item.same(&track));
            if let Err(error) = remote::save_library(&self.library) {
                self.notice = error;
            } else {
                self.notice = format!("Removed {}", track.title);
            }
            return Task::none();
        }
        self.download_key = Some(track_key(&track));
        self.notice = format!("Downloading {}…", track.title);
        let requested = track.clone();
        let cached = remote::absolute_media(&track.thumbnail).and_then(|url| self.pictures.get(&url).cloned());
        Task::perform(
            async move {
                let bytes = remote::audio(&requested).await?;
                let picture = saved_or_fetched(&requested, cached).await;
                remote::save_download(&requested, &bytes, picture.as_deref())?;
                Ok((bytes.len(), picture))
            },
            move |result| Message::Stored(track, result),
        )
    }

    fn replay_current(&mut self) -> bool {
        let volume = self.audible();
        let Some(audio) = self.audio.as_mut() else { return false };
        if audio.replay().is_err() {
            return false;
        }
        audio.set_volume(volume);
        self.position = 0.0;
        self.playing = true;
        self.busy.clear();
        self.playback_error.clear();
        self.publish_session();
        true
    }

    fn next_track(&mut self) -> Task<Message> {
        if self.queue.is_empty() {
            return Task::none();
        }
        if self.repeat == Repeat::One {
            if self.replay_current() {
                return Task::none();
            }
            return self.launch();
        }
        if self.index + 1 < self.queue.len() {
            self.index += 1;
            self.launch()
        } else if self.repeat == Repeat::All {
            self.index = 0;
            self.launch()
        } else {
            self.playing = false;
            if let Some(audio) = &self.audio {
                audio.pause();
            }
            self.publish_session();
            Task::none()
        }
    }

    fn previous(&mut self) -> Task<Message> {
        if self.position > 3.0 {
            self.position = 0.0;
            if let Some(audio) = &self.audio {
                audio.seek(0.0);
            }
            self.note_resume();
            self.publish_session();
            return Task::none();
        }
        if self.index > 0 {
            self.index -= 1;
            self.launch()
        } else if self.repeat == Repeat::All && !self.queue.is_empty() {
            self.index = self.queue.len() - 1;
            self.launch()
        } else {
            Task::none()
        }
    }

    fn remove_at(&mut self, index: usize) -> Task<Message> {
        if index >= self.queue.len() {
            return Task::none();
        }
        self.queue.remove(index);
        if self.queue.is_empty() {
            self.now = None;
            self.audio = None;
            self.playing = false;
            self.index = 0;
            self.note_resume();
            self.publish_session();
            return Task::none();
        }
        if index < self.index {
            self.index -= 1;
            Task::none()
        } else if index == self.index {
            self.index = index.min(self.queue.len() - 1);
            self.launch()
        } else {
            Task::none()
        }
    }

    fn toggle(&mut self) -> Task<Message> {
        let mut paused = false;
        if let Some(audio) = &self.audio {
            if self.playing {
                audio.pause();
                self.playing = false;
                paused = true;
            } else {
                audio.resume();
                self.playing = true;
            }
        }
        if paused {
            self.note_resume();
        }
        Task::none()
    }

    fn audible(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }

    fn keep_length(&mut self, track: &Track, seconds: u64) {
        let write = |item: &mut Track| {
            if item.same(track) && item.duration.unwrap_or(0) == 0 {
                item.duration = Some(seconds);
            }
        };
        if let Some(current) = self.now.as_mut() {
            write(current);
        }
        for item in &mut self.queue {
            write(item);
        }
        if let Some(results) = self.results.as_mut() {
            for item in results.songs.iter_mut().chain(results.videos.iter_mut()) {
                write(item);
            }
        }
        if let Some(collection) = self.collection.as_mut() {
            for item in &mut collection.tracks {
                write(item);
            }
        }
        for item in self.library.recents.iter_mut().chain(self.library.likes.iter_mut()).chain(self.library.downloads.iter_mut()) {
            write(item);
        }
    }

    fn remember(&mut self, track: &Track) {
        self.library.recents.retain(|item| !item.same(track));
        self.library.recents.insert(0, track.clone());
        self.library.recents.truncate(30);
        self.note_resume();
    }

    fn note_resume(&mut self) {
        if self.queue.is_empty() {
            if self.library.resume.take().is_some() {
                let _ = remote::save_library(&self.library);
            }
            return;
        }
        self.library.resume = Some(Resume {
            queue: self.queue.clone(),
            index: self.index.min(self.queue.len() - 1),
            position: if self.position.is_finite() { self.position.max(0.0) } else { 0.0 },
        });
        self.resume_saved = Instant::now();
        if let Err(error) = remote::save_library(&self.library) {
            self.playback_error = error;
        }
    }

    fn remember_search(&mut self) {
        let query = self.query.trim();
        if query.len() < 2 {
            return;
        }
        let query = query.to_string();
        self.library.searches.retain(|item| item != &query);
        self.library.searches.insert(0, query);
        self.library.searches.truncate(8);
        let _ = remote::save_library(&self.library);
    }

    fn cover_task(&mut self) -> Task<Message> {
        let mut urls = Vec::new();
        let mut push = |raw: &str| {
            let Some(url) = remote::absolute_media(raw) else { return };
            if self.covers.contains_key(&url) || self.pending.contains(&url) || urls.len() >= 64 {
                return;
            }
            urls.push(url);
        };
        if let Some(home) = &self.home {
            for shelf in &home.shelves {
                for item in &shelf.items {
                    push(item.art());
                }
            }
        }
        if let Some(results) = &self.results {
            for track in results.tracks() {
                push(&track.thumbnail);
            }
            for card in results.albums.iter().chain(&results.artists).chain(&results.playlists) {
                push(card.art());
            }
        }
        if let Some(collection) = &self.collection {
            push(&collection.thumbnail);
            for track in &collection.tracks {
                push(&track.thumbnail);
            }
        }
        for track in self.library.likes.iter().chain(&self.library.recents).chain(&self.queue) {
            push(&track.thumbnail);
        }
        for track in &self.library.downloads {
            if self.covers.contains_key(&remote::cover_key(track)) {
                continue;
            }
            push(&track.thumbnail);
        }
        if let Some(track) = &self.now {
            push(&track.thumbnail);
        }
        for url in &urls {
            self.pending.insert(url.clone());
        }
        Task::batch(urls.into_iter().map(|url| {
            let request = url.clone();
            Task::perform(async move { remote::image(&request).await }, move |bytes| Message::Cover(url, bytes))
        }))
    }

    fn liked(&self, track: &Track) -> bool {
        self.library.likes.iter().any(|item| item.same(track))
    }

    fn downloading(&self, track: &Track) -> bool {
        self.download_key.as_ref().is_some_and(|key| key == &track_key(track))
    }

    fn view(&self) -> Element<'_, Message> {
        let middle = self.middle();
        let shell = container(column![self.header(), self.chips(), hairline(), middle, hairline(), self.player()].spacing(0).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(style::mica);
        let chrome = if self.maximized {
            shell.into()
        } else {
            stack([shell.into(), self.resize_border()]).width(Fill).height(Fill).into()
        };
        let Some(track) = self.menu.as_ref() else {
            return chrome;
        };
        let anchor = self.menu_anchor;
        let popup = iced::widget::Float::new(self.track_menu(track))
            .style(|_theme| iced::widget::float::Style {
                shadow: iced::Shadow {
                    color: iced::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.45 },
                    offset: Vector::new(0.0, 10.0),
                    blur_radius: 28.0,
                },
                shadow_border_radius: 12.0.into(),
            })
            .translate(move |bounds, viewport| place_menu(anchor, bounds, viewport));
        let dismiss = mouse_area(iced::widget::Space::new().width(Fill).height(Fill)).on_press(Message::Dismiss);
        stack([chrome, stack([dismiss.into(), popup.into()]).width(Fill).height(Fill).into()]).width(Fill).height(Fill).into()
    }

    fn middle(&self) -> Element<'_, Message> {
        let width = self.queue_anim.interpolate(0.0, QUEUE, self.frame).clamp(0.0, QUEUE);
        let stage = self.stage();
        if width < 1.0 {
            return stage;
        }
        let page = row![stage, iced::widget::Space::new().width(width).height(Fill)].width(Fill).height(Fill);
        let shown = width;
        let panel = iced::widget::Float::new(self.queue_panel()).translate(move |bounds, viewport| Vector::new(viewport.x + viewport.width - shown - bounds.x, 0.0));
        stack([page.into(), panel.into()]).width(Fill).height(Fill).clip(true).into()
    }

    fn stage(&self) -> Element<'_, Message> {
        let width = self.drawer_width();
        let page = row![iced::widget::Space::new().width(width).height(Fill), self.content()].width(Fill).height(Fill);
        let shift = if width < 1.0 { -DRAWER - 8.0 } else { width - DRAWER };
        let drawer = iced::widget::Float::new(self.sidebar()).translate(move |_bounds, _viewport| Vector::new(shift, 0.0));
        stack([page.into(), drawer.into()]).width(Fill).height(Fill).clip(true).into()
    }

    fn header(&self) -> Element<'_, Message> {
        let brand = row![mark(), text("ValDex").size(15).font(style::strong())].spacing(8).align_y(Vertical::Center);
        let search = text_input(&format!("Search {}", self.catalog.label()), &self.query)
            .on_input(Message::Query)
            .on_submit(Message::Submit)
            .padding([8, 16])
            .size(14)
            .style(style::search)
            .width(Fill);
        let bar = row![self.menu_button(), brand, container(search).width(Fill), self.window_controls()]
            .spacing(10)
            .align_y(Vertical::Center)
            .padding(pad(4.0, 8.0, 4.0, 10.0))
            .width(Fill);
        mouse_area(bar).on_press(Message::Drag).on_double_click(Message::ToggleMaximize).into()
    }

    fn menu_button(&self) -> Element<'_, Message> {
        let open = self.drawer.interpolate(0.0, 1.0, self.frame).clamp(0.0, 1.0);
        let middle = 8.0 + 8.0 * open;
        let gap = 3.0 + open;
        let bar = |width: f32| container(iced::widget::Space::new()).width(width).height(2.0).style(style::fill(style::fg(), 1.0));
        let glyph = column![bar(16.0), bar(middle), bar(16.0)].spacing(gap).align_x(Horizontal::Center);
        let icon = container(glyph).width(36.0).height(36.0).align_x(Horizontal::Center).align_y(Vertical::Center);
        press(button(icon).on_press(Message::ToggleSidebar).padding(0).style(style::ghost))
    }

    fn window_controls(&self) -> Element<'_, Message> {
        let color = style::fg();
        let minimize = self.caption(icons::draw(icons::Glyph::Minimize, 14.0, color), Message::Minimize, false);
        let zoom_glyph = if self.maximized { icons::Glyph::Restore } else { icons::Glyph::Maximize };
        let zoom = self.caption(icons::draw(zoom_glyph, 14.0, color), Message::ToggleMaximize, false);
        let close = self.caption(icons::draw(icons::Glyph::Close, 14.0, color), Message::Close, true);
        row![minimize, zoom, close].spacing(2).into()
    }

    fn caption<'a>(&self, icon: Element<'a, Message>, message: Message, close: bool) -> Element<'a, Message> {
        let icon = container(icon).width(40.0).height(32.0).align_x(Horizontal::Center).align_y(Vertical::Center);
        let button = button(icon).on_press(message).padding(0);
        press(if close { button.style(style::caption_close) } else { button.style(style::ghost) })
    }

    fn resize_border(&self) -> Element<'_, Message> {
        const EDGE: f32 = 5.0;
        const CORNER: f32 = 10.0;
        let grip = |width: iced::Length, height: iced::Length, direction, cursor| {
            mouse_area(iced::widget::Space::new().width(width).height(height)).on_press(Message::Resize(direction)).interaction(cursor)
        };
        column![
            row![
                grip(CORNER.into(), EDGE.into(), iced::window::Direction::NorthWest, iced::mouse::Interaction::ResizingDiagonallyDown),
                grip(Fill, EDGE.into(), iced::window::Direction::North, iced::mouse::Interaction::ResizingVertically),
                grip(CORNER.into(), EDGE.into(), iced::window::Direction::NorthEast, iced::mouse::Interaction::ResizingDiagonallyUp),
            ]
            .width(Fill),
            row![
                grip(EDGE.into(), Fill, iced::window::Direction::West, iced::mouse::Interaction::ResizingHorizontally),
                iced::widget::Space::new().width(Fill).height(Fill),
                grip(EDGE.into(), Fill, iced::window::Direction::East, iced::mouse::Interaction::ResizingHorizontally),
            ]
            .width(Fill)
            .height(Fill),
            row![
                grip(CORNER.into(), EDGE.into(), iced::window::Direction::SouthWest, iced::mouse::Interaction::ResizingDiagonallyUp),
                grip(Fill, EDGE.into(), iced::window::Direction::South, iced::mouse::Interaction::ResizingVertically),
                grip(CORNER.into(), EDGE.into(), iced::window::Direction::SouthEast, iced::mouse::Interaction::ResizingDiagonallyDown),
            ]
            .width(Fill),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn chips(&self) -> Element<'_, Message> {
        let chips = Source::ALL.map(|source| {
            let button = button(text(source.label()).size(14)).on_press(Message::Catalog(source)).padding([6, 14]);
            press(if source == self.catalog { button.style(style::chip_on) } else { button.style(style::chip_off) })
        });
        container(row(chips).spacing(8).padding(pad(0.0, 16.0, 10.0, 16.0)).wrap()).width(Fill).into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let library_on = matches!(self.screen, Screen::Library | Screen::Liked | Screen::Downloads | Screen::Recents);
        let nav = column![
            self.nav(Screen::Home, "Home", self.screen == Screen::Home),
            self.nav(Screen::Search, "Search", self.screen == Screen::Search),
            self.nav(Screen::Library, "Library", library_on),
            text("ON THIS PC").size(11).color(style::faint()),
            self.nav(Screen::Liked, "Liked songs", self.screen == Screen::Liked),
            self.nav(Screen::Downloads, "Downloads", self.screen == Screen::Downloads),
            self.nav(Screen::Recents, "Recently played", self.screen == Screen::Recents),
        ]
        .spacing(4);
        let body = column![nav, iced::widget::Space::new().height(Fill), self.settings_nav()]
            .spacing(4)
            .padding(12)
            .width(DRAWER - 1.0)
            .height(Fill);
        row![body, hairline_vertical()].width(DRAWER).height(Fill).into()
    }

    fn settings_nav(&self) -> Element<'_, Message> {
        let on = self.screen == Screen::Settings;
        let color = if on { style::fg() } else { style::muted() };
        let label = row![icons::draw(icons::Glyph::Settings, 16.0, color), text("Settings").size(14)].spacing(8).align_y(Vertical::Center);
        let button = button(label).on_press(Message::Go(Screen::Settings)).padding([8, 12]).width(Fill);
        press(if on { button.style(style::nav_on) } else { button.style(style::nav_off) })
    }

    fn nav<'a>(&'a self, screen: Screen, label: &'a str, on: bool) -> Element<'a, Message> {
        let button = button(text(label).size(14)).on_press(Message::Go(screen)).padding([8, 12]).width(Fill);
        press(if on { button.style(style::nav_on) } else { button.style(style::nav_off) })
    }

    fn content(&self) -> Element<'_, Message> {
        let drop = self.reveal.interpolate(18.0, 0.0, self.frame).clamp(0.0, 18.0);
        let page = container(self.page()).padding(pad(8.0 + drop, 28.0, 24.0, 28.0)).width(Fill);
        scrollable(page).width(Fill).height(Fill).into()
    }

    fn page(&self) -> Element<'_, Message> {
        let mut parts: Vec<Element<'_, Message>> = Vec::new();
        if !self.notice.is_empty() {
            parts.push(text(&self.notice).size(14).color(style::amber()).into());
        }
        if !self.page_error.is_empty() {
            parts.push(self.error_block());
        } else if !self.page_status.is_empty() && !self.has_page() {
            parts.push(text(&self.page_status).size(16).color(style::muted()).into());
        }
        parts.push(match self.screen {
            Screen::Home => self.home_page(),
            Screen::Search => self.search_page(),
            Screen::Library => self.library_page(),
            Screen::Liked => self.track_page("Liked songs", "Saved on this PC.", &self.library.likes),
            Screen::Downloads => self.downloads_page(),
            Screen::Recents => self.track_page("Recently played", "The last songs started on this PC.", &self.library.recents),
            Screen::Collection => self.collection_page(),
            Screen::Settings => self.settings_page(),
        });
        column(parts).spacing(16).width(Fill).into()
    }

    fn has_page(&self) -> bool {
        match self.screen {
            Screen::Home => self.home.is_some(),
            Screen::Search => self.query.trim().len() < 2 || self.results.is_some(),
            Screen::Collection => self.collection.is_some(),
            _ => true,
        }
    }

    fn settings_page(&self) -> Element<'_, Message> {
        let choices = row(Look::ALL.map(|look| self.theme_choice(look))).spacing(10).wrap();
        let folder = remote::downloads_folder().display().to_string();
        let searches = self.library.searches.len();
        column![
            text("Settings").size(32).font(style::strong()),
            text("Appearance").size(16).font(style::strong()),
            text(self.look.note()).size(13).color(style::muted()),
            choices,
            text("Window").size(16).font(style::strong()),
            self.above_row(),
            text("Library").size(16).font(style::strong()),
            text(folder).size(13).color(style::muted()).width(Fill),
            row![
                press(button(text("Open folder").size(14)).on_press(Message::OpenFolder).padding([8, 14]).style(style::chip_on)),
                press(button(text("Clear search history").size(14)).on_press(Message::ClearSearches).padding([8, 14]).style(style::chip_off)),
            ]
            .spacing(8)
            .wrap(),
            text(format!("{searches} saved searches. Volume stays where you left it.")).size(12).color(style::faint()).width(Fill),
        ]
        .spacing(12)
        .width(Fill)
        .into()
    }

    fn above_row(&self) -> Element<'_, Message> {
        let travel = self.above_anim.interpolate(0.0, 1.0, self.frame).clamp(0.0, 1.0);
        let knob = container(text("")).width(14.0).height(14.0).style(style::dot(if self.above { style::on_accent() } else { style::fg() }));
        let track = container(row![iced::widget::Space::new().width(3.0 + 18.0 * travel).height(14.0), knob].align_y(Vertical::Center))
            .width(40.0)
            .height(22.0)
            .align_y(Vertical::Center)
            .clip(true)
            .style(style::fill(if self.above { style::highlight() } else { style::faint() }, 11.0));
        let switch = press(button(track).on_press(Message::ToggleAbove).padding(0).style(style::ghost));
        row![
            column![text("Stay on top").size(14).font(style::strong()), text("Keep ValDex above other windows.").size(12).color(style::muted())].spacing(2).width(Fill),
            switch,
        ]
        .align_y(Vertical::Center)
        .width(Fill)
        .into()
    }

    fn theme_choice(&self, look: Look) -> Element<'_, Message> {
        let on = self.look == look;
        let preview = container(text("")).width(108.0).height(52.0).style(style::sample(look));
        let mark = container(text("")).width(8.0).height(8.0).style(style::dot(style::swatch(look)));
        let body = column![preview, row![mark, text(look.label()).size(13).font(style::strong())].spacing(6).align_y(Vertical::Center)].spacing(8).width(108);
        let button = button(body).on_press(Message::Theme(look)).padding(8);
        press(if on { button.style(style::nav_on) } else { button.style(style::nav_off) })
    }

    fn error_block(&self) -> Element<'_, Message> {
        let title = match self.screen {
            Screen::Search => "Search didn't finish",
            Screen::Collection => "That page didn't open",
            _ => "This catalog didn't answer",
        };
        column![
            text(title).size(28).font(style::strong()),
            text(&self.page_error).size(14).color(style::muted()),
            press(button(text("Try again").size(14)).on_press(Message::Retry).padding([8, 14]).style(style::chip_on)),
        ]
        .spacing(8)
        .into()
    }

    fn home_page(&self) -> Element<'_, Message> {
        let mut parts: Vec<Element<'_, Message>> = vec![
            text("Windows 11 edition").size(13).color(style::muted()).into(),
            text(greeting_now()).size(36).font(style::strong()).into(),
            text(self.catalog.blurb()).size(14).color(style::muted()).into(),
        ];
        if let Some(home) = &self.home {
            if !home.filters.is_empty() {
                let filters: Vec<Element<'_, Message>> = home
                    .filters
                    .iter()
                    .map(|filter| {
                        let button = button(text(filter).size(14)).on_press(Message::Mood(filter.clone())).padding([6, 14]);
                        press(if filter == &self.mood { button.style(style::chip_pink) } else { button.style(style::chip_off) })
                    })
                    .collect();
                parts.push(row(filters).spacing(8).wrap().into());
            }
            if home.shelves.is_empty() {
                parts.push(text("Nothing on this shelf. Try another filter, or search for something specific.").size(14).color(style::muted()).into());
            }
            for shelf in &home.shelves {
                parts.push(self.shelf(shelf.title.as_str(), &shelf.items));
            }
        }
        column(parts).spacing(14).width(Fill).into()
    }

    fn search_page(&self) -> Element<'_, Message> {
        let query = self.query.trim();
        if query.len() < 2 {
            let mut parts: Vec<Element<'_, Message>> = vec![
                text("Search").size(32).font(style::strong()).into(),
                text(format!("Look up a song, an album, an artist, or a playlist on {}.", self.catalog.label())).size(14).color(style::muted()).into(),
            ];
            if !self.library.searches.is_empty() {
                let chips: Vec<Element<'_, Message>> = self
                    .library
                    .searches
                    .iter()
                    .map(|item| press(button(chip_label(item)).on_press(Message::Query(item.clone())).padding([6, 14]).style(style::chip_off)))
                    .collect();
                parts.push(row(chips).spacing(8).wrap().into());
            }
            return column(parts).spacing(12).width(Fill).into();
        }
        let Some(results) = &self.results else {
            return column![].spacing(0).into();
        };
        let mut parts: Vec<Element<'_, Message>> = vec![
            text(format!("Results for “{query}”")).size(32).font(style::strong()).into(),
            text(self.catalog.label()).size(13).color(style::muted()).into(),
            self.search_tabs(),
        ];
        let songs = results.tracks();
        let show_songs = matches!(self.tab, SearchTab::All | SearchTab::Songs);
        let show_albums = matches!(self.tab, SearchTab::All | SearchTab::Albums);
        let show_artists = matches!(self.tab, SearchTab::All | SearchTab::Artists);
        let show_playlists = matches!(self.tab, SearchTab::All | SearchTab::Playlists);
        if show_songs {
            if songs.is_empty() && self.tab == SearchTab::Songs {
                parts.push(text("This search didn't include songs.").size(14).color(style::muted()).into());
            } else if !songs.is_empty() {
                parts.push(self.tracks(&songs));
            }
        }
        if show_albums {
            parts.push(self.optional_shelf("Albums", &results.albums, self.tab == SearchTab::Albums));
        }
        if show_artists {
            parts.push(self.optional_shelf("Artists", &results.artists, self.tab == SearchTab::Artists));
        }
        if show_playlists {
            parts.push(self.optional_shelf("Playlists", &results.playlists, self.tab == SearchTab::Playlists));
        }
        let empty = songs.is_empty() && results.albums.is_empty() && results.artists.is_empty() && results.playlists.is_empty();
        if empty {
            parts.push(text(format!("Nothing came back for “{query}”. Try the artist and the title together.")).size(14).color(style::muted()).into());
        }
        column(parts).spacing(16).width(Fill).into()
    }

    fn search_tabs(&self) -> Element<'_, Message> {
        let tabs = [SearchTab::All, SearchTab::Songs, SearchTab::Albums, SearchTab::Artists, SearchTab::Playlists];
        let chips = tabs.map(|tab| {
            let label = match tab {
                SearchTab::All => "All",
                SearchTab::Songs => "Songs",
                SearchTab::Albums => "Albums",
                SearchTab::Artists => "Artists",
                SearchTab::Playlists => "Playlists",
            };
            let button = button(text(label).size(13)).on_press(Message::Tab(tab)).padding([5, 12]);
            press(if tab == self.tab { button.style(style::chip_on) } else { button.style(style::chip_off) })
        });
        row(chips).spacing(8).wrap().into()
    }

    fn optional_shelf<'a>(&'a self, title: &'a str, cards: &'a [Card], required: bool) -> Element<'a, Message> {
        if cards.is_empty() {
            if required { text(format!("This search didn't include {title}.")).size(14).color(style::muted()).into() } else { column![].into() }
        } else {
            self.shelf(title, cards)
        }
    }

    fn library_page(&self) -> Element<'_, Message> {
        column![
            text("Library").size(32).font(style::strong()),
            text("Saved on this PC.").size(14).color(style::muted()),
            self.library_row("Liked songs", format!("{} saved", self.library.likes.len()), Screen::Liked),
            self.library_row("Downloads", self.download_summary(), Screen::Downloads),
            self.library_row("Recently played", format!("{} songs", self.library.recents.len()), Screen::Recents),
        ]
        .spacing(12)
        .width(Fill)
        .into()
    }

    fn download_summary(&self) -> String {
        let saved: Vec<&Track> = self.library.downloads.iter().filter(|track| remote::has_download(track)).collect();
        if saved.is_empty() {
            return "Nothing saved yet".into();
        }
        let bytes = saved.iter().map(|track| remote::download_len(track)).sum();
        format!("{} songs · {}", saved.len(), format_bytes(bytes))
    }

    fn downloads_page(&self) -> Element<'_, Message> {
        let tracks: Vec<Track> = self.library.downloads.iter().filter(|track| remote::has_download(track)).cloned().collect();
        let body = if tracks.is_empty() {
            "No downloads yet. Open ··· on a song and choose Download."
        } else {
            "These songs stay on this PC. Play uses the saved file."
        };
        self.track_page("Downloads", body, &tracks)
    }

    fn library_row<'a>(&'a self, title: &'a str, detail: String, screen: Screen) -> Element<'a, Message> {
        let body = row![
            column![text(title).size(16).font(style::strong()), text(detail).size(13).color(style::muted())].width(Fill).spacing(2),
            text("Open").size(13).color(style::muted()),
        ]
        .align_y(Vertical::Center)
        .padding([12, 14]);
        mouse_area(container(body).width(Fill).style(style::row_off)).on_press(Message::Go(screen)).into()
    }

    fn collection_page(&self) -> Element<'_, Message> {
        let Some(collection) = &self.collection else {
            return column![].into();
        };
        let tracks = collection.playable();
        let mut info = column![text(&collection.title).size(32).font(style::strong()), text(&collection.subtitle).size(14).color(style::muted())].spacing(8);
        if !tracks.is_empty() {
            info = info.push(press(button(icons::draw(icons::Glyph::Play, 16.0, style::play_icon())).on_press(Message::PlayTrack(tracks.clone(), 0)).padding(10).style(style::play)));
        }
        column![
            row![self.cover(&collection.thumbnail, &collection.title, 112.0, 18.0), info].spacing(16).align_y(Vertical::Center),
            self.tracks(&tracks),
        ]
        .spacing(18)
        .width(Fill)
        .into()
    }

    fn track_page<'a>(&'a self, title: &'a str, body: &'a str, tracks: &[Track]) -> Element<'a, Message> {
        column![text(title).size(32).font(style::strong()), text(body).size(14).color(style::muted()).width(Fill), self.tracks(tracks)].spacing(12).width(Fill).into()
    }

    fn shelf<'a>(&'a self, title: &'a str, cards: &'a [Card]) -> Element<'a, Message> {
        if cards.is_empty() {
            return column![].into();
        }
        let cards: Vec<Element<'_, Message>> = cards.iter().map(|card| self.card(card)).collect();
        let scroller = scrollable(row(cards).spacing(14)).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new())).height(248).width(Fill);
        column![text(title).size(22).font(style::strong()), scroller].spacing(8).into()
    }

    fn card<'a>(&'a self, card: &'a Card) -> Element<'a, Message> {
        let open = Message::Open(card.clone());
        let play = Message::PlayCard(card.clone());
        let subtitle = if card.subtitle.is_empty() { card.kind.label().to_string() } else { card.subtitle.clone() };
        column![
            mouse_area(self.cover(card.art(), &card.title, 160.0, 16.0)).on_press(open.clone()),
            row![
                column![text(&card.title).size(14).font(style::strong()), text(subtitle).size(12).color(style::muted())].width(Fill).spacing(2),
                press(button(icons::draw(icons::Glyph::Play, 14.0, style::play_icon())).on_press(play).padding(8).style(style::play)),
            ]
            .spacing(6)
            .align_y(Vertical::Center),
        ]
        .spacing(8)
        .width(168)
        .into()
    }

    fn tracks(&self, tracks: &[Track]) -> Element<'_, Message> {
        if tracks.is_empty() {
            return text("Nothing in this list yet.").size(14).color(style::muted()).into();
        }
        column(tracks.iter().enumerate().map(|(index, track)| self.track_row(track, index, tracks)).collect::<Vec<_>>()).spacing(2).width(Fill).into()
    }

    fn track_row(&self, track: &Track, index: usize, list: &[Track]) -> Element<'_, Message> {
        let active = self.now.as_ref().is_some_and(|current| current.same(track));
        let title = text(track.title.clone()).size(14).font(style::strong()).width(Fill).color(if active { style::highlight() } else { style::fg() });
        let mut detail = track.artist();
        if let Some(album) = track.album.as_ref().filter(|album| !album.name.is_empty()) {
            detail.push_str(" · ");
            detail.push_str(&album.name);
        }
        let main = row![
            text(format!("{}", index + 1)).size(13).color(if active { style::highlight() } else { style::faint() }).width(28),
            self.track_cover(track, 44.0, 8.0),
            column![title, text(detail).size(12).color(style::muted()).width(Fill)].width(Fill).spacing(2),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
        .width(Fill);
        let liked = self.liked(track);
        let open = self.menu.as_ref().is_some_and(|item| item.same(track));
        let heart = press(
            button(icons::draw(if liked { icons::Glyph::HeartOn } else { icons::Glyph::Heart }, 18.0, if liked { style::highlight() } else { style::faint() }))
                .on_press(Message::Like(track.clone()))
                .padding(6)
                .style(if liked { style::heart_on } else { style::heart_off }),
        );
        let more = press(
            button(icons::draw(icons::Glyph::More, 18.0, if open { style::highlight() } else { style::fg() }))
                .on_press(Message::Menu(track.clone()))
                .padding(6)
                .style(if open { style::chip_on } else { style::ghost }),
        );
        let tail = row![text(format_duration(track.duration)).size(12).color(style::faint()), heart, more].spacing(4).align_y(Vertical::Center);
        let row = row![container(mouse_area(main).on_press(Message::PlayTrack(list.to_vec(), index))).width(Fill), tail].spacing(8).align_y(Vertical::Center).width(Fill);
        container(row).padding([4, 6]).width(Fill).style(if active { style::row_on } else { style::row_off }).into()
    }

    fn track_menu(&self, track: &Track) -> Element<'_, Message> {
        let like = if self.liked(track) { "Remove from liked" } else { "Save to liked songs" };
        let download = if self.downloading(track) {
            "Downloading…"
        } else if remote::has_download(track) {
            "Remove download"
        } else {
            "Download"
        };
        let mut download_button = button(text(download).size(13)).padding([8, 10]).width(Fill).style(style::ghost);
        if !self.downloading(track) {
            download_button = download_button.on_press(Message::Download(track.clone()));
        }
        let items = column![
            self.menu_item("Play next", Message::PlayNext(track.clone())),
            self.menu_item("Add to queue", Message::Enqueue(track.clone())),
            self.menu_item(like, Message::Like(track.clone())),
            press(download_button),
        ]
        .spacing(2);
        container(items).width(220).padding(6).style(style::fill(style::surface(), 12.0)).into()
    }

    fn menu_item(&self, label: &str, message: Message) -> Element<'_, Message> {
        press(button(text(label.to_string()).size(13)).on_press(message).padding([8, 10]).width(Fill).style(style::ghost))
    }

    fn track_cover(&self, track: &Track, size: f32, radius: f32) -> Element<'_, Message> {
        let key = remote::cover_key(track);
        if let Some(handle) = self.covers.get(&key) {
            return iced::widget::image(handle.clone()).width(size).height(size).content_fit(ContentFit::Cover).border_radius(radius).into();
        }
        self.cover(&track.thumbnail, &track.title, size, radius)
    }

    fn keep_picture(&mut self, track: &Track, bytes: &[u8]) {
        let key = remote::cover_key(track);
        self.pictures.insert(key.clone(), bytes.to_vec());
        self.covers.insert(key, Handle::from_bytes(bytes.to_vec()));
        if let Some(url) = remote::absolute_media(&track.thumbnail) {
            self.pictures.insert(url.clone(), bytes.to_vec());
            self.covers.insert(url, Handle::from_bytes(bytes.to_vec()));
        }
    }

    fn keep_picture_url(&mut self, url: &str, bytes: &[u8]) {
        self.pictures.insert(url.to_string(), bytes.to_vec());
        let saved: Vec<Track> = self.library.downloads.iter().filter(|track| remote::absolute_media(&track.thumbnail).as_deref() == Some(url)).cloned().collect();
        for track in saved {
            let _ = remote::save_cover(&track, bytes);
            self.keep_picture(&track, bytes);
        }
    }

    fn art_bytes(&self, track: &Track) -> Option<Vec<u8>> {
        self.pictures.get(&remote::cover_key(track)).cloned().or_else(|| remote::absolute_media(&track.thumbnail).and_then(|url| self.pictures.get(&url).cloned()))
    }

    fn publish_session(&mut self) {
        let Some(track) = self.now.clone() else {
            if let Some(session) = self.session.as_mut() {
                session.clear();
            }
            self.discord.clear();
            return;
        };
        if self.session.is_none() {
            self.session = session::Session::open().ok();
        }
        let key = remote::cover_key(&track);
        let need_art = match self.session.as_ref() {
            Some(session) => session.needs_art(&key),
            None => true,
        };
        let art = if need_art { self.art_bytes(&track) } else { None };
        let playing = self.playing;
        let position = self.position;
        if let Some(session) = self.session.as_mut() {
            session.show(&track, playing, position, art.as_deref());
        }
        let duration = track.duration.unwrap_or(0) as f32;
        let image = remote::absolute_media(&track.thumbnail).unwrap_or_default();
        self.discord.show(&track.title, &track.artist(), &image, playing, position, duration);
    }

    fn media(&mut self, command: session::Command) -> Task<Message> {
        match command {
            session::Command::Play => {
                if !self.playing {
                    let _ = self.toggle();
                }
            }
            session::Command::Pause | session::Command::Stop => {
                if self.playing {
                    let _ = self.toggle();
                }
            }
            session::Command::Next => return self.next_track(),
            session::Command::Previous => return self.previous(),
            session::Command::Seek(seconds) => {
                self.position = seconds;
                self.seek_hold = Instant::now();
                if let Some(audio) = &self.audio {
                    audio.seek(seconds);
                }
            }
        }
        self.publish_session();
        Task::none()
    }

    fn cover(&self, raw: &str, title: &str, size: f32, radius: f32) -> Element<'_, Message> {
        let letter = title.chars().next().unwrap_or('♪').to_string();
        let placeholder = container(text(letter).size(size * 0.34).font(style::strong()).color(style::on_accent()))
            .width(size)
            .height(size)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(style::fill(if title.len() % 2 == 0 { style::highlight() } else { style::purple() }, radius));
        let Some(url) = remote::absolute_media(raw) else { return placeholder.into() };
        let Some(handle) = self.covers.get(&url) else { return placeholder.into() };
        iced::widget::image(handle.clone()).width(size).height(size).content_fit(ContentFit::Cover).border_radius(radius).into()
    }

    fn player(&self) -> Element<'_, Message> {
        let duration = self.now.as_ref().and_then(|track| track.duration).unwrap_or(0) as f32;
        let max = duration.max(self.position).max(1.0);
        let seek = slider(0.0..=max, self.position.clamp(0.0, max), Message::Seek).step(0.1_f32).on_release(Message::SeekRelease).width(Fill).height(18).style(style::seek);
        let cover = self.now.as_ref().map(|track| self.track_cover(track, 48.0, 8.0)).unwrap_or_else(|| self.cover("", "", 48.0, 8.0));
        let title = self.now.as_ref().map(|track| track.title.clone()).unwrap_or_else(|| "Nothing playing".into());
        let artist = if !self.playback_error.is_empty() {
            text(&self.playback_error).size(12).color(style::amber()).width(Fill).wrapping(iced::widget::text::Wrapping::None)
        } else if !self.busy.is_empty() {
            text(&self.busy).size(12).color(style::muted()).width(Fill).wrapping(iced::widget::text::Wrapping::None)
        } else {
            text(self.now.as_ref().map(|track| track.artist()).unwrap_or_else(|| "Pick a song from Home or Search".into()))
                .size(12)
                .color(style::muted())
                .width(Fill)
                .wrapping(iced::widget::text::Wrapping::None)
        };
        let identity = container(
            row![cover, column![text(title).size(14).font(style::strong()).width(Fill).wrapping(iced::widget::text::Wrapping::None), artist].width(Fill).spacing(2)]
                .spacing(10)
                .align_y(Vertical::Center),
        )
        .width(Fill)
        .clip(true);
        let controls = row![
            self.control(icons::Glyph::Shuffle, Message::Shuffle, self.shuffle),
            self.control(icons::Glyph::Previous, Message::Previous, false),
            press(button(icons::draw(if self.playing { icons::Glyph::Pause } else { icons::Glyph::Play }, 18.0, style::play_icon())).on_press(Message::Toggle).padding(12).style(style::play)),
            self.control(icons::Glyph::Next, Message::Next, false),
            self.control(if self.repeat == Repeat::One { icons::Glyph::RepeatOne } else { icons::Glyph::Repeat }, Message::CycleRepeat, self.repeat != Repeat::Off),
        ]
        .spacing(4)
        .align_y(Vertical::Center);
        let liked = self.now.as_ref().is_some_and(|track| self.liked(track));
        let heart: Option<Element<'_, Message>> = self.now.as_ref().map(|track| {
            press(
                button(icons::draw(if liked { icons::Glyph::HeartOn } else { icons::Glyph::Heart }, 18.0, if liked { style::highlight() } else { style::faint() }))
                    .on_press(Message::Like(track.clone()))
                    .padding(6)
                    .style(if liked { style::heart_on } else { style::heart_off }),
            )
        });
        let volume = slider(0.0..=1.0, if self.muted { 0.0 } else { self.volume }, Message::Volume).step(0.01_f32).width(96).style(style::volume);
        let volume_icon = if self.muted || self.volume == 0.0 {
            icons::Glyph::VolumeOff
        } else if self.volume < 0.45 {
            icons::Glyph::VolumeLow
        } else {
            icons::Glyph::Volume
        };
        let mut extras = row![
            text(format!("{} / {}", format_clock(self.position), format_clock(duration))).size(12).color(style::faint()),
            self.control(volume_icon, Message::Mute, false),
            volume,
            self.control(icons::Glyph::Queue, Message::Queue(!self.queue_open), self.queue_open),
        ]
        .spacing(6)
        .align_y(Vertical::Center);
        if let Some(heart) = heart {
            extras = extras.push(heart);
        }
        let side = row![iced::widget::Space::new().width(Fill), extras].width(Fill).align_y(Vertical::Center);
        let bar = row![identity, controls, side].spacing(12).align_y(Vertical::Center).padding(pad(8.0, 16.0, 12.0, 16.0)).width(Fill);
        container(column![seek, bar].spacing(6).width(Fill)).width(Fill).style(style::fill(style::bar(), 0.0)).into()
    }

    fn control(&self, glyph: icons::Glyph, message: Message, accent: bool) -> Element<'_, Message> {
        let color = if accent { style::highlight() } else { style::fg() };
        press(button(icons::draw(glyph, 18.0, color)).on_press(message).padding(8).style(if accent { style::accent } else { style::ghost }))
    }

    fn queue_panel(&self) -> Element<'_, Message> {
        let mut rows: Vec<Element<'_, Message>> = Vec::new();
        if self.queue.is_empty() {
            rows.push(text("The queue is empty.").size(14).color(style::muted()).into());
        }
        for (index, track) in self.queue.iter().enumerate() {
            let current = index == self.index && self.now.as_ref().is_some_and(|playing| playing.same(track));
            let line = row![
                container(mouse_area(column![text(&track.title).size(13).color(if current { style::highlight() } else { style::fg() }), text(track.artist()).size(12).color(style::muted())].width(Fill).spacing(1)).on_press(Message::Jump(index))).width(Fill),
                press(button(icons::draw(icons::Glyph::Close, 14.0, style::fg())).on_press(Message::Remove(index)).padding(4).style(style::ghost)),
            ]
            .spacing(6)
            .align_y(Vertical::Center);
            rows.push(container(line).padding([6, 4]).style(if current { style::row_on } else { style::row_off }).into());
        }
        let header = row![
            column![text("Queue").size(16).font(style::strong()), text(format!("{} songs", self.queue.len())).size(12).color(style::muted())].spacing(2).width(Fill),
            press(button(text("Clear").size(12)).on_press(Message::ClearUpcoming).padding([4, 8]).style(style::ghost)),
            press(button(text("Close").size(12)).on_press(Message::Queue(false)).padding([4, 8]).style(style::ghost)),
        ]
        .align_y(Vertical::Center)
        .padding(pad(12.0, 12.0, 4.0, 12.0));
        container(column![header, scrollable(column(rows).spacing(2).padding(pad(0.0, 8.0, 12.0, 8.0))).height(Fill)].spacing(4))
            .width(320)
            .height(Fill)
            .style(style::fill(style::bar(), 0.0))
            .into()
    }
}

fn slide(on: bool, millis: u64) -> Animation<bool> {
    Animation::new(on).easing(if millis < 300 { Easing::EaseOutCubic } else { Easing::EaseInOutCubic }).duration(std::time::Duration::from_millis(millis))
}

fn pad(top: f32, right: f32, bottom: f32, left: f32) -> iced::Padding {
    iced::Padding { top, right, bottom, left }
}

fn chip_label(label: &str) -> Element<'static, Message> {
    let long = label.chars().count() > 28;
    let label = text(label.to_string()).size(14);
    if long {
        label.width(220).wrapping(iced::widget::text::Wrapping::WordOrGlyph).into()
    } else {
        label.into()
    }
}

fn mark() -> Element<'static, Message> {
    container(text("V").size(16).font(style::strong()).color(style::on_accent()))
        .width(28)
        .height(28)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(style::fill(style::highlight(), 8.0))
        .into()
}

fn hairline() -> Element<'static, Message> {
    container(iced::widget::Space::new().height(1).width(Fill)).width(Fill).height(1).style(style::line).into()
}

fn hairline_vertical() -> Element<'static, Message> {
    container(iced::widget::Space::new().width(1).height(Fill)).width(1).height(Fill).style(style::line).into()
}

fn saved_covers(library: &Library) -> (HashMap<String, Handle>, HashMap<String, Vec<u8>>) {
    let mut covers = HashMap::new();
    let mut pictures = HashMap::new();
    for track in &library.downloads {
        let Some(bytes) = remote::read_cover(track) else { continue };
        let handle = Handle::from_bytes(bytes.clone());
        covers.insert(remote::cover_key(track), handle.clone());
        pictures.insert(remote::cover_key(track), bytes.clone());
        if let Some(url) = remote::absolute_media(&track.thumbnail) {
            covers.insert(url.clone(), handle);
            pictures.insert(url, bytes);
        }
    }
    (covers, pictures)
}

async fn saved_or_fetched(track: &Track, cached: Option<Vec<u8>>) -> Option<Vec<u8>> {
    if let Some(bytes) = remote::read_cover(track) {
        return Some(bytes);
    }
    let bytes = if let Some(bytes) = cached {
        bytes
    } else {
        let url = remote::absolute_media(&track.thumbnail)?;
        remote::image(&url).await.ok()?
    };
    let _ = remote::save_cover(track, &bytes);
    Some(bytes)
}

async fn with_level(result: Result<Vec<u8>, String>) -> Result<(Vec<u8>, f32), String> {
    let bytes = std::sync::Arc::new(result?);
    let measured = std::sync::Arc::clone(&bytes);
    let gain = match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.spawn_blocking(move || output::level(&measured)).await.unwrap_or(1.0),
        Err(_) => output::level(&bytes),
    };
    let bytes = std::sync::Arc::try_unwrap(bytes).unwrap_or_else(|shared| (*shared).clone());
    Ok((bytes, gain))
}

fn track_key(track: &Track) -> String {
    format!("{}:{}", track.source().as_str(), track.id)
}

fn shuffle(tracks: &mut [Track]) {
    let mut state = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_nanos() as u64).unwrap_or(1);
    for index in (1..tracks.len()).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let swap = (state as usize) % (index + 1);
        tracks.swap(index, swap);
    }
}

static POINTER_X: AtomicU32 = AtomicU32::new(0);
static POINTER_Y: AtomicU32 = AtomicU32::new(0);
static MENU_OPEN: AtomicBool = AtomicBool::new(false);

fn pointer() -> Point {
    Point::new(f32::from_bits(POINTER_X.load(Ordering::Acquire)), f32::from_bits(POINTER_Y.load(Ordering::Acquire)))
}

fn track_pointer(event: iced::Event, _status: iced::event::Status, _window: iced::window::Id) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            POINTER_X.store(position.x.to_bits(), Ordering::Release);
            POINTER_Y.store(position.y.to_bits(), Ordering::Release);
            None
        }
        iced::Event::Mouse(iced::mouse::Event::WheelScrolled { .. }) if MENU_OPEN.load(Ordering::Acquire) => Some(Message::Dismiss),
        _ => None,
    }
}

fn place_menu(anchor: Point, bounds: Rectangle, viewport: Rectangle) -> Vector {
    let mut x = anchor.x + 22.0 - bounds.width;
    let mut y = anchor.y + 16.0;
    let left = viewport.x + 8.0;
    let top = viewport.y + 8.0;
    let right = viewport.x + viewport.width - 8.0;
    let bottom = viewport.y + viewport.height - 8.0;
    if y + bounds.height > bottom {
        let above = anchor.y - bounds.height - 12.0;
        if above >= top {
            y = above;
        }
    }
    if x + bounds.width > right {
        x = right - bounds.width;
    }
    if x < left {
        x = left;
    }
    if y < top {
        y = top;
    }
    if y + bounds.height > bottom {
        y = (bottom - bounds.height).max(top);
    }
    Vector::new(x - bounds.x, y - bounds.y)
}

fn on_key(event: iced::Event, status: iced::event::Status, _window: iced::window::Id) -> Option<Message> {
    if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape), repeat: false, .. }) = &event {
        return Some(Message::Dismiss);
    }
    if status == iced::event::Status::Captured {
        return None;
    }
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Space), repeat: false, .. }) => Some(Message::Toggle),
        _ => None,
    }
}
