// Place OGG files at:
//   examples/assets/music.ogg    — background music (looping)
//   examples/assets/impact.ogg   — sound effect (Space to trigger)
use rukoh::{Colour, Font, KeyCode, Music, Rukoh, RukohConfig, Sound, SoundParams, Vec2};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — audio",
        ..Default::default()
    })?;

    // ── Load audio ────────────────────────────────────────────────────────────

    let music_bytes = std::fs::read("examples/assets/music.ogg")
        .expect("Place a looping OGG file at examples/assets/music.ogg");
    let impact_bytes = std::fs::read("examples/assets/impact.ogg")
        .expect("Place a short OGG file at examples/assets/impact.ogg");

    let music = Music::load(&app, &music_bytes)?;
    let impact = Sound::load(&app, &impact_bytes)?;

    // ── Font for on-screen instructions ──────────────────────────────────────

    let font_bytes: Vec<u8> = if std::path::Path::new("examples/assets/font.ttf").exists() {
        std::fs::read("examples/assets/font.ttf").unwrap()
    } else {
        std::fs::read(r"C:\Windows\Fonts\segoeui.ttf")
            .or_else(|_| std::fs::read(r"C:\Windows\Fonts\arial.ttf"))
            .expect("No font found — place a TTF at examples/assets/font.ttf")
    };
    let font = Font::load(&app, &font_bytes, 20.0)?;

    let mut music_playing = false;
    let mut music_volume = 1.0f32;

    while let Some(mut frame) = app.next_frame() {
        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // ── Controls ──────────────────────────────────────────────────────────

        if frame.is_key_pressed(KeyCode::M) {
            if music_playing {
                frame.stop_music();
                music_playing = false;
            } else {
                frame.play_music(&music);
                music_playing = true;
            }
        }

        if frame.is_key_pressed(KeyCode::P) && music_playing {
            frame.pause_music();
        }

        if frame.is_key_pressed(KeyCode::R) && music_playing {
            frame.resume_music();
        }

        if frame.is_key_pressed(KeyCode::Up) {
            music_volume = (music_volume + 0.1).min(1.0);
            frame.set_music_volume(music_volume);
        }

        if frame.is_key_pressed(KeyCode::Down) {
            music_volume = (music_volume - 0.1).max(0.0);
            frame.set_music_volume(music_volume);
        }

        if frame.is_key_pressed(KeyCode::Space) {
            frame.play_sound(&impact, SoundParams::default());
        }

        // ── Draw ──────────────────────────────────────────────────────────────

        frame.clear(Colour::DARKBLUE);

        let vol_pct = (music_volume * 100.0) as u32;
        let status = if music_playing { "playing" } else { "stopped" };
        let lines = [
            "M — play / stop music",
            "P — pause music",
            "R — resume music",
            "Up / Down — volume",
            "Space — play sound effect",
            "Esc — quit",
            "",
            &format!("Music: {status}   Volume: {vol_pct}%"),
        ];

        for (i, line) in lines.iter().enumerate() {
            frame.draw_text(
                &font,
                line,
                Vec2::new(40.0, 60.0 + i as f32 * 30.0),
                Colour::WHITE,
            );
        }
    }

    Ok(())
}
