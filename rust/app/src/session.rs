use std::sync::mpsc::{self, Receiver};

use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::Playback::MediaPlayer;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs, SystemMediaTransportControls, SystemMediaTransportControlsButton,
    SystemMediaTransportControlsButtonPressedEventArgs, SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference};
use windows::core::HSTRING;

use crate::model::Track;

pub enum Command {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(f32),
}

pub struct Session {
    _player: MediaPlayer,
    controls: SystemMediaTransportControls,
    rx: Receiver<Command>,
    shown: String,
    art_ready: bool,
}

impl Session {
    pub fn open() -> Result<Self, String> {
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED) };
        let player = MediaPlayer::new().map_err(|error| error.to_string())?;
        if let Ok(manager) = player.CommandManager() {
            let _ = manager.SetIsEnabled(false);
        }
        let controls = player.SystemMediaTransportControls().map_err(|error| error.to_string())?;
        controls.SetIsEnabled(true).map_err(|error| error.to_string())?;
        let _ = controls.SetIsPlayEnabled(true);
        let _ = controls.SetIsPauseEnabled(true);
        let _ = controls.SetIsStopEnabled(true);
        let _ = controls.SetIsNextEnabled(true);
        let _ = controls.SetIsPreviousEnabled(true);
        let (tx, rx) = mpsc::channel();
        let buttons = tx.clone();
        let button_handler = TypedEventHandler::new(move |_sender, args: windows::core::Ref<SystemMediaTransportControlsButtonPressedEventArgs>| {
            let Some(args) = args.as_ref() else { return Ok(()) };
            if let Some(command) = command_for(args.Button()?) {
                let _ = buttons.send(command);
            }
            Ok(())
        });
        controls.ButtonPressed(&button_handler).map_err(|error| error.to_string())?;
        let seek_handler = TypedEventHandler::new(move |_sender, args: windows::core::Ref<PlaybackPositionChangeRequestedEventArgs>| {
            let Some(args) = args.as_ref() else { return Ok(()) };
            let span = args.RequestedPlaybackPosition()?;
            let _ = tx.send(Command::Seek(span.Duration as f32 / 10_000_000.0));
            Ok(())
        });
        controls.PlaybackPositionChangeRequested(&seek_handler).map_err(|error| error.to_string())?;
        Ok(Self { _player: player, controls, rx, shown: String::new(), art_ready: false })
    }

    pub fn poll(&self) -> Vec<Command> {
        let mut commands = Vec::new();
        while let Ok(command) = self.rx.try_recv() {
            commands.push(command);
        }
        commands
    }

    pub fn needs_art(&self, key: &str) -> bool {
        self.shown != key || !self.art_ready
    }

    pub fn clear(&mut self) {
        self.shown.clear();
        self.art_ready = false;
        let _ = self.controls.SetPlaybackStatus(MediaPlaybackStatus::Stopped);
    }

    pub fn show(&mut self, track: &Track, playing: bool, position: f32, art: Option<&[u8]>) {
        let key = crate::remote::cover_key(track);
        let status = if playing { MediaPlaybackStatus::Playing } else { MediaPlaybackStatus::Paused };
        let _ = self.controls.SetIsEnabled(true);
        let _ = self.controls.SetPlaybackStatus(status);
        let changed = self.shown != key;
        if changed {
            self.shown = key;
            self.art_ready = false;
        }
        if changed || !self.art_ready {
            if let Ok(updater) = self.controls.DisplayUpdater() {
                if changed {
                    let _ = updater.SetType(MediaPlaybackType::Music);
                    let _ = updater.SetAppMediaId(&HSTRING::from("ValDex"));
                    if let Ok(music) = updater.MusicProperties() {
                        let _ = music.SetTitle(&HSTRING::from(track.title.as_str()));
                        let _ = music.SetArtist(&HSTRING::from(track.artist()));
                        let album = track.album.as_ref().map(|album| album.name.as_str()).unwrap_or("");
                        let _ = music.SetAlbumTitle(&HSTRING::from(album));
                    }
                }
                if !self.art_ready {
                    if let Some(bytes) = art {
                        if picture(bytes).and_then(|reference| updater.SetThumbnail(&reference)).is_ok() {
                            self.art_ready = true;
                        }
                    }
                }
                let _ = updater.Update();
            }
        }
        let duration = track.duration.unwrap_or(0) as f32;
        let end = duration.max(position).max(1.0);
        if let Ok(timeline) = SystemMediaTransportControlsTimelineProperties::new() {
            let _ = timeline.SetStartTime(ticks(0.0));
            let _ = timeline.SetMinSeekTime(ticks(0.0));
            let _ = timeline.SetEndTime(ticks(end));
            let _ = timeline.SetMaxSeekTime(ticks(end));
            let _ = timeline.SetPosition(ticks(position.clamp(0.0, end)));
            let _ = self.controls.UpdateTimelineProperties(&timeline);
        }
    }
}

fn command_for(button: SystemMediaTransportControlsButton) -> Option<Command> {
    if button == SystemMediaTransportControlsButton::Play {
        Some(Command::Play)
    } else if button == SystemMediaTransportControlsButton::Pause {
        Some(Command::Pause)
    } else if button == SystemMediaTransportControlsButton::Stop {
        Some(Command::Stop)
    } else if button == SystemMediaTransportControlsButton::Next {
        Some(Command::Next)
    } else if button == SystemMediaTransportControlsButton::Previous {
        Some(Command::Previous)
    } else {
        None
    }
}

fn ticks(seconds: f32) -> TimeSpan {
    TimeSpan { Duration: (seconds.max(0.0) as f64 * 10_000_000.0) as i64 }
}

fn picture(bytes: &[u8]) -> windows::core::Result<RandomAccessStreamReference> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream.GetOutputStreamAt(0)?)?;
    writer.WriteBytes(bytes)?;
    writer.StoreAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    RandomAccessStreamReference::CreateFromStream(&stream)
}
