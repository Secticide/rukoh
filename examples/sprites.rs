use rukoh::{
    graphics::Texture2D, BlendMode, Colour, DrawParams, KeyCode, Rect, Rukoh, RukohConfig,
    TextureFilter, Vec2, WindowMode,
};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — sprites",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        ..Default::default()
    })?;

    // ── Load a texture from a file ────────────────────────────────────────────
    //
    // Texture2D::load accepts any &[u8] — use include_bytes! to embed the asset
    // at compile time, or read it at runtime for dev convenience.
    let uv = Texture2D::load(
        &app,
        include_bytes!("assets/uv-texture.png"),
        TextureFilter::Bilinear,
    )?;

    // Scale the texture so its longest side is at most 160 px.
    let uv_scale = 160.0 / uv.width.max(uv.height) as f32;
    let uv_w = uv.width as f32 * uv_scale;
    let uv_h = uv.height as f32 * uv_scale;

    // ── Build a 32×32 checkerboard texture from raw RGBA pixels ──────────────
    const SIZE: u32 = 32;
    const CELL: u32 = 8;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
            let checker = (x / CELL + y / CELL).is_multiple_of(2);
            if checker {
                pixels[idx..idx + 4].copy_from_slice(&[255, 165, 0, 255]); // orange
            } else {
                pixels[idx..idx + 4].copy_from_slice(&[80, 80, 80, 255]); // dark grey
            }
        }
    }
    let checker = Texture2D::from_pixels(&app, &pixels, SIZE, SIZE, TextureFilter::Point)?;

    // ── Sprite state ──────────────────────────────────────────────────────────

    // UV texture — bounces around, slow counter-clockwise rotation.
    let mut uv_pos = Vec2::new(400.0, 220.0);
    let mut uv_vel = Vec2::new(-190.0, 145.0);
    let mut uv_rot = 0.0f32;

    // Checkerboard — bounces independently, faster clockwise rotation.
    const CHECKER_SIZE: f32 = 64.0;
    let mut ch_pos = Vec2::new(100.0, 100.0);
    let mut ch_vel = Vec2::new(220.0, 170.0);
    let mut ch_rot = 0.0f32;

    while let Some(mut frame) = app.next_frame() {
        let dt = frame.delta_time();
        let w = frame.width() as f32;
        let h = frame.height() as f32;

        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // ── Update ────────────────────────────────────────────────────────────

        uv_pos = uv_pos + uv_vel * dt;
        uv_rot -= dt * 0.6;

        if uv_pos.x < 0.0 || uv_pos.x + uv_w > w {
            uv_vel.x = -uv_vel.x;
            uv_pos.x = uv_pos.x.clamp(0.0, w - uv_w);
        }
        if uv_pos.y < 0.0 || uv_pos.y + uv_h > h {
            uv_vel.y = -uv_vel.y;
            uv_pos.y = uv_pos.y.clamp(0.0, h - uv_h);
        }

        ch_pos = ch_pos + ch_vel * dt;
        ch_rot += dt * 1.5;

        if ch_pos.x < 0.0 || ch_pos.x + CHECKER_SIZE > w {
            ch_vel.x = -ch_vel.x;
            ch_pos.x = ch_pos.x.clamp(0.0, w - CHECKER_SIZE);
        }
        if ch_pos.y < 0.0 || ch_pos.y + CHECKER_SIZE > h {
            ch_vel.y = -ch_vel.y;
            ch_pos.y = ch_pos.y.clamp(0.0, h - CHECKER_SIZE);
        }

        // ── Draw ──────────────────────────────────────────────────────────────

        frame.clear(Colour::DARKBLUE);

        // UV texture — loaded from a PNG file, rotated around its centre.
        frame.draw_texture_ex(
            &uv,
            &DrawParams {
                dest_rect: Rect::new(uv_pos.x, uv_pos.y, uv_w, uv_h),
                rotation: uv_rot,
                origin: Vec2::new(uv_w * 0.5, uv_h * 0.5),
                tint: Colour::WHITE,
                ..Default::default()
            },
        );

        // Checkerboard — built from raw pixels via Texture2D::from_pixels.
        frame.draw_texture_ex(
            &checker,
            &DrawParams {
                dest_rect: Rect::new(ch_pos.x, ch_pos.y, CHECKER_SIZE, CHECKER_SIZE),
                rotation: ch_rot,
                origin: Vec2::new(CHECKER_SIZE * 0.5, CHECKER_SIZE * 0.5),
                tint: Colour::WHITE,
                ..Default::default()
            },
        );

        // Shapes.
        frame.draw_rect(Rect::new(10.0, 10.0, 120.0, 60.0), Colour::RED);
        frame.draw_rect_lines(Rect::new(10.0, 80.0, 120.0, 60.0), 3.0, Colour::GREEN);
        frame.draw_circle(Vec2::new(w * 0.5, h * 0.5), 45.0, Colour::YELLOW);
        frame.draw_circle_lines(Vec2::new(w * 0.5, h * 0.5), 62.0, 3.0, Colour::CYAN);

        // Triangle — counter-clockwise, pointing up.
        frame.draw_triangle(
            Vec2::new(w - 60.0, h - 120.0),
            Vec2::new(w - 110.0, h - 40.0),
            Vec2::new(w - 10.0, h - 40.0),
            Colour::GREEN,
        );
        frame.draw_line(Vec2::new(0.0, h), Vec2::new(w, 0.0), 2.0, Colour::CYAN);

        // Rotated rectangle — pivots around its centre.
        let rr_w = 100.0f32;
        let rr_h = 50.0f32;
        frame.draw_rect_ex(
            Rect::new(w - 160.0, 20.0, rr_w, rr_h),
            Vec2::new(rr_w * 0.5, rr_h * 0.5),
            uv_rot * 2.0, // reuse the uv sprite's angle for variety
            Colour::ORANGE,
        );

        // ── Blend mode demonstration ───────────────────────────────────────────
        //
        // Three panels, each showing two overlapping semi-transparent circles.
        // Only the blend mode differs; the geometry and colours are identical.
        //
        //  Alpha     — standard compositing: overlap blends red and blue.
        //  Additive  — overlap adds to the background; circles brighten where
        //              they meet and against the grey panel.
        //  Multiplied — overlap multiplies against the background; circles
        //              darken the panel and tint each other's region.
        const PANEL_W: f32 = 240.0;
        const PANEL_H: f32 = 140.0;
        const PANEL_GAP: f32 = 20.0;
        const PANEL_Y: f32 = 440.0;

        let panels = [
            (BlendMode::Alpha, PANEL_GAP),
            (BlendMode::Additive, PANEL_GAP * 2.0 + PANEL_W),
            (BlendMode::Multiplied, PANEL_GAP * 3.0 + PANEL_W * 2.0),
        ];

        for (mode, panel_x) in panels {
            // Draw the panel background in Alpha mode (always).
            frame.draw_rect(
                Rect::new(panel_x, PANEL_Y, PANEL_W, PANEL_H),
                Colour::new(0.35, 0.35, 0.35, 1.0),
            );

            let cx = panel_x + PANEL_W * 0.5;
            let cy = PANEL_Y + PANEL_H * 0.5;

            frame.set_blend_mode(mode);
            frame.draw_circle(
                Vec2::new(cx - 22.0, cy),
                38.0,
                Colour::new(1.0, 0.2, 0.2, 0.8),
            );
            frame.draw_circle(
                Vec2::new(cx + 22.0, cy),
                38.0,
                Colour::new(0.2, 0.4, 1.0, 0.8),
            );
            frame.set_blend_mode(BlendMode::Alpha);
        }
    }

    Ok(())
}
