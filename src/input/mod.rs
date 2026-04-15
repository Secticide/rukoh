pub mod gamepad;
pub(crate) mod hid;
pub mod keyboard;

pub use gamepad::{GamepadBackend, GamepadButton, GamepadState};
pub use keyboard::KeyCode;

use crate::maths::Vec2;

/// Mouse button identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
    Middle = 2,
}

/// All input state for one frame, owned by [`Rukoh`](crate::Rukoh).
///
/// Keyboard and mouse are updated from Win32 messages pumped in
/// `next_frame`. Gamepad is polled via XInput.
pub(crate) struct InputState {
    // ── Keyboard ─────────────────────────────────────────────────────────────
    pub keys_current: [bool; 256],
    pub keys_prev: [bool; 256],
    /// The first key that transitioned to pressed this frame, or `None`.
    pub last_key_pressed: Option<KeyCode>,

    // ── Mouse (render-space) ─────────────────────────────────────────────────
    pub mouse_pos: Vec2,
    pub mouse_delta: Vec2,
    pub mouse_prev_pos: Vec2,
    pub mouse_buttons_curr: [bool; 3],
    pub mouse_buttons_prev: [bool; 3],
    pub mouse_scroll: f32,

    // ── Gamepad ───────────────────────────────────────────────────────────────
    pub gamepad: Option<GamepadState>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            keys_current: [false; 256],
            keys_prev: [false; 256],
            last_key_pressed: None,
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            mouse_prev_pos: Vec2::ZERO,
            mouse_buttons_curr: [false; 3],
            mouse_buttons_prev: [false; 3],
            mouse_scroll: 0.0,
            gamepad: None,
        }
    }
}
