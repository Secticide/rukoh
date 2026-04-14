use std::sync::Arc;

use crate::Error;

/// A fully-decoded audio track for background music.
///
/// Load with [`Music::load`]. Control playback via [`Frame::play_music`](crate::Frame::play_music),
/// [`Frame::pause_music`](crate::Frame::pause_music), [`Frame::resume_music`](crate::Frame::resume_music),
/// [`Frame::stop_music`](crate::Frame::stop_music), and [`Frame::set_music_volume`](crate::Frame::set_music_volume).
///
/// Music loops indefinitely by default; call `stop_music()` to end it.
pub struct Music {
    /// Raw PCM bytes (little-endian i16 samples, interleaved channels).
    pub(crate) data: Arc<[u8]>,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
}

impl Music {
    /// Load and decode an OGG Vorbis file from `bytes`.
    pub fn load(_rukoh: &crate::Rukoh, bytes: &[u8]) -> Result<Self, Error> {
        let (data, channels, sample_rate) = super::decode_ogg(bytes)?;
        Ok(Self {
            // ALLOCATION: single fat-pointer allocation — Arc<[u8]> fuses the control block
            // and the byte slice into one heap allocation (no separate Vec header).
            data: Arc::from(data.into_boxed_slice()),
            channels,
            sample_rate,
        })
    }
}
