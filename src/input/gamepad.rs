use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};

use crate::math::Vec2;

/// Radial dead zone applied to both analogue sticks.
const STICK_DEADZONE: f32 = 0.24;

/// A gamepad button, named by position (hardware-agnostic).
///
/// | Variant          | Xbox | PlayStation |
/// |------------------|------|-------------|
/// | `South`          | A    | Cross       |
/// | `East`           | B    | Circle      |
/// | `West`           | X    | Square      |
/// | `North`          | Y    | Triangle    |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South, // A / Cross
    East,  // B / Circle
    West,  // X / Square
    North, // Y / Triangle
    LeftShoulder,
    RightShoulder,
    LeftThumb,
    RightThumb,
    Start,
    Back,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl GamepadButton {
    fn xinput_mask(self) -> u16 {
        match self {
            Self::DpadUp => 0x0001,
            Self::DpadDown => 0x0002,
            Self::DpadLeft => 0x0004,
            Self::DpadRight => 0x0008,
            Self::Start => 0x0010,
            Self::Back => 0x0020,
            Self::LeftThumb => 0x0040,
            Self::RightThumb => 0x0080,
            Self::LeftShoulder => 0x0100,
            Self::RightShoulder => 0x0200,
            Self::South => 0x1000, // A
            Self::East => 0x2000,  // B
            Self::West => 0x4000,  // X
            Self::North => 0x8000, // Y
        }
    }
}

/// Snapshot of the connected gamepad state for a single frame.
///
/// Returned by [`Frame::gamepad`](crate::Frame::gamepad).
/// `None` means no controller is connected.
#[derive(Clone, Copy, Debug, Default)]
pub struct GamepadState {
    pub(crate) buttons_current: u16,
    pub(crate) buttons_prev: u16,
    left_stick: Vec2,
    right_stick: Vec2,
    left_trigger: f32,
    right_trigger: f32,
}

impl GamepadState {
    /// `true` while the button is held this frame.
    #[inline]
    pub fn is_button_down(&self, button: GamepadButton) -> bool {
        self.buttons_current & button.xinput_mask() != 0
    }

    /// `true` on the first frame the button is pressed (rising edge).
    #[inline]
    pub fn is_button_pressed(&self, button: GamepadButton) -> bool {
        let m = button.xinput_mask();
        self.buttons_current & m != 0 && self.buttons_prev & m == 0
    }

    /// `true` on the first frame the button is released (falling edge).
    #[inline]
    pub fn is_button_released(&self, button: GamepadButton) -> bool {
        let m = button.xinput_mask();
        self.buttons_current & m == 0 && self.buttons_prev & m != 0
    }

    /// Left analogue stick as a unit-clamped [`Vec2`], radial dead zone applied.
    #[inline]
    pub fn left_stick(&self) -> Vec2 {
        self.left_stick
    }

    /// Right analogue stick as a unit-clamped [`Vec2`], radial dead zone applied.
    #[inline]
    pub fn right_stick(&self) -> Vec2 {
        self.right_stick
    }

    /// Left trigger in `[0.0, 1.0]`.
    #[inline]
    pub fn left_trigger(&self) -> f32 {
        self.left_trigger
    }

    /// Right trigger in `[0.0, 1.0]`.
    #[inline]
    pub fn right_trigger(&self) -> f32 {
        self.right_trigger
    }
}

/// Poll XInput slot 0. Returns `None` if no controller is connected.
/// `prev` is passed in so the previous button state can be preserved for
/// pressed/released edge detection.
pub(crate) fn poll_gamepad(prev_buttons: u16) -> Option<GamepadState> {
    let mut raw = XINPUT_STATE::default();
    let result = unsafe { XInputGetState(0, &mut raw) };

    // ERROR_SUCCESS = 0; anything else means disconnected.
    if result != 0 {
        return None;
    }

    let gp = &raw.Gamepad;

    Some(GamepadState {
        buttons_current: gp.wButtons.0,
        buttons_prev: prev_buttons,
        left_stick: normalise_stick(gp.sThumbLX, gp.sThumbLY),
        right_stick: normalise_stick(gp.sThumbRX, gp.sThumbRY),
        left_trigger: gp.bLeftTrigger as f32 / 255.0,
        right_trigger: gp.bRightTrigger as f32 / 255.0,
    })
}

/// Convert a raw XInput i16 stick pair into a normalised Vec2 with a
/// radial dead zone applied.
fn normalise_stick(raw_x: i16, raw_y: i16) -> Vec2 {
    // Normalise to [-1, 1], clamping the asymmetric i16 minimum.
    let x = (raw_x as f32).clamp(-32767.0, 32767.0) / 32767.0;
    let y = (raw_y as f32).clamp(-32767.0, 32767.0) / 32767.0;

    let len = (x * x + y * y).sqrt();
    if len < STICK_DEADZONE {
        return Vec2::ZERO;
    }

    // Rescale so the dead zone edge maps to 0 and the outer edge maps to 1.
    let scale = ((len - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)).min(1.0) / len;
    Vec2::new(x * scale, y * scale)
}
