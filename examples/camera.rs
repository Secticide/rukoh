use rukoh::{
    graphics::Texture2D, Camera2D, Colour, DrawParams, Font, KeyCode, MouseButton, Rect,
    RenderTarget, Rukoh, RukohConfig, TextureFilter, Vec2, WindowMode,
};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — camera & text",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        ..Default::default()
    })?;

    // ── Resources ─────────────────────────────────────────────────────────────

    // Build a small checkerboard world texture.
    const TEX: u32 = 32;
    const CELL: u32 = 8;
    let mut px = vec![0u8; (TEX * TEX * 4) as usize];
    for y in 0..TEX {
        for x in 0..TEX {
            let i = ((y * TEX + x) * 4) as usize;
            let c = if (x / CELL + y / CELL).is_multiple_of(2) {
                200u8
            } else {
                80u8
            };
            px[i..i + 4].copy_from_slice(&[c, c, c, 255]);
        }
    }
    let world_tex = Texture2D::from_pixels(&app, &px, TEX, TEX, TextureFilter::Point)?;

    let font = Font::load(&app, include_bytes!("assets/lexend.ttf"), 20.0)?;

    // Off-screen render target (half the render size).
    let rt = RenderTarget::new(&app, 400, 300)?;

    // ── Camera state ──────────────────────────────────────────────────────────
    let mut camera = Camera2D::centred(Vec2::new(400.0, 300.0), 800, 600, 1.0);

    while let Some(mut frame) = app.next_frame() {
        let dt = frame.delta_time();
        let w = frame.width() as f32;
        let h = frame.height() as f32;

        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // ── Camera pan & zoom ─────────────────────────────────────────────────
        let speed = 200.0 / camera.zoom;
        if frame.is_key_down(KeyCode::Left) {
            camera.position.x -= speed * dt;
        }
        if frame.is_key_down(KeyCode::Right) {
            camera.position.x += speed * dt;
        }
        if frame.is_key_down(KeyCode::Up) {
            camera.position.y -= speed * dt;
        }
        if frame.is_key_down(KeyCode::Down) {
            camera.position.y += speed * dt;
        }

        // Right-click drag to pan.
        if frame.is_mouse_down(MouseButton::Right) {
            let delta = frame.mouse_delta();
            camera.position.x -= delta.x / camera.zoom;
            camera.position.y -= delta.y / camera.zoom;
        }

        let scroll = frame.mouse_scroll();
        if scroll != 0.0 {
            camera.zoom = (camera.zoom * (1.0 + scroll * 0.1)).clamp(0.1, 10.0);
        }

        // ── Render scene into off-screen target ───────────────────────────────
        frame.begin_texture_mode(&rt)?;
        frame.clear(Colour::DARK_GREY);
        frame.set_camera(&camera)?;

        // Draw a grid of world tiles.
        for row in 0..8i32 {
            for col in 0..8i32 {
                frame.draw_texture_ex(
                    &world_tex,
                    &DrawParams {
                        dest_rect: Rect::new(col as f32 * 64.0, row as f32 * 64.0, 64.0, 64.0),
                        tint: Colour::WHITE,
                        ..Default::default()
                    },
                );
            }
        }

        // World-space decorations.
        frame.draw_circle(Vec2::new(256.0, 256.0), 30.0, Colour::RED);
        frame.draw_rect_lines(Rect::new(0.0, 0.0, 512.0, 512.0), 2.0, Colour::GREEN);

        frame.reset_camera()?;
        frame.end_texture_mode();

        // ── Compose final frame ───────────────────────────────────────────────
        frame.clear(Colour::DARKBLUE);

        // Draw the RT (centred, scaled up to fill the screen).
        frame.draw_texture_ex(
            &rt,
            &DrawParams {
                dest_rect: Rect::new(0.0, 0.0, w, h),
                tint: Colour::WHITE,
                ..Default::default()
            },
        );

        // UI overlay — drawn in screen space.
        frame.draw_rect(
            Rect::new(0.0, 0.0, w, 36.0),
            Colour::new(0.0, 0.0, 0.0, 0.6),
        );
        frame.draw_text(
            &font,
            "Arrows / RMB drag: pan  |  Scroll: zoom  |  Esc: quit",
            Vec2::new(8.0, 6.0),
            Colour::WHITE,
        );

        // Show mouse world position. Draw a tight outline box sized by
        // measure_text so you can verify it tracks the string width correctly.
        let mouse_world = camera.screen_to_world(frame.mouse_pos(), frame.width(), frame.height());
        let coord_str = format!("World ({:.0}, {:.0})", mouse_world.x, mouse_world.y);
        let text_pos = Vec2::new(8.0, h - 28.0);
        let text_size = font.measure_text(&coord_str);
        const PAD: f32 = 4.0;
        frame.draw_rect_lines(
            Rect::new(
                text_pos.x - PAD,
                text_pos.y - PAD,
                text_size.x + PAD * 2.0,
                text_size.y + PAD * 2.0,
            ),
            1.0,
            Colour::new(1.0, 1.0, 0.0, 0.6),
        );
        frame.draw_text(&font, &coord_str, text_pos, Colour::LIGHT_GREY);
    }

    Ok(())
}
