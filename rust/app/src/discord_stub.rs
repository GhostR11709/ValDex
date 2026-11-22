pub struct Presence;

impl Presence {
    pub fn start() -> Self {
        Self
    }

    pub fn show(&self, _title: &str, _artist: &str, _image: &str, _playing: bool, _position: f32, _duration: f32) {}

    pub fn clear(&self) {}
}

pub fn valid_id(value: &str) -> bool {
    let count = value.len();
    (17..=20).contains(&count) && value.bytes().all(|byte| byte.is_ascii_digit())
}
