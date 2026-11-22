pub struct Output {
    sink: rodio::MixerDeviceSink,
    player: rodio::Player,
    bytes: Vec<u8>,
    /// Multiplier that brings this song toward the same loudness as the others. The slider still scales it.
    gain: f32,
}

impl Output {
    pub fn start(bytes: Vec<u8>, gain: f32) -> Result<Self, String> {
        let gain = if gain.is_finite() && gain > 0.0 { gain } else { 1.0 };
        let sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?;
        let player = play_bytes(&sink, &bytes)?;
        Ok(Self { sink, player, bytes, gain })
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
        let user = if volume.is_finite() { volume.clamp(0.0, 1.0) } else { 0.0 };
        self.player.set_volume(user * self.gain);
    }
}

/// Gain that pulls a song toward the same loudness as the others, without pushing its peaks into clipping.
/// The first three minutes are enough to judge a song and keep the wait short.
pub fn level(bytes: &[u8]) -> f32 {
    use rodio::Source;
    let Ok(mut decoder) = rodio::Decoder::try_from(std::io::Cursor::new(bytes.to_vec())) else {
        return 1.0;
    };
    let rate = u64::from(decoder.sample_rate().get()).max(1);
    let channels = u64::from(decoder.channels().get()).max(1);
    let limit = rate.saturating_mul(channels).saturating_mul(180);
    let mut sum = 0.0f64;
    let mut count = 0u64;
    let mut peak = 0.0f32;
    for sample in decoder.by_ref() {
        let wide = f64::from(sample);
        sum += wide * wide;
        peak = peak.max(sample.abs());
        count += 1;
        if count >= limit {
            break;
        }
    }
    if count < rate.saturating_mul(channels) / 5 || peak < 1.0e-4 {
        return 1.0;
    }
    let rms = (sum / count as f64).sqrt();
    if !rms.is_finite() || rms < 1.0e-4 {
        return 1.0;
    }
    let mut gain = TARGET_RMS / rms as f32;
    let peak_gain = PEAK_CEILING / peak;
    if gain > peak_gain {
        gain = peak_gain;
    }
    if !gain.is_finite() { 1.0 } else { gain.clamp(0.12, 8.0) }
}

const TARGET_RMS: f32 = 0.20;
const PEAK_CEILING: f32 = 0.98;

pub fn song_ended(finished: bool, position: f32, duration: f32) -> bool {
    finished || (duration > 1.0 && position >= duration)
}

fn play_bytes(sink: &rodio::MixerDeviceSink, bytes: &[u8]) -> Result<rodio::Player, String> {
    let player = rodio::play(sink.mixer(), std::io::Cursor::new(bytes.to_vec())).map_err(|error| error.to_string())?;
    player.set_volume(0.0);
    Ok(player)
}

#[cfg(test)]
mod tests {
    use super::{level, song_ended};

    #[test]
    fn repeat_notices_when_the_audio_ends_before_the_catalog_time() {
        assert!(song_ended(true, 12.0, 180.0));
        assert!(!song_ended(false, 12.0, 180.0));
        assert!(song_ended(false, 180.0, 180.0));
        assert!(!song_ended(false, 0.0, 0.0));
    }

    #[test]
    fn a_quiet_song_is_lifted_above_a_loud_one() {
        let quiet = level(&sine_wav(0.05));
        let loud = level(&sine_wav(0.5));
        assert!(quiet > loud, "quiet {quiet} should sit above loud {loud}");
        assert!(loud < 1.0, "a loud song should come down, got {loud}");
        assert!(quiet > 1.0, "a quiet song should come up, got {quiet}");
    }

    #[test]
    fn a_spike_does_not_get_turned_up_into_clipping() {
        let mut samples = vec![0.0f32; 4_000];
        samples[100] = 1.0;
        let gain = level(&wav(8_000, &samples));
        assert!(gain <= 1.0, "peak was already full, gain was {gain}");
        assert!(gain * 1.0 <= 0.99);
    }

    fn sine_wav(amplitude: f32) -> Vec<u8> {
        let rate = 8_000u32;
        let samples: Vec<f32> = (0..4_000).map(|index| amplitude * (2.0 * std::f32::consts::PI * 440.0 * index as f32 / rate as f32).sin()).collect();
        wav(rate, &samples)
    }

    fn wav(rate: u32, samples: &[f32]) -> Vec<u8> {
        let mut pcm = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            pcm.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32_767.0) as i16).to_le_bytes());
        }
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        out.extend(pcm);
        out
    }
}
