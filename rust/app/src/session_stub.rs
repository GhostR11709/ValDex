use crate::model::Track;

pub enum Command {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(f32),
}

pub struct Session;

impl Session {
    pub fn open() -> Result<Self, String> {
        Err("This system has no media session.".into())
    }

    pub fn poll(&self) -> Vec<Command> {
        Vec::new()
    }

    pub fn needs_art(&self, _key: &str) -> bool {
        false
    }

    pub fn clear(&mut self) {}

    pub fn show(&mut self, _track: &Track, _playing: bool, _position: f32, _art: Option<&[u8]>) {}
}
