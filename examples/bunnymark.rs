//! Bunnymark — rendering stress test.
//!
//! Left-click anywhere to spawn 500 more bunnies at the cursor.
//! Hold left-click to continuously spawn.
//! Press Escape to quit.
//!
//! Place the sprite at: examples/assets/bunny.png
use rukoh::{graphics::Texture2D, Colour, Font, KeyCode, MouseButton, Rukoh, RukohConfig, Vec2};

const GRAVITY: f32 = 1_500.0;
const DAMPING: f32 = 0.85; // velocity retained on floor bounce
const SPAWN_COUNT: usize = 500;

struct Bunny {
    pos: Vec2,
    vel: Vec2,
}

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — bunnymark",
        // Large batch so the entire bunny population is drawn in as few
        // DrawIndexed calls as possible.
        batch_size: 65536,
        ..Default::default()
    })?;

    // ── Assets ────────────────────────────────────────────────────────────────

    let bunny = Texture2D::load(&app, include_bytes!("assets/bunny.png"))?;
    let bw = bunny.width as f32;
    let bh = bunny.height as f32;

    let font = Font::load(&app, include_bytes!("assets/lexend.ttf"), 20.0)?;

    // ── State ─────────────────────────────────────────────────────────────────

    let mut bunnies: Vec<Bunny> = Vec::new();

    // Smoothed FPS display — averaging over a 0.5-second window.
    let mut fps_accum = 0.0f32;
    let mut fps_frames = 0u32;
    let mut fps_display = 0u32;

    while let Some(mut frame) = app.next_frame() {
        let dt = frame.delta_time().min(0.05); // cap dt to prevent tunnelling
        let w = frame.width() as f32;
        let h = frame.height() as f32;

        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // ── Spawn ─────────────────────────────────────────────────────────────

        if frame.is_mouse_down(MouseButton::Left) {
            let origin = frame.mouse_pos();
            for i in 0..SPAWN_COUNT {
                // Fan the initial velocities so bunnies spread on spawn.
                let col = (i % 20) as f32 - 10.0; // -10 … +10
                let row = (i / 20) as f32;
                bunnies.push(Bunny {
                    pos: origin,
                    vel: Vec2::new(col * 60.0, -(row * 40.0 + 150.0)),
                });
            }
        }

        // ── Update ────────────────────────────────────────────────────────────

        for b in &mut bunnies {
            b.vel.y += GRAVITY * dt;
            b.pos = b.pos + b.vel * dt;

            // Left / right walls
            if b.pos.x < 0.0 {
                b.pos.x = 0.0;
                b.vel.x = b.vel.x.abs();
            } else if b.pos.x + bw > w {
                b.pos.x = w - bw;
                b.vel.x = -b.vel.x.abs();
            }

            // Ceiling / floor
            if b.pos.y < 0.0 {
                b.pos.y = 0.0;
                b.vel.y = b.vel.y.abs();
            } else if b.pos.y + bh > h {
                b.pos.y = h - bh;
                b.vel.y = -b.vel.y.abs() * DAMPING;
            }
        }

        // ── FPS smoothing ─────────────────────────────────────────────────────

        fps_accum += dt;
        fps_frames += 1;
        if fps_accum >= 0.5 {
            fps_display = (fps_frames as f32 / fps_accum).round() as u32;
            fps_accum = 0.0;
            fps_frames = 0;
        }

        // ── Draw ──────────────────────────────────────────────────────────────

        frame.clear(Colour::new(0.13, 0.13, 0.13, 1.0));

        for b in &bunnies {
            frame.draw_texture(&bunny, b.pos, Colour::WHITE);
        }

        // HUD — dark backing strip so text is readable over any background.
        frame.draw_rect(
            rukoh::Rect::new(0.0, 0.0, w, 36.0),
            Colour::new(0.0, 0.0, 0.0, 0.55),
        );
        frame.draw_text(
            &font,
            &format!(
                "Bunnies: {:>6}   FPS: {:>4}   Hold LMB to spawn {} at a time",
                bunnies.len(),
                fps_display,
                SPAWN_COUNT,
            ),
            Vec2::new(8.0, 8.0),
            Colour::WHITE,
        );
    }

    Ok(())
}
