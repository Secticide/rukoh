pub mod audio;
mod colour;
mod error;
mod frame;
mod gfx;
pub mod graphics;
pub mod input;
mod maths;
mod window;

pub use audio::{Music, Sound, SoundParams};
pub use colour::Colour;
pub use error::Error;
pub use frame::Frame;
pub use graphics::{BlendMode, Camera2D, DrawParams, Font, RenderTarget, Texture2D};
pub use input::{GamepadBackend, GamepadButton, GamepadState, KeyCode, MouseButton};
pub use maths::{Rect, Vec2};

use windows::Win32::Graphics::Direct3D11::ID3D11RenderTargetView;

use audio::AudioDevice;
use gfx::GfxDevice;
use graphics::renderer::SpriteBatch;
use input::{gamepad::poll_gamepad, hid::HidManager, InputState};
use window::{client_size, Win32Window};

use windows::Win32::{
    Graphics::Direct3D11::ID3D11Device,
    System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE},
};

// ── Configuration ────────────────────────────────────────────────────────────

/// Window display mode.
pub enum WindowMode {
    /// Fixed-size window centred on the primary monitor.
    Windowed { width: u32, height: u32 },
    /// Borderless window covering the entire primary monitor.
    BorderlessWindowed,
}

/// Configuration passed to [`Rukoh::new`].
pub struct RukohConfig {
    /// Window title bar text.
    pub title: &'static str,
    /// Window display mode.
    pub window_mode: WindowMode,
    /// Override the back-buffer (render) resolution. `None` (default) makes it
    /// match the window client area automatically — including monitor resolution
    /// when using [`WindowMode::BorderlessWindowed`].
    ///
    /// Set explicitly for lower-resolution rendering scaled up to the window
    /// (e.g. pixel-art at `Some((320, 240))` inside an 800×600 window).
    pub render_size: Option<(u32, u32)>,
    /// When `true`, `Present` syncs to the monitor refresh rate (vsync on).
    pub vsync: bool,
    /// Maximum number of concurrent sound-effect voices in the shared pool.
    /// Music playback is separate and not counted against this limit.
    pub sound_voices: u32,
    /// Maximum number of quads (sprites + shapes) that can be batched before a
    /// draw call is flushed to the GPU.
    ///
    /// Increase for workloads that draw many sprites per frame (e.g. particle
    /// systems or stress tests). The vertex buffer is pre-allocated at startup
    /// to `batch_size × 4 × 32` bytes so there are no per-frame allocations
    /// regardless of the value chosen. Default: `16 384`.
    pub batch_size: usize,
}

impl Default for RukohConfig {
    fn default() -> Self {
        Self {
            title: "rukoh",
            window_mode: WindowMode::BorderlessWindowed,
            render_size: None,
            vsync: true,
            sound_voices: 32,
            batch_size: 2048,
        }
    }
}

// ── Rukoh ────────────────────────────────────────────────────────────────────

/// The main entry point for the library.
///
/// Owns the Win32 window, D3D11 device, sprite batch, audio device, and all
/// input state. Drive the game loop with [`next_frame`](Self::next_frame).
pub struct Rukoh {
    // Drop order: batch (GPU resources) → gfx (D3D11 device) → audio → window (HWND)
    pub(crate) batch: SpriteBatch,
    pub(crate) gfx: GfxDevice,
    pub(crate) audio: AudioDevice,
    window: Win32Window,
    input: InputState,
    hid: HidManager,
    vsync: bool,
    pub(crate) render_width: u32,
    pub(crate) render_height: u32,
    /// The RTV of the currently active render target, or `None` for the back buffer.
    pub(crate) current_rtv: Option<ID3D11RenderTargetView>,
    /// Window client area dimensions, used to map mouse coords to render-space.
    window_client_w: u32,
    window_client_h: u32,
    qpc_freq: i64,
    last_frame_time: i64,
    cursor_visible: bool,
}

impl Rukoh {
    /// Create the window and initialise Direct3D 11.
    pub fn new(config: RukohConfig) -> Result<Self, Error> {
        let mut qpc_freq: i64 = 0;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut qpc_freq);
        };

        let (window_client_w, window_client_h) = client_size(&config.window_mode);
        let (render_width, render_height) = config
            .render_size
            .unwrap_or((window_client_w, window_client_h));
        let window = Win32Window::new(config.title, &config.window_mode)?;
        let hwnd = window.hwnd;

        let gfx = GfxDevice::new(hwnd, render_width, render_height)?;
        let batch = SpriteBatch::new(
            &gfx.device,
            &gfx.context,
            render_width,
            render_height,
            config.batch_size,
        )?;
        let audio = AudioDevice::new(config.sound_voices)?;

        let mut hid = HidManager::new();
        hid.enumerate();

        Ok(Self {
            batch,
            gfx,
            audio,
            window,
            input: InputState::default(),
            hid,
            vsync: config.vsync,
            render_width,
            render_height,
            current_rtv: None,
            window_client_w,
            window_client_h,
            qpc_freq,
            last_frame_time: 0,
            cursor_visible: true,
        })
    }

    /// Pump the Win32 message queue and return a [`Frame`] for this tick.
    ///
    /// Returns `None` when the user has closed the window.
    pub fn next_frame(&mut self) -> Option<Frame<'_>> {
        // ── 1. Snapshot current state as "previous" ───────────────────────────
        self.input
            .keys_prev
            .copy_from_slice(&self.input.keys_current);
        self.input
            .mouse_buttons_prev
            .copy_from_slice(&self.input.mouse_buttons_curr);
        self.input.mouse_prev_pos = self.input.mouse_pos;
        let prev_gamepad_buttons = self.input.gamepad.map(|g| g.buttons_current).unwrap_or(0);

        // ── 2. Pump Win32 messages → window.state updated ─────────────────────
        let mut msg = MSG::default();
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        if self.window.state.should_close {
            return None;
        }

        // ── 3. Sync keyboard ──────────────────────────────────────────────────
        self.input
            .keys_current
            .copy_from_slice(&self.window.state.keys);

        self.input.last_key_pressed = (0..256).find_map(|vk| {
            if self.input.keys_current[vk] && !self.input.keys_prev[vk] {
                input::keyboard::KeyCode::from_vk(vk)
            } else {
                None
            }
        });

        // ── 4. Sync mouse ─────────────────────────────────────────────────────
        // Map window client coords → render-space coords.
        let render_x = self.window.state.mouse_x as f32 * self.render_width as f32
            / self.window_client_w as f32;
        let render_y = self.window.state.mouse_y as f32 * self.render_height as f32
            / self.window_client_h as f32;

        self.input.mouse_pos = Vec2::new(
            render_x.clamp(0.0, self.render_width as f32),
            render_y.clamp(0.0, self.render_height as f32),
        );
        self.input.mouse_delta = self.input.mouse_pos - self.input.mouse_prev_pos;
        self.input
            .mouse_buttons_curr
            .copy_from_slice(&self.window.state.mouse_buttons);

        // Consume the scroll accumulator.
        self.input.mouse_scroll = self.window.state.mouse_scroll_accum;
        self.window.state.mouse_scroll_accum = 0.0;

        // ── 5. Poll gamepad ───────────────────────────────────────────────────
        // Re-enumerate HID devices when the OS signals a connect/disconnect.
        if self.window.state.devices_changed {
            self.window.state.devices_changed = false;
            self.hid.enumerate();
        }

        // XInput wins; HID is the fallback for non-XInput controllers.
        self.input.gamepad = poll_gamepad(prev_gamepad_buttons)
            .or_else(|| self.hid.process_reports(&mut self.window.state.hid_reports));
        // Always drain buffered HID reports (even when XInput won this frame).
        self.window.state.hid_reports.clear();

        // ── 6. Delta time ─────────────────────────────────────────────────────
        let mut now: i64 = 0;
        unsafe {
            let _ = QueryPerformanceCounter(&mut now);
        };

        let dt = if self.last_frame_time == 0 {
            0.0f32
        } else {
            (now - self.last_frame_time) as f32 / self.qpc_freq as f32
        };
        self.last_frame_time = now;

        // ── 7. Bind the rendering pipeline for this frame ─────────────────────
        self.batch.begin_frame();

        Some(Frame::new(self, dt))
    }

    /// The D3D11 device. Used internally by resource constructors.
    pub(crate) fn d3d_device(&self) -> &ID3D11Device {
        &self.gfx.device
    }
}
