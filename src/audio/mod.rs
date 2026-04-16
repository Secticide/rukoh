pub mod music;
pub mod sound;

pub use music::Music;
pub use sound::Sound;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use windows::core::{ScopedInterface, HRESULT};
use windows::Win32::Media::Audio::{
    XAudio2::{
        IXAudio2, IXAudio2EngineCallback, IXAudio2EngineCallback_Impl, IXAudio2MasteringVoice,
        IXAudio2SourceVoice, XAudio2CreateWithVersionInfo, XAUDIO2_BUFFER,
        XAUDIO2_DEFAULT_PROCESSOR, XAUDIO2_VOICE_STATE,
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
/// XAUDIO2_E_DEVICE_INVALIDATED — default audio endpoint was removed or changed.
const XAUDIO2_E_DEVICE_INVALIDATED: u32 = 0x8896_0004;

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

// ── Device callback ──────────────────────────────────────────────────────────

/// XAudio2 engine callback — fires when the audio device is invalidated.
///
/// Called from XAudio2's processing thread. Only sets an atomic flag; the
/// game thread checks this flag in [`AudioDevice::tick`] and reinitialises.
struct DeviceCallback {
    needs_reinit: Arc<AtomicBool>,
}

impl IXAudio2EngineCallback_Impl for DeviceCallback {
    fn OnProcessingPassStart(&self) {}
    fn OnProcessingPassEnd(&self) {}
    fn OnCriticalError(&self, error: HRESULT) {
        if error.0 as u32 == XAUDIO2_E_DEVICE_INVALIDATED {
            self.needs_reinit.store(true, Ordering::Relaxed);
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

/// Music state that survives engine reinit so the track can be restarted.
struct PendingMusic {
    data: Arc<[u8]>,
    channels: u16,
    sample_rate: u32,
    /// Last volume set by `set_music_volume`; restored after reinit.
    volume: f32,
    /// Whether the track is currently playing (false = paused).
    playing: bool,
}

/// Live XAudio2 state: engine, mastering voice, voice pool, and music voice.
///
/// `_callback` is declared last so it drops after all other fields (including
/// `xaudio2`). [`AudioInner::drop`] calls `UnregisterForCallbacks` first so no
/// callback fires against the `ScopedHeap` after it is freed.
struct AudioInner {
    xaudio2: IXAudio2,
    _master: IXAudio2MasteringVoice,
    // ALLOCATION: voice pool backing store — starts empty, grows by one PoolSlot each
    // time a new audio format is first played, capped at max_voices.
    pool: Vec<PoolSlot>,
    music: Option<MusicSlot>,
    /// The `ScopedInterface` wraps a heap-allocated `ScopedHeap` that XAudio2 holds a
    /// raw pointer to. Must be kept alive until after `UnregisterForCallbacks`.
    ///
    /// # Safety invariant
    /// `AudioDevice` declares `inner` before `callback_impl`, so `inner` (and
    /// therefore this `ScopedInterface`) always drops before `callback_impl`
    /// (`Box<DeviceCallback>`). The raw pointer stored in the `ScopedHeap` remains
    /// valid for the entire lifetime of this `ScopedInterface`.
    _callback: ScopedInterface<'static, IXAudio2EngineCallback>,
}

impl AudioInner {
    fn new(
        callback_scope: ScopedInterface<'static, IXAudio2EngineCallback>,
    ) -> Result<Self, Error> {
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

            xaudio2
                .RegisterForCallbacks(Some(&*callback_scope))
                .map_err(|e| Error::Audio(e.to_string()))?;

            Ok(Self {
                xaudio2,
                _master: master,
                pool: Vec::new(),
                music: None,
                _callback: callback_scope,
            })
        }
    }
}

impl Drop for AudioInner {
    fn drop(&mut self) {
        // Unregister before the engine is released. UnregisterForCallbacks
        // blocks until any in-progress callback completes, so no callback will
        // fire against the ScopedHeap inside _callback after this returns.
        unsafe {
            self.xaudio2.UnregisterForCallbacks(Some(&*self._callback));
        }
        // Fields drop after this in declaration order: xaudio2, _master,
        // pool, music, _callback. The _callback ScopedHeap is safe to free
        // because we just unregistered.
    }
}

/// Owns the XAudio2 engine, mastering voice, sound pool, and music voice.
///
/// Held by [`Rukoh`](crate::Rukoh). All methods are called from the game thread.
///
/// Field declaration order is load-bearing: `inner` drops before `callback_impl`,
/// ensuring the `ScopedInterface` inside `inner._callback` is freed before the
/// `DeviceCallback` it points to.
pub(crate) struct AudioDevice {
    /// `None` when audio is unavailable (no device at init, or reinit pending).
    /// Declared first so it drops before `callback_impl`.
    inner: Option<AudioInner>,
    needs_reinit: Arc<AtomicBool>,
    /// Declared after `inner` so it outlives the `ScopedInterface` in `inner._callback`.
    callback_impl: Box<DeviceCallback>,
    /// Music state kept across engine reinits so the track can be restarted.
    pending_music: Option<PendingMusic>,
    max_voices: usize,
}

impl AudioDevice {
    pub(crate) fn new(max_voices: u32) -> Result<Self, Error> {
        let needs_reinit = Arc::new(AtomicBool::new(false));
        // ALLOCATION: Box<DeviceCallback> — one per AudioDevice lifetime.
        let callback_impl = Box::new(DeviceCallback {
            needs_reinit: Arc::clone(&needs_reinit),
        });

        let inner = {
            // SAFETY: callback_impl is in a Box (stable address) and is declared
            // after inner in AudioDevice, so it outlives the ScopedInterface.
            let scope = unsafe { make_callback_scope(&callback_impl) };
            match AudioInner::new(scope) {
                Ok(i) => Some(i),
                Err(e) => {
                    // No audio device available. The app continues without sound.
                    eprintln!("rukoh: audio unavailable — {e}");
                    None
                }
            }
        };

        Ok(Self {
            inner,
            needs_reinit,
            callback_impl,
            pending_music: None,
            max_voices: max_voices as usize,
        })
    }

    /// Called once per frame. Reinitialises the audio engine if the default
    /// audio device was invalidated (e.g. the user switched output devices).
    pub(crate) fn tick(&mut self) {
        if !self.needs_reinit.swap(false, Ordering::Relaxed) {
            return;
        }
        self.try_reinit();
    }

    fn try_reinit(&mut self) {
        // Drop the old engine. AudioInner::drop calls UnregisterForCallbacks,
        // ensuring no callback fires against the old ScopedHeap after this.
        self.inner = None;

        // SAFETY: same invariant as in new() — callback_impl is Boxed and
        // outlives the new ScopedInterface we're creating here.
        let scope = unsafe { make_callback_scope(&self.callback_impl) };
        match AudioInner::new(scope) {
            Ok(mut inner) => {
                // Restart music if a track was playing when the device was lost.
                if let Some(pm) = &self.pending_music {
                    if pm.playing {
                        start_music_voice(&mut inner, pm);
                    }
                }
                self.inner = Some(inner);
            }
            Err(e) => {
                // Still no device. Stay silent; will retry on the next
                // OnCriticalError notification.
                eprintln!("rukoh: audio reinit failed — {e}");
            }
        }
    }

    // ── Sound effects ────────────────────────────────────────────────────────

    /// Play a sound effect. Finds an idle pool slot with the same format and
    /// reuses it; or creates a new slot if the pool is not full. Silently
    /// drops the request if all slots are busy or audio is unavailable.
    pub(crate) fn play_sound(&mut self, sound: &Sound, params: SoundParams) {
        let Some(inner) = &mut self.inner else { return };

        // Try to reuse an idle slot with a matching audio format.
        for slot in &mut inner.pool {
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
        if inner.pool.len() >= self.max_voices {
            return;
        }

        let fmt = waveformat(sound.channels, sound.sample_rate);
        let mut voice_out: Option<IXAudio2SourceVoice> = None;
        unsafe {
            if inner
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

        // ALLOCATION: Vec push — happens at most max_voices times total.
        inner.pool.push(PoolSlot {
            voice,
            data,
            channels: sound.channels,
            sample_rate: sound.sample_rate,
        });
    }

    // ── Music ────────────────────────────────────────────────────────────────

    /// Start playing `music`, looping indefinitely. Stops any currently-playing track.
    pub(crate) fn play_music(&mut self, music: &crate::audio::music::Music) {
        self.pending_music = Some(PendingMusic {
            data: Arc::clone(&music.data),
            channels: music.channels,
            sample_rate: music.sample_rate,
            volume: 1.0,
            playing: true,
        });
        let Some(inner) = &mut self.inner else { return };
        stop_music_voice(inner);
        start_music_voice(inner, self.pending_music.as_ref().unwrap());
    }

    /// Pause music playback (preserves position; call `resume_music` to continue).
    pub(crate) fn pause_music(&mut self) {
        if let Some(pm) = &mut self.pending_music {
            pm.playing = false;
        }
        let Some(inner) = &mut self.inner else { return };
        if let Some(slot) = &inner.music {
            unsafe {
                let _ = slot.voice.Stop(0, COMMIT_NOW);
            }
        }
    }

    /// Resume paused music playback.
    pub(crate) fn resume_music(&mut self) {
        if let Some(pm) = &mut self.pending_music {
            pm.playing = true;
        }
        let Some(inner) = &mut self.inner else { return };
        if let Some(slot) = &inner.music {
            unsafe {
                let _ = slot.voice.Start(0, COMMIT_NOW);
            }
        }
    }

    /// Stop music and reset position to the beginning.
    pub(crate) fn stop_music(&mut self) {
        self.pending_music = None;
        let Some(inner) = &mut self.inner else { return };
        stop_music_voice(inner);
    }

    /// Set the music volume (0.0 = silent, 1.0 = full).
    pub(crate) fn set_music_volume(&mut self, volume: f32) {
        if let Some(pm) = &mut self.pending_music {
            pm.volume = volume;
        }
        let Some(inner) = &mut self.inner else { return };
        if let Some(slot) = &inner.music {
            unsafe {
                let _ = slot.voice.SetVolume(volume, COMMIT_NOW);
            }
        }
    }
}

// ── Music voice helpers ──────────────────────────────────────────────────────

/// Stop and drop the active music voice, if any.
fn stop_music_voice(inner: &mut AudioInner) {
    if let Some(slot) = inner.music.take() {
        unsafe {
            let _ = slot.voice.Stop(0, COMMIT_NOW);
            let _ = slot.voice.FlushSourceBuffers();
        }
        // slot dropped here — voice released
    }
}

/// Create and start a looping music voice from `pm`. Silently returns on failure.
fn start_music_voice(inner: &mut AudioInner, pm: &PendingMusic) {
    let fmt = waveformat(pm.channels, pm.sample_rate);
    let mut voice_out: Option<IXAudio2SourceVoice> = None;
    unsafe {
        if inner
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

    let buf = music_buffer(&pm.data);
    unsafe {
        if voice.SubmitSourceBuffer(&buf, None).is_err() {
            return;
        }
        let _ = voice.SetVolume(pm.volume, COMMIT_NOW);
        let _ = voice.Start(0, COMMIT_NOW);
    }

    inner.music = Some(MusicSlot {
        voice,
        _data: Arc::clone(&pm.data),
    });
}

// ── Callback helper ──────────────────────────────────────────────────────────

/// Wrap `callback_impl` in a `ScopedInterface`, transmuted to `'static`.
///
/// # Safety
/// The caller must ensure `callback_impl` outlives the returned `ScopedInterface`.
/// `AudioDevice` upholds this: `inner` is declared before `callback_impl` in its
/// fields, so `inner` (which owns the `ScopedInterface`) always drops first.
unsafe fn make_callback_scope(
    callback_impl: &DeviceCallback,
) -> ScopedInterface<'static, IXAudio2EngineCallback> {
    let scoped = IXAudio2EngineCallback::new(callback_impl);
    // SAFETY: guaranteed by the caller's lifetime invariant.
    std::mem::transmute(scoped)
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
