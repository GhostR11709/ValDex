pub struct Output {
    sink: rodio::MixerDeviceSink,
    player: rodio::Player,
    bytes: Vec<u8>,
}

impl Output {
    pub fn start(bytes: Vec<u8>) -> Result<Self, String> {
        let sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
        let player = play_bytes(&sink, &bytes)?;
        Ok(Self { sink, player, bytes })
    }

    pub fn finished(&self) -> bool {
        self.player.empty()
    }

    pub fn replay(&mut self) -> Result<(), String> {
        self.player = play_bytes(&self.sink, &self.bytes)?;
        Ok(())
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn position(&self) -> f32 {
        self.player.get_pos().as_secs_f32()
    }

    pub fn seek(&self, seconds: f32) {
        let _ = self.player.try_seek(std::time::Duration::from_secs_f32(seconds.max(0.0)));
    }

    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
    }
}

pub fn song_ended(finished: bool, position: f32, duration: f32) -> bool {
    finished || (duration > 1.0 && position >= duration)
}

fn play_bytes(sink: &rodio::MixerDeviceSink, bytes: &[u8]) -> Result<rodio::Player, String> {
    rodio::play(sink.mixer(), std::io::Cursor::new(bytes.to_vec())).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::song_ended;

    #[test]
    fn repeat_notices_when_the_audio_ends_before_the_catalog_time() {
        assert!(song_ended(true, 12.0, 180.0));
        assert!(!song_ended(false, 12.0, 180.0));
        assert!(song_ended(false, 180.0, 180.0));
        assert!(!song_ended(false, 0.0, 0.0));
    }
}
