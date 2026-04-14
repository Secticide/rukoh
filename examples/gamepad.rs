use rukoh::{
    Colour, Font, Frame, GamepadButton, GamepadState, KeyCode, Rect, Rukoh, RukohConfig, Vec2,
    WindowMode,
};

// ── Colour palette ────────────────────────────────────────────────────────────

const COL_BG: Colour = Colour::new(0.08, 0.08, 0.08, 1.0);
const COL_BODY: Colour = Colour::new(0.18, 0.18, 0.18, 1.0);
const COL_BODY_EDGE: Colour = Colour::new(0.12, 0.12, 0.12, 1.0);
const COL_BTN_OFF: Colour = Colour::new(0.32, 0.32, 0.32, 1.0);
const COL_STICK_RING_OFF: Colour = Colour::new(0.28, 0.28, 0.28, 1.0);
const COL_STICK_RING_ON: Colour = Colour::new(0.75, 0.75, 0.75, 1.0);
const COL_STICK_INNER: Colour = Colour::new(0.12, 0.12, 0.12, 1.0);
const COL_STICK_DOT: Colour = Colour::new(0.85, 0.85, 0.85, 1.0);
const COL_TRIGGER_BG: Colour = Colour::new(0.25, 0.25, 0.25, 1.0);
const COL_TRIGGER_FILL: Colour = Colour::new(1.0, 0.75, 0.1, 1.0);
const COL_SHOULDER_OFF: Colour = Colour::new(0.3, 0.3, 0.3, 1.0);
const COL_SHOULDER_ON: Colour = Colour::WHITE;
const COL_MENU_OFF: Colour = Colour::new(0.38, 0.38, 0.38, 1.0);
const COL_MENU_ON: Colour = Colour::WHITE;
const COL_TEXT: Colour = Colour::WHITE;
const COL_TEXT_DIM: Colour = Colour::new(0.55, 0.55, 0.55, 1.0);
const COL_TEXT_LABEL: Colour = Colour::new(0.42, 0.42, 0.42, 1.0);
const COL_OVERLAY: Colour = Colour::new(0.0, 0.0, 0.0, 0.72);

// Face button colours (Xbox layout).
const COL_A: Colour = Colour::new(0.22, 0.78, 0.26, 1.0);
const COL_B: Colour = Colour::new(0.9, 0.22, 0.22, 1.0);
const COL_X: Colour = Colour::new(0.22, 0.49, 0.9, 1.0);
const COL_Y: Colour = Colour::new(0.95, 0.78, 0.1, 1.0);

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — gamepad",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        vsync: true,
        ..Default::default()
    })?;

    // Font — try examples/assets/font.ttf, then Windows system fonts.
    let font_bytes: Vec<u8> = if std::path::Path::new("examples/assets/font.ttf").exists() {
        std::fs::read("examples/assets/font.ttf").unwrap()
    } else {
        std::fs::read(r"C:\Windows\Fonts\segoeui.ttf")
            .or_else(|_| std::fs::read(r"C:\Windows\Fonts\arial.ttf"))
            .expect("No font found — place a TTF at examples/assets/font.ttf")
    };
    let font = Font::load(&app, &font_bytes, 18.0)?;
    let font_sm = Font::load(&app, &font_bytes, 13.0)?;

    while let Some(mut frame) = app.next_frame() {
        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        let gp = frame.gamepad();

        frame.clear(COL_BG);

        draw_controller(&mut frame, &font, &font_sm, gp);

        if gp.is_none() {
            // Semi-transparent overlay + message when nothing is connected.
            frame.draw_rect(Rect::new(0.0, 0.0, 800.0, 600.0), COL_OVERLAY);
            frame.draw_text(
                &font,
                "No controller connected",
                Vec2::new(264.0, 282.0),
                COL_TEXT,
            );
            frame.draw_text(
                &font_sm,
                "Connect an Xbox-compatible gamepad and press any button",
                Vec2::new(190.0, 308.0),
                COL_TEXT_DIM,
            );
        }

        frame.draw_text(
            &font_sm,
            "Esc  quit",
            Vec2::new(12.0, 578.0),
            COL_TEXT_LABEL,
        );
    }

    Ok(())
}

// ── Full controller drawing ───────────────────────────────────────────────────

fn draw_controller(frame: &mut Frame<'_>, font: &Font, font_sm: &Font, gp: Option<GamepadState>) {
    let btn = |b: GamepadButton| gp.map(|g| g.is_button_down(b)).unwrap_or(false);

    let lt = gp.map(|g| g.left_trigger()).unwrap_or(0.0);
    let rt = gp.map(|g| g.right_trigger()).unwrap_or(0.0);
    let ls = gp.map(|g| g.left_stick()).unwrap_or(Vec2::ZERO);
    let rs = gp.map(|g| g.right_stick()).unwrap_or(Vec2::ZERO);

    // ── Body silhouette ───────────────────────────────────────────────────────

    // Main torso.
    frame.draw_rect(Rect::new(157.0, 197.0, 486.0, 186.0), COL_BODY_EDGE);
    frame.draw_rect(Rect::new(161.0, 201.0, 478.0, 178.0), COL_BODY);

    // Left grip.
    frame.draw_rect(Rect::new(157.0, 359.0, 162.0, 132.0), COL_BODY_EDGE);
    frame.draw_rect(Rect::new(161.0, 363.0, 154.0, 124.0), COL_BODY);

    // Right grip.
    frame.draw_rect(Rect::new(481.0, 359.0, 162.0, 132.0), COL_BODY_EDGE);
    frame.draw_rect(Rect::new(485.0, 363.0, 154.0, 124.0), COL_BODY);

    // ── Triggers (LT / RT) ────────────────────────────────────────────────────

    draw_trigger(frame, font_sm, Vec2::new(161.0, 148.0), lt);
    draw_trigger(frame, font_sm, Vec2::new(489.0, 148.0), rt);

    frame.draw_text(font_sm, "LT", Vec2::new(224.0, 118.0), COL_TEXT_LABEL);
    frame.draw_text(font_sm, "RT", Vec2::new(552.0, 118.0), COL_TEXT_LABEL);

    // ── Shoulder buttons (LB / RB) ────────────────────────────────────────────

    frame.draw_rect(
        Rect::new(161.0, 192.0, 154.0, 13.0),
        if btn(GamepadButton::LeftShoulder) {
            COL_SHOULDER_ON
        } else {
            COL_SHOULDER_OFF
        },
    );
    frame.draw_rect(
        Rect::new(485.0, 192.0, 154.0, 13.0),
        if btn(GamepadButton::RightShoulder) {
            COL_SHOULDER_ON
        } else {
            COL_SHOULDER_OFF
        },
    );

    frame.draw_text(font_sm, "LB", Vec2::new(224.0, 192.0), COL_TEXT_LABEL);
    frame.draw_text(font_sm, "RB", Vec2::new(552.0, 192.0), COL_TEXT_LABEL);

    // ── Analogue sticks ───────────────────────────────────────────────────────

    draw_stick(
        frame,
        Vec2::new(268.0, 312.0),
        ls,
        btn(GamepadButton::LeftThumb),
    );
    draw_stick(
        frame,
        Vec2::new(462.0, 382.0),
        rs,
        btn(GamepadButton::RightThumb),
    );

    frame.draw_text(font_sm, "LS", Vec2::new(254.0, 352.0), COL_TEXT_LABEL);
    frame.draw_text(font_sm, "RS", Vec2::new(448.0, 422.0), COL_TEXT_LABEL);

    // ── D-pad ─────────────────────────────────────────────────────────────────

    draw_dpad(
        frame,
        Vec2::new(207.0, 393.0),
        btn(GamepadButton::DpadUp),
        btn(GamepadButton::DpadDown),
        btn(GamepadButton::DpadLeft),
        btn(GamepadButton::DpadRight),
    );

    // ── Face buttons (ABXY) ───────────────────────────────────────────────────

    draw_face_btn(
        frame,
        font_sm,
        Vec2::new(548.0, 352.0),
        btn(GamepadButton::South),
        COL_A,
        "A",
    );
    draw_face_btn(
        frame,
        font_sm,
        Vec2::new(582.0, 320.0),
        btn(GamepadButton::East),
        COL_B,
        "B",
    );
    draw_face_btn(
        frame,
        font_sm,
        Vec2::new(514.0, 320.0),
        btn(GamepadButton::West),
        COL_X,
        "X",
    );
    draw_face_btn(
        frame,
        font_sm,
        Vec2::new(548.0, 288.0),
        btn(GamepadButton::North),
        COL_Y,
        "Y",
    );

    // ── Menu buttons (Back / Start) ───────────────────────────────────────────

    draw_menu_btn(frame, Vec2::new(348.0, 282.0), btn(GamepadButton::Back));
    draw_menu_btn(frame, Vec2::new(452.0, 282.0), btn(GamepadButton::Start));

    frame.draw_text(font_sm, "Back", Vec2::new(328.0, 265.0), COL_TEXT_LABEL);
    frame.draw_text(font_sm, "Start", Vec2::new(432.0, 265.0), COL_TEXT_LABEL);

    // ── Raw value readout ─────────────────────────────────────────────────────

    frame.draw_text(
        font,
        "Analogue values",
        Vec2::new(30.0, 468.0),
        COL_TEXT_DIM,
    );

    let ls_str = format!("LS  x: {:+.3}   y: {:+.3}", ls.x, ls.y);
    let rs_str = format!("RS  x: {:+.3}   y: {:+.3}", rs.x, rs.y);
    let tr_str = format!("LT  {:.3}          RT  {:.3}", lt, rt);

    frame.draw_text(font_sm, &ls_str, Vec2::new(30.0, 492.0), COL_TEXT);
    frame.draw_text(font_sm, &rs_str, Vec2::new(30.0, 510.0), COL_TEXT);
    frame.draw_text(font_sm, &tr_str, Vec2::new(30.0, 528.0), COL_TEXT);
}

// ── Shape helpers ─────────────────────────────────────────────────────────────

/// Trigger bar: amber fill grows from left as value approaches 1.0.
fn draw_trigger(frame: &mut Frame<'_>, font: &Font, origin: Vec2, value: f32) {
    const W: f32 = 148.0;
    const H: f32 = 42.0;

    frame.draw_rect(Rect::new(origin.x, origin.y, W, H), COL_TRIGGER_BG);

    let fill_w = value * W;
    if fill_w > 0.5 {
        frame.draw_rect(Rect::new(origin.x, origin.y, fill_w, H), COL_TRIGGER_FILL);
    }

    frame.draw_rect_lines(Rect::new(origin.x, origin.y, W, H), 1.5, COL_BODY_EDGE);

    // Percentage centred in the bar.
    let label = format!("{}%", (value * 100.0).round() as u32);
    frame.draw_text(
        font,
        &label,
        Vec2::new(origin.x + W * 0.5 - 13.0, origin.y + 13.0),
        COL_TEXT,
    );
}

/// Analogue stick: outer ring + inner dark disc + offset dot indicating deflection.
fn draw_stick(frame: &mut Frame<'_>, centre: Vec2, value: Vec2, click: bool) {
    const OUTER_R: f32 = 33.0;
    const INNER_R: f32 = 27.0;
    const DOT_R: f32 = 9.0;
    const TRAVEL: f32 = INNER_R - DOT_R;

    frame.draw_circle(
        centre,
        OUTER_R,
        if click {
            COL_STICK_RING_ON
        } else {
            COL_STICK_RING_OFF
        },
    );
    frame.draw_circle(centre, INNER_R, COL_STICK_INNER);

    // XInput Y is up-positive; screen Y is down-positive — negate Y.
    let dot = Vec2::new(centre.x + value.x * TRAVEL, centre.y - value.y * TRAVEL);
    frame.draw_circle(dot, DOT_R, COL_STICK_DOT);
}

/// D-pad cross: four rectangular arms, each lit when its direction is held.
fn draw_dpad(frame: &mut Frame<'_>, centre: Vec2, up: bool, down: bool, left: bool, right: bool) {
    const ARM_W: f32 = 24.0;
    const ARM_H: f32 = 28.0;
    let half = ARM_W * 0.5;

    let col = |on: bool| if on { Colour::WHITE } else { COL_BTN_OFF };

    // Up.
    frame.draw_rect(
        Rect::new(centre.x - half, centre.y - half - ARM_H, ARM_W, ARM_H),
        col(up),
    );
    // Down.
    frame.draw_rect(
        Rect::new(centre.x - half, centre.y + half, ARM_W, ARM_H),
        col(down),
    );
    // Left.
    frame.draw_rect(
        Rect::new(centre.x - half - ARM_H, centre.y - half, ARM_H, ARM_W),
        col(left),
    );
    // Right.
    frame.draw_rect(
        Rect::new(centre.x + half, centre.y - half, ARM_H, ARM_W),
        col(right),
    );
    // Centre fill (always visible, neutral colour).
    frame.draw_rect(
        Rect::new(centre.x - half, centre.y - half, ARM_W, ARM_W),
        COL_BTN_OFF,
    );
}

/// Face button: circle with a letter label. Active = full colour; idle = dark.
fn draw_face_btn(
    frame: &mut Frame<'_>,
    font: &Font,
    centre: Vec2,
    active: bool,
    colour: Colour,
    label: &str,
) {
    const R: f32 = 15.0;
    const INNER_R: f32 = 12.0;

    if active {
        frame.draw_circle(centre, R, colour);
        frame.draw_circle(centre, INNER_R, colour);
    } else {
        frame.draw_circle(centre, R, COL_BTN_OFF);
        frame.draw_circle(centre, INNER_R, Colour::new(0.2, 0.2, 0.2, 1.0));
    }

    let text_col = if active {
        Colour::WHITE
    } else {
        COL_TEXT_LABEL
    };
    frame.draw_text(
        font,
        label,
        Vec2::new(centre.x - 4.0, centre.y - 7.0),
        text_col,
    );
}

/// Small circular menu button (Back / Start).
fn draw_menu_btn(frame: &mut Frame<'_>, centre: Vec2, active: bool) {
    const R: f32 = 10.0;
    frame.draw_circle(centre, R, if active { COL_MENU_ON } else { COL_MENU_OFF });
}
