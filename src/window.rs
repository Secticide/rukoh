use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics,
            GetWindowLongPtrW, LoadCursorW, RegisterClassExW, SetWindowLongPtrW, CREATESTRUCTW,
            CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW, SM_CXSCREEN, SM_CYSCREEN,
            WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE,
            WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSEXW, WS_CAPTION,
            WS_MINIMIZEBOX, WS_OVERLAPPED, WS_POPUP, WS_SYSMENU, WS_VISIBLE,
        },
    },
};

use crate::{Error, WindowMode};

/// Raw per-window state written by the window procedure and read by `next_frame`.
///
/// Lives on the heap (inside a `Box`) so its address is stable for `GWLP_USERDATA`.
pub struct WindowState {
    // ── Lifecycle ─────────────────────────────────────────────────────────
    pub should_close: bool,

    // ── Keyboard ──────────────────────────────────────────────────────────
    /// `true` while a key is physically held. Indexed by Windows virtual-key code.
    pub keys: [bool; 256],

    // ── Mouse ─────────────────────────────────────────────────────────────
    /// Cursor position in window client coordinates (pixels).
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// `[Left, Right, Middle]`
    pub mouse_buttons: [bool; 3],
    /// Accumulated scroll this frame. Reset by `next_frame` after it reads the value.
    pub mouse_scroll_accum: f32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            should_close: false,
            keys: [false; 256],
            mouse_x: 0,
            mouse_y: 0,
            mouse_buttons: [false; 3],
            mouse_scroll_accum: 0.0,
        }
    }
}

pub struct Win32Window {
    pub hwnd: HWND,
    // Box gives a stable heap address for the GWLP_USERDATA pointer.
    pub state: Box<WindowState>,
}

impl Win32Window {
    pub fn new(title: &'static str, mode: &WindowMode) -> Result<Self, Error> {
        let hinstance = unsafe { GetModuleHandleW(None)? };

        let class_name = windows::core::w!("rukoh_wc");

        // Ignore ALREADY_EXISTS — multiple Rukoh instances share the class.
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            ..Default::default()
        };
        unsafe { RegisterClassExW(&wc) };

        // ALLOCATION: WindowState on the heap — required for a stable raw pointer passed as
        // lpCreateParams / stored in GWLP_USERDATA. The address must survive beyond
        // CreateWindowExW and be valid for the window's lifetime.
        let mut state = Box::new(WindowState::default());
        let state_ptr: *mut WindowState = &mut *state;

        // ALLOCATION: UTF-16 window title — one-time at startup; violates the CLAUDE.md rule
        // of no runtime encode_utf16().collect(). Avoidable for compile-time literals with w!(),
        // but title is a &'static str so w!() doesn't apply. Alternative: encode into a
        // fixed-size stack array [u16; 128] and avoid the heap entirely.
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let (style, x, y, w, h) = window_geometry(mode)?;

        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR::from_raw(title_wide.as_ptr()),
                style | WS_VISIBLE,
                x,
                y,
                w,
                h,
                None,
                None,
                hinstance,
                Some(state_ptr.cast()),
            )?
        };

        Ok(Self { hwnd, state })
    }
}

impl Drop for Win32Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn window_geometry(mode: &WindowMode) -> Result<(WINDOW_STYLE, i32, i32, i32, i32), Error> {
    match mode {
        WindowMode::Windowed { width, height } => {
            let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: *width as i32,
                bottom: *height as i32,
            };
            unsafe { AdjustWindowRect(&mut rect, style, false)? };

            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;

            let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            let x = ((screen_w - w) / 2).max(0);
            let y = ((screen_h - h) / 2).max(0);

            Ok((style, x, y, w, h))
        }
        WindowMode::BorderlessWindowed => {
            let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            Ok((WS_POPUP, 0, 0, w, h))
        }
    }
}

/// Returns the client-area dimensions (in pixels) for a given window mode.
pub fn client_size(mode: &WindowMode) -> (u32, u32) {
    match mode {
        WindowMode::Windowed { width, height } => (*width, *height),
        WindowMode::BorderlessWindowed => {
            let w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
            let h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
            (w, h)
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
    if state_ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *state_ptr;

    match msg {
        // ── Lifecycle ────────────────────────────────────────────────────────
        WM_CLOSE => {
            state.should_close = true;
            LRESULT(0) // suppress DefWindowProcW — destruction is in Win32Window::drop
        }

        // ── Keyboard ─────────────────────────────────────────────────────────
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam.0;
            if vk < 256 {
                state.keys[vk] = true;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let vk = wparam.0;
            if vk < 256 {
                state.keys[vk] = false;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        // ── Mouse position ───────────────────────────────────────────────────
        WM_MOUSEMOVE => {
            state.mouse_x = (lparam.0 & 0xFFFF) as i16 as i32;
            state.mouse_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            LRESULT(0)
        }

        // ── Mouse buttons ────────────────────────────────────────────────────
        WM_LBUTTONDOWN => {
            state.mouse_buttons[0] = true;
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            state.mouse_buttons[0] = false;
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            state.mouse_buttons[1] = true;
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            state.mouse_buttons[1] = false;
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            state.mouse_buttons[2] = true;
            LRESULT(0)
        }
        WM_MBUTTONUP => {
            state.mouse_buttons[2] = false;
            LRESULT(0)
        }

        // ── Scroll wheel ─────────────────────────────────────────────────────
        WM_MOUSEWHEEL => {
            // High word of wparam is the signed scroll delta in WHEEL_DELTA (120) units.
            let delta = ((wparam.0 >> 16) as i16) as f32 / 120.0;
            state.mouse_scroll_accum += delta;
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
