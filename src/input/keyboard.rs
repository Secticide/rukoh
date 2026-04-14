/// A keyboard key, identified by its Windows virtual-key code.
///
/// Note: `Shift`, `Ctrl`, and `Alt` detect either side. The sided variants
/// (`LeftShift`, `RightShift`, etc.) rely on Windows sending distinct VK codes,
/// which only occurs for WM_KEYDOWN when the scan code differs — behaviour is
/// correct on all standard keyboards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum KeyCode {
    // ── Letters ──────────────────────────────────────────────────────────────
    A = 0x41,
    B = 0x42,
    C = 0x43,
    D = 0x44,
    E = 0x45,
    F = 0x46,
    G = 0x47,
    H = 0x48,
    I = 0x49,
    J = 0x4A,
    K = 0x4B,
    L = 0x4C,
    M = 0x4D,
    N = 0x4E,
    O = 0x4F,
    P = 0x50,
    Q = 0x51,
    R = 0x52,
    S = 0x53,
    T = 0x54,
    U = 0x55,
    V = 0x56,
    W = 0x57,
    X = 0x58,
    Y = 0x59,
    Z = 0x5A,

    // ── Top-row digits ───────────────────────────────────────────────────────
    Key0 = 0x30,
    Key1 = 0x31,
    Key2 = 0x32,
    Key3 = 0x33,
    Key4 = 0x34,
    Key5 = 0x35,
    Key6 = 0x36,
    Key7 = 0x37,
    Key8 = 0x38,
    Key9 = 0x39,

    // ── Function keys ────────────────────────────────────────────────────────
    F1 = 0x70,
    F2 = 0x71,
    F3 = 0x72,
    F4 = 0x73,
    F5 = 0x74,
    F6 = 0x75,
    F7 = 0x76,
    F8 = 0x77,
    F9 = 0x78,
    F10 = 0x79,
    F11 = 0x7A,
    F12 = 0x7B,

    // ── Navigation ───────────────────────────────────────────────────────────
    Left = 0x25,
    Up = 0x26,
    Right = 0x27,
    Down = 0x28,
    Home = 0x24,
    End = 0x23,
    PageUp = 0x21,
    PageDown = 0x22,
    Insert = 0x2D,
    Delete = 0x2E,

    // ── Editing / whitespace ─────────────────────────────────────────────────
    Escape = 0x1B,
    Enter = 0x0D,
    Tab = 0x09,
    Backspace = 0x08,
    Space = 0x20,

    // ── Modifiers (generic — either side) ────────────────────────────────────
    Shift = 0x10,
    Ctrl = 0x11,
    Alt = 0x12,

    // ── Modifiers (sided) ────────────────────────────────────────────────────
    LeftShift = 0xA0,
    RightShift = 0xA1,
    LeftCtrl = 0xA2,
    RightCtrl = 0xA3,
    LeftAlt = 0xA4,
    RightAlt = 0xA5,

    // ── Lock keys ────────────────────────────────────────────────────────────
    CapsLock = 0x14,
    NumLock = 0x90,
    ScrollLock = 0x91,

    // ── System ───────────────────────────────────────────────────────────────
    PrintScreen = 0x2C,
    Pause = 0x13,

    // ── Numpad ───────────────────────────────────────────────────────────────
    Numpad0 = 0x60,
    Numpad1 = 0x61,
    Numpad2 = 0x62,
    Numpad3 = 0x63,
    Numpad4 = 0x64,
    Numpad5 = 0x65,
    Numpad6 = 0x66,
    Numpad7 = 0x67,
    Numpad8 = 0x68,
    Numpad9 = 0x69,
    NumpadAdd = 0x6B,
    NumpadSub = 0x6D,
    NumpadMul = 0x6A,
    NumpadDiv = 0x6F,
    NumpadDecimal = 0x6E,

    // ── Punctuation (OEM) ────────────────────────────────────────────────────
    Semicolon = 0xBA,    // ; :
    Equals = 0xBB,       // = +
    Comma = 0xBC,        // , <
    Minus = 0xBD,        // - _
    Period = 0xBE,       // . >
    Slash = 0xBF,        // / ?
    Backtick = 0xC0,     // ` ~
    LeftBracket = 0xDB,  // [ {
    Backslash = 0xDC,    // \ |
    RightBracket = 0xDD, // ] }
    Apostrophe = 0xDE,   // ' "
}

impl KeyCode {
    /// Returns the Windows virtual-key code for this key.
    #[inline]
    pub(crate) fn vk(self) -> usize {
        self as u8 as usize
    }
}
