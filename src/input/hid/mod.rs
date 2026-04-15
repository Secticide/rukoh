mod profiles;

use windows::Win32::{
    Devices::HumanInterfaceDevice::{
        HidP_GetCaps, HidP_GetUsageValue, HidP_GetUsages, HidP_GetValueCaps, HidP_Input, HIDP_CAPS,
        HIDP_VALUE_CAPS, PHIDP_PREPARSED_DATA,
    },
    Foundation::HANDLE,
    UI::Input::{
        GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICEINFO,
        RIDI_DEVICENAME, RIDI_PREPARSEDDATA, RID_DEVICE_INFO, RIM_TYPEHID,
    },
};

use self::profiles::{profile_for, ControllerProfile};
use crate::{
    input::gamepad::{GamepadBackend, GamepadButton, GamepadState},
    maths::Vec2,
};

// ── Public types ─────────────────────────────────────────────────────────────

/// A single raw HID report received via `WM_INPUT`.
pub(crate) struct HidReport {
    /// Raw-input device handle stored as `isize` (from `RAWINPUTHEADER::hDevice.0`).
    pub device: isize,
    /// Complete HID input report bytes (including report-ID byte if used).
    pub data: Vec<u8>,
}

const STICK_DEADZONE: f32 = 0.24;

// ── HidDevice ─────────────────────────────────────────────────────────────────

struct HidDevice {
    /// Raw-input device handle (stored as isize from `HANDLE.0 as isize`).
    ri_handle: isize,
    /// Preparsed data buffer allocated with `u64` alignment.
    /// The contained bytes are cast to `PHIDP_PREPARSED_DATA` when calling HidP functions.
    preparsed: Vec<u64>,
    #[allow(dead_code)]
    input_report_len: u32,
    profile: ControllerProfile,
    /// Logical (min, max) range for each axis usage, from `HidP_GetValueCaps`.
    lx_range: (i32, i32),
    ly_range: (i32, i32),
    rx_range: (i32, i32),
    ry_range: (i32, i32),
    lt_range: (i32, i32),
    rt_range: (i32, i32),
    buttons_prev: u16,
    buttons_current: u16,
    left_stick: Vec2,
    right_stick: Vec2,
    left_trigger: f32,
    right_trigger: f32,
    /// `true` once at least one report has been received. Prevents returning
    /// default-zero state before the device has been heard from.
    has_report: bool,
}

impl HidDevice {
    fn preparsed_handle(&self) -> PHIDP_PREPARSED_DATA {
        PHIDP_PREPARSED_DATA(self.preparsed.as_ptr() as isize)
    }

    /// Parse one raw HID input report and update internal state.
    ///
    /// `data` must be a mutable slice to satisfy `HidP_GetUsages`'s API
    /// (which takes `&mut [u8]`); the function does not actually modify it.
    fn parse_report(&mut self, data: &mut [u8]) {
        self.has_report = true;
        self.buttons_prev = self.buttons_current;
        self.buttons_current = 0;

        let preparsed = self.preparsed_handle();

        unsafe {
            // ── Buttons (Usage Page 0x09) ──────────────────────────────────────
            let mut usage_count: u32 = 64;
            let mut usages = [0u16; 64];
            // Return value ignored: HIDP_STATUS_BUFFER_TOO_SMALL just means
            // more buttons were pressed than our 64-slot buffer, which cannot
            // happen on a standard gamepad.
            let _ = HidP_GetUsages(
                HidP_Input,
                0x09, // Generic Desktop buttons usage page
                0,    // all link collections
                usages.as_mut_ptr(),
                &mut usage_count,
                preparsed,
                data, // &mut [u8]
            );
            let map = self.profile.buttons;
            for &raw_usage in &usages[..usage_count as usize] {
                let idx = raw_usage as usize;
                if idx >= 1 && idx <= map.len() {
                    if let Some(btn) = map[idx - 1] {
                        self.buttons_current |= btn.xinput_mask();
                    }
                }
            }

            // ── Hat switch → D-pad (Usage Page 0x01, Usage 0x39) ──────────────
            let mut hat: u32 = 0;
            if HidP_GetUsageValue(HidP_Input, 0x01, 0, 0x39, &mut hat, preparsed, data).is_ok() {
                self.buttons_current |= hat_to_dpad_bits(hat);
            }

            // ── Analogue axes (Usage Page 0x01) ────────────────────────────────
            let lx = read_axis(preparsed, data, self.profile.left_x, self.lx_range, false);
            let ly = read_axis(
                preparsed,
                data,
                self.profile.left_y,
                self.ly_range,
                self.profile.invert_left_y,
            );
            self.left_stick = apply_deadzone(lx, ly);

            let rx = read_axis(preparsed, data, self.profile.right_x, self.rx_range, false);
            let ry = read_axis(
                preparsed,
                data,
                self.profile.right_y,
                self.ry_range,
                self.profile.invert_right_y,
            );
            self.right_stick = apply_deadzone(rx, ry);

            self.left_trigger = read_trigger(preparsed, data, self.profile.lt_usage, self.lt_range);
            self.right_trigger =
                read_trigger(preparsed, data, self.profile.rt_usage, self.rt_range);
        }
    }
}

// ── Axis helpers ──────────────────────────────────────────────────────────────

/// Read one analogue axis and normalise to `[-1.0, 1.0]`, centred on the
/// logical midpoint. Inverts if `invert` is true.
///
/// # Safety
/// `preparsed` must be a valid `PHIDP_PREPARSED_DATA` handle for the device
/// that produced `data`.
unsafe fn read_axis(
    preparsed: PHIDP_PREPARSED_DATA,
    data: &[u8],
    usage: u16,
    range: (i32, i32),
    invert: bool,
) -> f32 {
    let mut raw: u32 = 0;
    if !HidP_GetUsageValue(HidP_Input, 0x01, 0, usage, &mut raw, preparsed, data).is_ok() {
        return 0.0;
    }
    let (min, max) = range;
    if max <= min {
        return 0.0;
    }
    let mid = (min + max) / 2;
    let half = (max - min) as f32 / 2.0;
    let v = ((raw as i32).wrapping_sub(mid)) as f32 / half;
    let v = v.clamp(-1.0, 1.0);
    if invert {
        -v
    } else {
        v
    }
}

/// Read a trigger axis and normalise to `[0.0, 1.0]`.
///
/// # Safety
/// Same requirements as [`read_axis`].
unsafe fn read_trigger(
    preparsed: PHIDP_PREPARSED_DATA,
    data: &[u8],
    usage: u16,
    range: (i32, i32),
) -> f32 {
    let mut raw: u32 = 0;
    if !HidP_GetUsageValue(HidP_Input, 0x01, 0, usage, &mut raw, preparsed, data).is_ok() {
        return 0.0;
    }
    let (min, max) = range;
    if max <= min {
        return 0.0;
    }
    ((raw as i32 - min) as f32 / (max - min) as f32).clamp(0.0, 1.0)
}

/// Convert a hat-switch value (0–7 = 8 compass directions, anything else =
/// centred) into the D-pad bitmask used by `GamepadState::buttons_current`.
fn hat_to_dpad_bits(hat: u32) -> u16 {
    let up = GamepadButton::DpadUp.xinput_mask();
    let down = GamepadButton::DpadDown.xinput_mask();
    let left = GamepadButton::DpadLeft.xinput_mask();
    let right = GamepadButton::DpadRight.xinput_mask();
    match hat {
        0 => up,
        1 => up | right,
        2 => right,
        3 => down | right,
        4 => down,
        5 => down | left,
        6 => left,
        7 => up | left,
        _ => 0, // centred (value 8) or out of range
    }
}

/// Apply a radial dead zone and rescale so the outer edge maps to 1.0.
fn apply_deadzone(x: f32, y: f32) -> Vec2 {
    let len = (x * x + y * y).sqrt();
    if len < STICK_DEADZONE {
        return Vec2::ZERO;
    }
    let scale = ((len - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)).min(1.0) / len;
    Vec2::new(x * scale, y * scale)
}

// ── HidManager ────────────────────────────────────────────────────────────────

/// Manages all non-XInput HID gamepad devices.
///
/// Call [`enumerate`](Self::enumerate) at startup and whenever
/// `WindowState::devices_changed` is set. Call
/// [`process_reports`](Self::process_reports) each frame.
pub(crate) struct HidManager {
    devices: Vec<HidDevice>,
}

impl HidManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    /// Rebuild the device list from scratch.
    ///
    /// Discards all previous device state; call after connect/disconnect
    /// notifications.
    pub fn enumerate(&mut self) {
        self.devices.clear();
        unsafe { self.enumerate_inner() }
    }

    unsafe fn enumerate_inner(&mut self) {
        let entry_size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
        let mut count: u32 = 0;

        // First call: get the device count.
        if GetRawInputDeviceList(None, &mut count, entry_size) == u32::MAX || count == 0 {
            return;
        }

        let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
        if GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, entry_size) == u32::MAX {
            return;
        }

        for entry in &list[..count as usize] {
            if entry.dwType != RIM_TYPEHID {
                continue;
            }
            if let Some(dev) = build_device(entry.hDevice) {
                self.devices.push(dev);
            }
        }
    }

    /// Parse all buffered HID reports and return a `GamepadState` from the
    /// first connected device, or `None` if no HID gamepad is active.
    ///
    /// The `reports` vec is drained by this call.
    pub fn process_reports(&mut self, reports: &mut Vec<HidReport>) -> Option<GamepadState> {
        for report in reports.iter_mut() {
            if let Some(dev) = self
                .devices
                .iter_mut()
                .find(|d| d.ri_handle == report.device)
            {
                dev.parse_report(&mut report.data);
            }
        }
        reports.clear();

        // Return state only after the first real report has been received.
        self.devices
            .first()
            .filter(|d| d.has_report)
            .map(|dev| GamepadState {
                buttons_current: dev.buttons_current,
                buttons_prev: dev.buttons_prev,
                left_stick: dev.left_stick,
                right_stick: dev.right_stick,
                left_trigger: dev.left_trigger,
                right_trigger: dev.right_trigger,
                backend: GamepadBackend::Hid,
            })
    }
}

// ── Device construction ───────────────────────────────────────────────────────

/// Attempt to build a `HidDevice` from a raw-input device handle.
///
/// Returns `None` if the device is an XInput device (already handled by
/// `XInputGetState`), is not a gamepad/joystick, or on any API error.
///
/// # Safety
/// `handle` must be a valid raw-input device handle.
unsafe fn build_device(handle: HANDLE) -> Option<HidDevice> {
    // ── 1. Skip XInput devices — they always have "IG_" in their path ─────
    let mut name_len: u32 = 0;
    GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, None, &mut name_len);
    if name_len == 0 {
        return None;
    }
    let mut name_buf = vec![0u16; name_len as usize];
    GetRawInputDeviceInfoW(
        handle,
        RIDI_DEVICENAME,
        Some(name_buf.as_mut_ptr() as *mut _),
        &mut name_len,
    );
    let name = String::from_utf16_lossy(&name_buf[..name_len.saturating_sub(1) as usize]);
    if name.contains("IG_") {
        return None;
    }

    // ── 2. Check usage page/usage; read VID/PID ────────────────────────────
    let mut info_size = std::mem::size_of::<RID_DEVICE_INFO>() as u32;
    let mut info = RID_DEVICE_INFO {
        cbSize: info_size,
        ..Default::default()
    };
    if GetRawInputDeviceInfoW(
        handle,
        RIDI_DEVICEINFO,
        Some(&mut info as *mut _ as *mut _),
        &mut info_size,
    ) == u32::MAX
    {
        return None;
    }
    // Safety: RIDI_DEVICEINFO succeeded and dwType == RIM_TYPEHID for this
    // entry, so the `hid` union variant is valid.
    let hid_info = info.Anonymous.hid;
    if hid_info.usUsagePage != 0x01 || (hid_info.usUsage != 0x05 && hid_info.usUsage != 0x04) {
        return None; // not a gamepad or joystick
    }
    let vid = hid_info.dwVendorId as u16;
    let pid = hid_info.dwProductId as u16;

    // ── 3. Fetch preparsed data ────────────────────────────────────────────
    let mut preparsed_size: u32 = 0;
    GetRawInputDeviceInfoW(handle, RIDI_PREPARSEDDATA, None, &mut preparsed_size);
    if preparsed_size == 0 {
        return None;
    }
    // Allocate as u64 so the buffer is 8-byte aligned (required by HIDP_PREPARSED_DATA).
    let word_count = (preparsed_size as usize).div_ceil(8);
    let mut preparsed_buf = vec![0u64; word_count];
    if GetRawInputDeviceInfoW(
        handle,
        RIDI_PREPARSEDDATA,
        Some(preparsed_buf.as_mut_ptr() as *mut _),
        &mut preparsed_size,
    ) == u32::MAX
    {
        return None;
    }

    let preparsed = PHIDP_PREPARSED_DATA(preparsed_buf.as_ptr() as isize);

    // ── 4. Read capabilities ───────────────────────────────────────────────
    let mut caps = HIDP_CAPS::default();
    if !HidP_GetCaps(preparsed, &mut caps).is_ok() {
        return None;
    }

    // ── 5. Extract axis logical ranges from value caps ─────────────────────
    let profile = profile_for(vid, pid);
    let axis_usages = [
        profile.left_x,
        profile.left_y,
        profile.right_x,
        profile.right_y,
        profile.lt_usage,
        profile.rt_usage,
    ];
    let mut ranges = [(0i32, 255i32); 6]; // sensible default for 8-bit axes

    if caps.NumberInputValueCaps > 0 {
        let mut num = caps.NumberInputValueCaps;
        let mut vcaps: Vec<HIDP_VALUE_CAPS> =
            (0..num as usize).map(|_| std::mem::zeroed()).collect();
        let _ = HidP_GetValueCaps(HidP_Input, vcaps.as_mut_ptr(), &mut num, preparsed);

        for vc in &vcaps[..num as usize] {
            if vc.UsagePage != 0x01 {
                continue;
            }
            if vc.IsRange.as_bool() {
                continue; // range caps not used by our known profiles
            }
            // Safety: IsRange is false → NotRange variant is valid.
            let usage = vc.Anonymous.NotRange.Usage;
            let range = (vc.LogicalMin, vc.LogicalMax);
            for (i, &au) in axis_usages.iter().enumerate() {
                if au == usage {
                    ranges[i] = range;
                }
            }
        }
    }

    Some(HidDevice {
        ri_handle: handle.0 as isize,
        preparsed: preparsed_buf,
        input_report_len: caps.InputReportByteLength as u32,
        profile,
        lx_range: ranges[0],
        ly_range: ranges[1],
        rx_range: ranges[2],
        ry_range: ranges[3],
        lt_range: ranges[4],
        rt_range: ranges[5],
        buttons_prev: 0,
        buttons_current: 0,
        left_stick: Vec2::ZERO,
        right_stick: Vec2::ZERO,
        left_trigger: 0.0,
        right_trigger: 0.0,
        has_report: false,
    })
}
