use rukoh::{
    Colour, Font, GamepadButton, KeyCode, MouseButton, Rect, Rukoh, RukohConfig, Vec2, WindowMode,
};

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

    let font = Font::load(&app, include_bytes!("assets/lexend.ttf"), 18.0)?;

    let mut cursor_visible = true;

    while let Some(mut frame) = app.next_frame() {
        frame.clear(Colour::DARK_GREY);

        // ── Cursor toggle ─────────────────────────────────────────────────────
        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }
        if frame.is_key_pressed(KeyCode::H) {
            cursor_visible = !cursor_visible;
            if cursor_visible {
                frame.show_cursor();
            } else {
                frame.hide_cursor();
            }
        }

        let w = frame.width() as f32;
        let h = frame.height() as f32;
        let lh = font.line_height();
        let dim = Colour::new(0.45, 0.45, 0.45, 1.0);
        let lit = Colour::WHITE;
        let active = Colour::new(0.3, 1.0, 0.4, 1.0); // green — key/button held
        let col_left = 20.0_f32;
        let col_right = w * 0.5 + 10.0;

        // ── Helper: coloured key label ────────────────────────────────────────
        // Drawn inline below — returns the colour based on held state.

        // ── Section: Keyboard ─────────────────────────────────────────────────
        let mut y = 20.0_f32;
        frame.draw_text(&font, "KEYBOARD", Vec2::new(col_left, y), Colour::YELLOW);
        y += lh + 6.0;

        // Movement keys row
        for (key, label) in [
            (KeyCode::W, "W"),
            (KeyCode::A, "A"),
            (KeyCode::S, "S"),
            (KeyCode::D, "D"),
        ] {
            frame.draw_text(&font, label, Vec2::new(col_left, y), Colour::WHITE);
            let colour = if frame.is_key_down(key) { active } else { dim };
            frame.draw_rect(
                Rect::new(
                    col_left + font.measure_text(label).x + 4.0,
                    y + 2.0,
                    10.0,
                    lh - 4.0,
                ),
                colour,
            );
            y += lh + 2.0;
        }

        y += 4.0;
        for (key, label) in [
            (KeyCode::Up, "Up"),
            (KeyCode::Down, "Down"),
            (KeyCode::Left, "Left"),
            (KeyCode::Right, "Right"),
            (KeyCode::Space, "Space"),
        ] {
            let colour = if frame.is_key_down(key) { active } else { dim };
            frame.draw_text(&font, label, Vec2::new(col_left, y), colour);
            y += lh + 2.0;
        }

        y += 4.0;
        let cursor_label = if cursor_visible {
            "H — cursor: visible"
        } else {
            "H — cursor: hidden"
        };
        let cursor_colour = if cursor_visible { lit } else { dim };
        frame.draw_text(&font, cursor_label, Vec2::new(col_left, y), cursor_colour);

        // ── Section: Mouse ────────────────────────────────────────────────────
        let mut y = 20.0_f32;
        let pos = frame.mouse_pos();
        let delta = frame.mouse_delta();
        let scroll = frame.mouse_scroll();

        frame.draw_text(&font, "MOUSE", Vec2::new(col_right, y), Colour::YELLOW);
        y += lh + 6.0;

        frame.draw_text(
            &font,
            &format!("Position  ({:.1}, {:.1})", pos.x, pos.y),
            Vec2::new(col_right, y),
            lit,
        );
        y += lh + 2.0;
        frame.draw_text(
            &font,
            &format!("Delta     ({:.1}, {:.1})", delta.x, delta.y),
            Vec2::new(col_right, y),
            lit,
        );
        y += lh + 2.0;
        frame.draw_text(
            &font,
            &format!("Scroll    {scroll:.2}"),
            Vec2::new(col_right, y),
            lit,
        );
        y += lh + 8.0;

        for (btn, label) in [
            (MouseButton::Left, "LMB"),
            (MouseButton::Right, "RMB"),
            (MouseButton::Middle, "MMB"),
        ] {
            let colour = if frame.is_mouse_down(btn) {
                active
            } else {
                dim
            };
            frame.draw_text(&font, label, Vec2::new(col_right, y), colour);
            y += lh + 2.0;
        }

        // ── Section: Gamepad ──────────────────────────────────────────────────
        let section_y = h * 0.5 + 10.0;
        let mut y_l = section_y;
        let mut y_r = section_y;

        frame.draw_text(&font, "GAMEPAD", Vec2::new(col_left, y_l), Colour::YELLOW);
        y_l += lh + 6.0;
        frame.draw_text(&font, "GAMEPAD", Vec2::new(col_right, y_r), Colour::YELLOW);
        y_r += lh + 6.0;

        if let Some(pad) = frame.gamepad() {
            let ls = pad.left_stick();
            let rs = pad.right_stick();
            let lt = pad.left_trigger();
            let rt = pad.right_trigger();

            frame.draw_text(
                &font,
                &format!("LS  ({:+.2}, {:+.2})", ls.x, ls.y),
                Vec2::new(col_left, y_l),
                lit,
            );
            y_l += lh + 2.0;
            frame.draw_text(
                &font,
                &format!("RS  ({:+.2}, {:+.2})", rs.x, rs.y),
                Vec2::new(col_left, y_l),
                lit,
            );
            y_l += lh + 2.0;
            frame.draw_text(
                &font,
                &format!("LT  {lt:.2}   RT  {rt:.2}"),
                Vec2::new(col_left, y_l),
                lit,
            );

            for (btn, label) in [
                (GamepadButton::South, "A/Cross"),
                (GamepadButton::East, "B/Circle"),
                (GamepadButton::West, "X/Square"),
                (GamepadButton::North, "Y/Triangle"),
                (GamepadButton::LeftShoulder, "LB"),
                (GamepadButton::RightShoulder, "RB"),
                (GamepadButton::Start, "Start"),
                (GamepadButton::Back, "Back"),
                (GamepadButton::DpadUp, "DPad Up"),
                (GamepadButton::DpadDown, "DPad Down"),
                (GamepadButton::DpadLeft, "DPad Left"),
                (GamepadButton::DpadRight, "DPad Right"),
            ] {
                let colour = if pad.is_button_down(btn) { active } else { dim };
                frame.draw_text(&font, label, Vec2::new(col_right, y_r), colour);
                y_r += lh + 2.0;
            }
        } else {
            frame.draw_text(&font, "No gamepad connected", Vec2::new(col_left, y_l), dim);
        }

        // ── ESC hint ──────────────────────────────────────────────────────────
        frame.draw_text(&font, "Esc — quit", Vec2::new(col_left, h - lh - 10.0), dim);
    }

    Ok(())
}
