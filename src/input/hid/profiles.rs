use crate::input::gamepad::GamepadButton;

/// Describes how to decode HID reports for a particular controller family.
pub(crate) struct ControllerProfile {
    /// Button map: index = (HID button usage − 1). `None` = ignore.
    pub buttons: &'static [Option<GamepadButton>],
    /// HID Generic Desktop axis usages for each analogue input.
    pub left_x: u16,
    pub left_y: u16,
    pub right_x: u16,
    pub right_y: u16,
    pub lt_usage: u16,
    pub rt_usage: u16,
    /// `true` when the axis is physically inverted relative to our convention
    /// (HID Y+ = down; we want Y+ = up to match XInput).
    pub invert_left_y: bool,
    pub invert_right_y: bool,
}

// ── Button maps ───────────────────────────────────────────────────────────────

// PS4 DualShock 4 / PS5 DualSense button layout.
// HID button usages are 1-based; this slice is indexed by (usage − 1).
static PS4_BUTTONS: &[Option<GamepadButton>] = &[
    Some(GamepadButton::West),          // 1  Square
    Some(GamepadButton::South),         // 2  Cross
    Some(GamepadButton::East),          // 3  Circle
    Some(GamepadButton::North),         // 4  Triangle
    Some(GamepadButton::LeftShoulder),  // 5  L1
    Some(GamepadButton::RightShoulder), // 6  R1
    None,                               // 7  L2 digital (we use axis Rx)
    None,                               // 8  R2 digital (we use axis Ry)
    Some(GamepadButton::Back),          // 9  Share / Create
    Some(GamepadButton::Start),         // 10 Options
    Some(GamepadButton::LeftThumb),     // 11 L3
    Some(GamepadButton::RightThumb),    // 12 R3
];

// Switch Pro / generic USB gamepad button layout.
static GENERIC_BUTTONS: &[Option<GamepadButton>] = &[
    Some(GamepadButton::South),         // 1
    Some(GamepadButton::East),          // 2
    Some(GamepadButton::West),          // 3
    Some(GamepadButton::North),         // 4
    Some(GamepadButton::LeftShoulder),  // 5
    Some(GamepadButton::RightShoulder), // 6
    None,                               // 7
    None,                               // 8
    Some(GamepadButton::Back),          // 9
    Some(GamepadButton::Start),         // 10
    Some(GamepadButton::LeftThumb),     // 11
    Some(GamepadButton::RightThumb),    // 12
];

// ── Profile constructors ──────────────────────────────────────────────────────

fn ps4_profile() -> ControllerProfile {
    ControllerProfile {
        buttons: PS4_BUTTONS,
        left_x: 0x30,
        left_y: 0x31,
        right_x: 0x32,
        right_y: 0x35,
        lt_usage: 0x33,
        rt_usage: 0x34,
        invert_left_y: true,
        invert_right_y: true,
    }
}

fn switch_pro_profile() -> ControllerProfile {
    ControllerProfile {
        buttons: GENERIC_BUTTONS,
        left_x: 0x30,
        left_y: 0x31,
        right_x: 0x32,
        right_y: 0x35,
        lt_usage: 0x33,
        rt_usage: 0x34,
        invert_left_y: true,
        invert_right_y: true,
    }
}

fn generic_profile() -> ControllerProfile {
    ControllerProfile {
        buttons: GENERIC_BUTTONS,
        left_x: 0x30,
        left_y: 0x31,
        right_x: 0x32,
        right_y: 0x35,
        lt_usage: 0x33,
        rt_usage: 0x34,
        invert_left_y: true,
        invert_right_y: true,
    }
}

// ── VID/PID lookup ────────────────────────────────────────────────────────────

/// Return the controller profile for the given USB vendor/product ID pair.
/// Falls back to a generic profile for unknown devices.
pub(crate) fn profile_for(vid: u16, pid: u16) -> ControllerProfile {
    match (vid, pid) {
        (0x054C, 0x05C4) | (0x054C, 0x09CC) => ps4_profile(), // DualShock 4 v1, v2
        (0x054C, 0x0CE6) => ps4_profile(),                    // DualSense
        (0x057E, 0x2009) => switch_pro_profile(),             // Switch Pro
        _ => generic_profile(),
    }
}
