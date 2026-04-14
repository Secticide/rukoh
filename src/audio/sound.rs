use std::sync::Arc;

use crate::Error;

/// A fully-decoded audio clip for short sound effects.
///
/// Load with [`Sound::load`]. Play via [`Frame::play_sound`](crate::Frame::play_sound).
/// The decoded PCM data lives in an `Arc` so submitting the same sound
/// concurrently is allocation-free.
pub struct Sound {
    /// Raw PCM bytes (little-endian i16 samples, interleaved channels).
    pub(crate) data: Arc<[u8]>,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
}

impl Sound {
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
