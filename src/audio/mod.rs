pub mod music;
pub mod sound;

pub use music::Music;
pub use sound::Sound;

use std::sync::Arc;

use windows::Win32::Media::Audio::{
    XAudio2::{
        IXAudio2, IXAudio2MasteringVoice, IXAudio2SourceVoice, XAudio2CreateWithVersionInfo,
        XAUDIO2_BUFFER, XAUDIO2_DEFAULT_PROCESSOR, XAUDIO2_VOICE_STATE,
    },
    WAVEFORMATEX, WAVE_FORMAT_PCM,
};

use crate::Error;

// ── Constants not directly exposed by windows-rs ────────────────────────────

/// XAudio2_COMMIT_NOW — apply operation immediately.
const COMMIT_NOW: u32 = 0;
/// XAUDIO2_END_OF_STREAM — marks the final buffer in a sequence.
const END_OF_STREAM: u32 = 0x0040;
/// XAUDIO2_LOOP_INFINITE — loop this buffer indefinitely.
const LOOP_INFINITE: u32 = 255;

// ── Public API types ─────────────────────────────────────────────────────────

/// Parameters for a single sound-effect playback request.
///
/// Pass to [`Frame::play_sound`](crate::Frame::play_sound).
/// All fields have `1.0` defaults.
#[derive(Clone, Copy, Debug)]
pub struct SoundParams {
    /// Playback volume multiplier (1.0 = original volume, 0.0 = silent).
    pub volume: f32,
    /// Playback pitch multiplier (1.0 = original pitch, 2.0 = one octave up).
    pub pitch: f32,
}

impl Default for SoundParams {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
        }
    }
}

// ── Internal audio device ────────────────────────────────────────────────────

/// A slot in the shared sound-effect voice pool.
struct PoolSlot {
    voice: IXAudio2SourceVoice,
    /// Keep the PCM data alive for as long as XAudio2 might be reading it.
    data: Arc<[u8]>,
    channels: u16,
    sample_rate: u32,
}

/// State for the currently-active music track.
struct MusicSlot {
    voice: IXAudio2SourceVoice,
    /// Keep the PCM data alive.
    _data: Arc<[u8]>,
}

/// Owns the XAudio2 engine, mastering voice, sound pool, and music voice.
///
/// Held by [`Rukoh`](crate::Rukoh). All methods are called from the game thread.
pub(crate) struct AudioDevice {
    xaudio2: IXAudio2,
    _master: IXAudio2MasteringVoice,
    pool: Vec<PoolSlot>,
    max_voices: usize,
    music: Option<MusicSlot>,
}

impl AudioDevice {
    pub(crate) fn new(max_voices: u32) -> Result<Self, Error> {
        unsafe {
            let mut xaudio2: Option<IXAudio2> = None;
            // ALLOCATION: error path — windows-rs error converted to String.
            XAudio2CreateWithVersionInfo(&mut xaudio2, 0, XAUDIO2_DEFAULT_PROCESSOR, 0)
                .map_err(|e| Error::Audio(e.to_string()))?;
            // ALLOCATION: error path — &'static str promoted to String.
            let xaudio2 =
                xaudio2.ok_or_else(|| Error::Audio("XAudio2Create returned null".into()))?;

            let mut master: Option<IXAudio2MasteringVoice> = None;
            xaudio2
                .CreateMasteringVoice(
                    &mut master,
                    0, // XAUDIO2_DEFAULT_CHANNELS
                    0, // XAUDIO2_DEFAULT_SAMPLERATE
                    0, // Flags
                    None,
                    None,
                    Default::default(), // AudioCategory_Other
                )
                // ALLOCATION: error path — windows-rs error converted to String.
                .map_err(|e| Error::Audio(e.to_string()))?;
            // ALLOCATION: error path — &'static str promoted to String.
            let master =
                master.ok_or_else(|| Error::Audio("CreateMasteringVoice returned null".into()))?;

            Ok(Self {
                xaudio2,
                _master: master,
                // ALLOCATION: voice pool backing store — starts empty, grows by one PoolSlot each
                // time a new audio format is first played, capped at max_voices. Not pre-allocated;
                // Vec::with_capacity(max_voices) in new() would prevent all subsequent reallocations.
                pool: Vec::new(),
                max_voices: max_voices as usize,
                music: None,
            })
        }
    }

    // ── Sound effects ────────────────────────────────────────────────────────

    /// Play a sound effect. Finds an idle pool slot with the same format and
    /// reuses it; or creates a new slot if the pool is not full. Silently
    /// drops the request if all slots are busy.
    pub(crate) fn play_sound(&mut self, sound: &Sound, params: SoundParams) {
        // Try to reuse an idle slot with a matching audio format.
        for slot in &mut self.pool {
            if slot.channels != sound.channels || slot.sample_rate != sound.sample_rate {
                continue;
            }
            let mut state = XAUDIO2_VOICE_STATE::default();
            unsafe { slot.voice.GetState(&mut state, 0) };
            if state.BuffersQueued == 0 {
                // Reuse: replace the data Arc and re-submit.
                slot.data = Arc::clone(&sound.data);
                let buf = sound_buffer(&slot.data);
                unsafe {
                    let _ = slot.voice.FlushSourceBuffers();
                    if slot.voice.SubmitSourceBuffer(&buf, None).is_err() {
                        return;
                    }
                    let _ = slot.voice.SetVolume(params.volume, COMMIT_NOW);
                    let _ = slot.voice.SetFrequencyRatio(params.pitch, COMMIT_NOW);
                    let _ = slot.voice.Start(0, COMMIT_NOW);
                }
                return;
            }
        }

        // No reusable slot; create a new one if the pool has room.
        if self.pool.len() >= self.max_voices {
            return;
        }

        let fmt = waveformat(sound.channels, sound.sample_rate);
        let mut voice_out: Option<IXAudio2SourceVoice> = None;
        unsafe {
            if self
                .xaudio2
                .CreateSourceVoice(
                    &mut voice_out,
                    &fmt,
                    0,
                    2.0_f32, // MaxFrequencyRatio
                    None,
                    None,
                    None,
                )
                .is_err()
            {
                return;
            }
        }
        let voice = match voice_out {
            Some(v) => v,
            None => return,
        };

        let data = Arc::clone(&sound.data);
        let buf = sound_buffer(&data);
        unsafe {
            if voice.SubmitSourceBuffer(&buf, None).is_err() {
                return;
            }
            let _ = voice.SetVolume(params.volume, COMMIT_NOW);
            let _ = voice.SetFrequencyRatio(params.pitch, COMMIT_NOW);
            let _ = voice.Start(0, COMMIT_NOW);
        }

        // ALLOCATION: Vec push — may reallocate the pool Vec; happens at most max_voices times
        // total (one per unique format group encountered). Eliminated after the pool is full.
        self.pool.push(PoolSlot {
            voice,
            data,
            channels: sound.channels,
            sample_rate: sound.sample_rate,
        });
    }

    // ── Music ────────────────────────────────────────────────────────────────

    /// Start playing `music`, looping indefinitely. Stops any currently-playing track.
    pub(crate) fn play_music(&mut self, music: &crate::audio::music::Music) {
        // Stop and drop the current music voice (if any) before starting a new one.
        // Dropping an IXAudio2SourceVoice is handled by windows-rs.
        if let Some(slot) = self.music.take() {
            unsafe {
                let _ = slot.voice.Stop(0, COMMIT_NOW);
                let _ = slot.voice.FlushSourceBuffers();
            }
            // slot dropped here — voice released
        }

        let fmt = waveformat(music.channels, music.sample_rate);
        let mut voice_out: Option<IXAudio2SourceVoice> = None;
        unsafe {
            if self
                .xaudio2
                .CreateSourceVoice(
                    &mut voice_out,
                    &fmt,
                    0,
                    1.0_f32, // pitch fixed for music
                    None,
                    None,
                    None,
                )
                .is_err()
            {
                return;
            }
        }
        let voice = match voice_out {
            Some(v) => v,
            None => return,
        };

        let data = Arc::clone(&music.data);
        let buf = music_buffer(&data);
        unsafe {
            if voice.SubmitSourceBuffer(&buf, None).is_err() {
                return;
            }
            let _ = voice.Start(0, COMMIT_NOW);
        }

        self.music = Some(MusicSlot { voice, _data: data });
    }

    /// Pause music playback (preserves position; call `resume_music` to continue).
    pub(crate) fn pause_music(&mut self) {
        if let Some(slot) = &self.music {
            unsafe {
                let _ = slot.voice.Stop(0, COMMIT_NOW);
            }
        }
    }

    /// Resume paused music playback.
    pub(crate) fn resume_music(&mut self) {
        if let Some(slot) = &self.music {
            unsafe {
                let _ = slot.voice.Start(0, COMMIT_NOW);
            }
        }
    }

    /// Stop music and reset position to the beginning.
    pub(crate) fn stop_music(&mut self) {
        if let Some(slot) = &self.music {
            unsafe {
                let _ = slot.voice.Stop(0, COMMIT_NOW);
                let _ = slot.voice.FlushSourceBuffers();
            }
        }
    }

    /// Set the music volume (0.0 = silent, 1.0 = full).
    pub(crate) fn set_music_volume(&mut self, volume: f32) {
        if let Some(slot) = &self.music {
            unsafe {
                let _ = slot.voice.SetVolume(volume, COMMIT_NOW);
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Decode OGG Vorbis bytes to interleaved little-endian i16 PCM bytes.
pub(crate) fn decode_ogg(bytes: &[u8]) -> Result<(Vec<u8>, u16, u32), Error> {
    use lewton::inside_ogg::OggStreamReader;

    let cursor = std::io::Cursor::new(bytes);
    // ALLOCATION: error path — lewton error converted to String.
    let mut reader = OggStreamReader::new(cursor).map_err(|e| Error::Audio(e.to_string()))?;

    let channels = reader.ident_hdr.audio_channels as u16;
    let sample_rate = reader.ident_hdr.audio_sample_rate;

    // ALLOCATION: intermediate i16 PCM staging buffer — full decoded audio in memory at load
    // time; immediately converted to bytes and dropped. Avoidable: write bytes directly by
    // iterating over lewton packets and pushing s.to_le_bytes() into a Vec<u8>, eliminating
    // this intermediate Vec<i16> entirely.
    let mut samples: Vec<i16> = Vec::new();
    loop {
        match reader.read_dec_packet_itl() {
            Ok(Some(pck)) => samples.extend_from_slice(&pck),
            Ok(None) => break,
            // ALLOCATION: error path — lewton error converted to String.
            Err(e) => return Err(Error::Audio(e.to_string())),
        }
    }

    // ALLOCATION: final PCM byte buffer — this is the keeper; stored in Arc<Vec<u8>> inside Sound/Music.
    // Its size = num_samples * 2 bytes. The intermediate Vec<i16> above is a redundant copy
    // of this same data; eliminating it halves peak memory during decode.
    let pcm_bytes: Vec<u8> = samples.iter().flat_map(|&s| s.to_le_bytes()).collect();

    Ok((pcm_bytes, channels, sample_rate))
}

fn waveformat(channels: u16, sample_rate: u32) -> WAVEFORMATEX {
    WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: channels,
        nSamplesPerSec: sample_rate,
        nAvgBytesPerSec: sample_rate * channels as u32 * 2,
        nBlockAlign: channels * 2,
        wBitsPerSample: 16,
        cbSize: 0,
    }
}

fn sound_buffer(data: &[u8]) -> XAUDIO2_BUFFER {
    XAUDIO2_BUFFER {
        Flags: END_OF_STREAM,
        AudioBytes: data.len() as u32,
        pAudioData: data.as_ptr(),
        PlayBegin: 0,
        PlayLength: 0,
        LoopBegin: 0,
        LoopLength: 0,
        LoopCount: 0,
        pContext: std::ptr::null_mut(),
    }
}

fn music_buffer(data: &[u8]) -> XAUDIO2_BUFFER {
    XAUDIO2_BUFFER {
        Flags: END_OF_STREAM,
        AudioBytes: data.len() as u32,
        pAudioData: data.as_ptr(),
        PlayBegin: 0,
        PlayLength: 0,
        LoopBegin: 0,
        LoopLength: 0,
        LoopCount: LOOP_INFINITE,
        pContext: std::ptr::null_mut(),
    }
}
