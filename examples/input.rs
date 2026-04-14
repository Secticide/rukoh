use rukoh::{Colour, GamepadButton, KeyCode, MouseButton, Rukoh, RukohConfig, WindowMode};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — input",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        vsync: true,
        ..Default::default()
    })?;

    println!("Click the window to focus it, then:");
    println!("  - Press keys to see keyboard events");
    println!("  - Move the mouse and click buttons");
    println!("  - Plug in a gamepad to see stick/button values");
    println!("  - Press Escape to quit");

    while let Some(mut frame) = app.next_frame() {
        frame.clear(Colour::DARK_GREY);

        // ── Keyboard ──────────────────────────────────────────────────────────
        if frame.is_key_pressed(KeyCode::Escape) {
            println!("Escape pressed — exiting.");
            break;
        }
        if frame.is_key_pressed(KeyCode::Space) {
            println!("Space PRESSED");
        }
        if frame.is_key_released(KeyCode::Space) {
            println!("Space RELEASED");
        }

        // Log any letter key presses (A-Z)
        for (key, name) in [
            (KeyCode::A, "A"),
            (KeyCode::B, "B"),
            (KeyCode::C, "C"),
            (KeyCode::D, "D"),
            (KeyCode::E, "E"),
            (KeyCode::F, "F"),
            (KeyCode::W, "W"),
            (KeyCode::S, "S"),
            (KeyCode::Left, "Left"),
            (KeyCode::Right, "Right"),
            (KeyCode::Up, "Up"),
            (KeyCode::Down, "Down"),
        ] {
            if frame.is_key_pressed(key) {
                println!("Key pressed:  {name}");
            }
        }

        // ── Mouse ─────────────────────────────────────────────────────────────
        let pos = frame.mouse_pos();
        let delta = frame.mouse_delta();
        let scroll = frame.mouse_scroll();

        if delta.x.abs() > 0.5 || delta.y.abs() > 0.5 {
            println!(
                "Mouse  pos=({:.1}, {:.1})  delta=({:.1}, {:.1})",
                pos.x, pos.y, delta.x, delta.y
            );
        }
        if scroll.abs() > f32::EPSILON {
            println!("Scroll  {scroll:.2}");
        }
        if frame.is_mouse_pressed(MouseButton::Left) {
            println!("LMB pressed  at ({:.1}, {:.1})", pos.x, pos.y);
        }
        if frame.is_mouse_pressed(MouseButton::Right) {
            println!("RMB pressed  at ({:.1}, {:.1})", pos.x, pos.y);
        }

        // ── Gamepad ───────────────────────────────────────────────────────────
        if let Some(pad) = frame.gamepad() {
            let ls = pad.left_stick();
            let rs = pad.right_stick();
            let lt = pad.left_trigger();
            let rt = pad.right_trigger();

            if ls.length_sq() > 0.01 || rs.length_sq() > 0.01 || lt > 0.05 || rt > 0.05 {
                println!(
                    "Gamepad  LS=({:.2},{:.2})  RS=({:.2},{:.2})  LT={lt:.2}  RT={rt:.2}",
                    ls.x, ls.y, rs.x, rs.y
                );
            }

            for (btn, name) in [
                (GamepadButton::South, "South (A/Cross)"),
                (GamepadButton::East, "East  (B/Circle)"),
                (GamepadButton::West, "West  (X/Square)"),
                (GamepadButton::North, "North (Y/Triangle)"),
                (GamepadButton::Start, "Start"),
                (GamepadButton::Back, "Back"),
                (GamepadButton::LeftShoulder, "LB"),
                (GamepadButton::RightShoulder, "RB"),
                (GamepadButton::DpadUp, "DPad Up"),
                (GamepadButton::DpadDown, "DPad Down"),
            ] {
                if pad.is_button_pressed(btn) {
                    println!("Gamepad  {name} PRESSED");
                }
            }
        }
    }

    Ok(())
}
