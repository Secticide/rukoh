# rukoh — Raylib-inspired 2D Game Library in Rust

## Phase Status

- [x] Phase 1 — Window & Main Loop
- [x] Phase 2 — Input
- [x] Phase 3 — 2D Core Rendering
- [x] Phase 4 — 2D Advanced Rendering
- [x] Phase 5 — Audio

---

## Context

Build `rukoh`, an idiomatic Rust game/multimedia library inspired by Raylib. Windows only (initially). Direct3D 11 for rendering, raw Win32 via `windows-rs` for windowing, XAudio2 for audio, `fontdue` for font rasterization, `image` crate for texture loading.

**Phase workflow (mandatory):**
1. **API design first** — ask questions one at a time until the public API is fully agreed.
2. **Implement** — write the code.
3. **Example** — write and verify the phase example.
4. **`cargo fmt`** — format all code.
5. **`cargo clippy`** — zero warnings required.
6. **Review** — wait for user sign-off before starting the next phase.

---

## Agreed API

```rust
fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "My Game",
        window_mode: WindowMode::Windowed { width: 800, height: 600 },
        render_width: 800,
        render_height: 600,
        vsync: true,
    })?;

    let tex   = Texture2D::load(&app, include_bytes!("../assets/player.png"))?;
    let font  = Font::load(&app, include_bytes!("../assets/font.ttf"), 24.0)?;
    let snd   = Sound::load(&app, include_bytes!("../assets/jump.wav"))?;
    let music = Music::load(&app, include_bytes!("../assets/theme.ogg"))?;

    let mut camera = Camera2D::centred(Vec2::new(400.0, 300.0), 800, 600, 1.0);
    let rt = RenderTarget::new(&app, 400, 300)?;

    while let Some(mut frame) = app.next_frame() {
        let dt = frame.delta_time();

        if frame.is_key_pressed(KeyCode::Escape) { break; }
        if frame.is_key_down(KeyCode::Left) { /* ... */ }
        let mouse = frame.mouse_pos();
        if frame.is_mouse_pressed(MouseButton::Left) { /* ... */ }
        if frame.is_mouse_down(MouseButton::Right) { /* drag */ }

        // Render to texture
        frame.begin_texture_mode(&rt)?;
        frame.clear(Colour::DARK_GREY);
        frame.set_camera(&camera)?;
        frame.draw_texture(&tex, Vec2::new(100.0, 200.0), Colour::WHITE);
        frame.reset_camera()?;
        frame.end_texture_mode();

        // Compose
        frame.clear(Colour::DARKBLUE);
        frame.draw_texture(&rt, Vec2::ZERO, Colour::WHITE); // Deref<Target=Texture2D>
        frame.draw_texture_ex(&tex, &DrawParams { dest_rect, rotation, origin, tint, .. Default::default() });
        frame.draw_rect(Rect::new(10.0, 10.0, 50.0, 50.0), Colour::RED);
        frame.draw_circle(Vec2::new(400.0, 300.0), 50.0, Colour::GREEN);
        frame.draw_line(Vec2::ZERO, Vec2::new(100.0, 100.0), 2.0, Colour::WHITE);
        frame.draw_text(&font, "Hello!", Vec2::new(10.0, 10.0), Colour::WHITE);

        snd.play();
        music.set_volume(0.8);
    }
    Ok(())
}
```

---

## Crate Dependencies

```toml
[dependencies]
bytemuck  = { version = "1", features = ["derive"] }
fontdue   = "0.9"
image     = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp", "tga"] }
windows   = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Direct3D11",
    "Win32_Graphics_Direct3D",
    "Win32_Graphics_Dxgi",
    "Win32_Graphics_Dxgi_Common",
    "Win32_Graphics_Gdi",
    "Win32_UI_Input_XboxController",
    "Win32_System_LibraryLoader",
    "Win32_System_Performance",
    # Phase 5 additions:
    "Win32_Media_Audio",
    "Win32_Media_Audio_XAudio2",
    "Win32_System_Com",
]}
# Phase 5:
# hound  = "3"
# lewton = "0.10"

[build-dependencies]
glob = "0.3"
```

---

## Phase 1 — Window & Main Loop ✅

**Goal:** Window opens, frame loop with delta-time, closes cleanly.
**Example:** `cargo run --example hello_window`
**Key types:** `Rukoh`, `RukohConfig`, `WindowMode`, `Frame<'_>`, `Colour`
**Notes:** GWLP_USERDATA pattern for Win32 wndproc; QPC for timing; WM_CLOSE suppressed.

---

## Phase 2 — Input ✅

**Goal:** Keyboard, mouse, and gamepad state queryable inside the frame.
**Example:** `cargo run --example input`
**Key types:** `KeyCode`, `MouseButton`, `GamepadState`, `GamepadButton`
**Notes:** Per-frame snapshot (current/prev) for edge detection; XInput radial dead zone 0.24; mouse mapped from client coords to render-space.

---

## Phase 3 — 2D Core Rendering ✅

**Goal:** Draw textured sprites and geometric shapes on screen.
**Example:** `cargo run --example sprites`
**Key types:** `Texture2D`, `DrawParams`, `SpriteBatch`
**Notes:**
- `SpriteBatch`: 2048-quad pre-allocated vertex buffer, static index buffer, flush on texture change.
- Shapes via 1×1 white texture trick. Circles via separate triangle-list draw (flush first, unbind index buffer).
- `Texture2D::from_pixels` for procedural textures.
- `build.rs` compiles `sprite.hlsl` → `sprite_vs.dxbc` / `sprite_ps.dxbc` via `fxc.exe`.
- windows-rs 0.58: D3D11 flag values need `.0 as u32` casts; shader creation takes `&[u8]` slices directly.

---

## Phase 4 — 2D Advanced Rendering ✅

**Goal:** Camera transforms, off-screen render targets, text rendering.
**Example:** `cargo run --example camera`
**Key types:** `Camera2D`, `RenderTarget`, `Font`
**Notes:**
- `Camera2D`: position (world-space top-left) + zoom + rotation. `centred()` constructor. `screen_to_world` / `world_to_screen` helpers. Row-major combined view-proj matrix uploaded to constant buffer.
- `RenderTarget`: `Deref<Target=Texture2D>`. `begin_texture_mode` returns `Err` if one already active. `frame.clear()` routes to the correct RTV (tracks `current_rtv: Option<ID3D11RenderTargetView>` in `Rukoh`).
- `Font`: fontdue glyph atlas (1024×1024 RGBA, shelf packing). `load()` = printable ASCII; `load_chars()` = custom set. `draw_text` is single-line.
- Camera example loads system font at runtime (examples may use file I/O; library may not).

---

## Phase 5 — Audio ✅

**Goal:** Load and play sound effects; play background music with pause/resume/stop.
**Example:** `cargo run --example audio`
**Expect:** Background music loops; Space plays a sound effect; M/P/R toggle music.

### Agreed API

```rust
// Resources (before loop) — OGG only (lewton)
let snd   = Sound::load(&app, bytes)?;   // fully decoded to PCM
let music = Music::load(&app, bytes)?;   // fully decoded to PCM

// In loop (all on Frame, consistent with draw API)
frame.play_sound(&snd, SoundParams { volume: 1.0, pitch: 1.0 });
frame.play_sound(&snd, SoundParams::default());

frame.play_music(&music);      // start / restart, loops indefinitely
frame.pause_music();           // freeze position
frame.resume_music();          // continue from pause
frame.stop_music();            // stop + reset to start
frame.set_music_volume(0.8);

// RukohConfig
sound_voices: u32  // default 32; shared pool for SFX (music is separate)
```

### Design notes
- OGG only (`lewton`); no WAV/hound.
- `Sound` and `Music` are passive data handles (like `Texture2D`). No methods.
- `SoundParams { volume: f32, pitch: f32 }` — `Copy`, `Default` (1.0, 1.0).
- Shared pool of up to `sound_voices` `IXAudio2SourceVoice` instances. Idle slots
  (same format) are reused; full pool silently drops the request.
- Music plays on its own dedicated voice, not counted against the pool.
- Both load fully to memory (no streaming thread). Looping via `XAUDIO2_LOOP_INFINITE`.
- `CoInitializeEx(COINIT_MULTITHREADED)` called at `AudioDevice::new()`; `RPC_E_CHANGED_MODE`
  ignored (COM may already be initialised).

### Tasks
1. ✅ `src/audio/mod.rs` — `AudioDevice` + `SoundParams` + `decode_ogg` helper.
2. ✅ `src/audio/sound.rs` — `Sound`.
3. ✅ `src/audio/music.rs` — `Music`.
4. ✅ Wire `AudioDevice` into `Rukoh`, add `sound_voices` to `RukohConfig`.
5. ✅ Audio methods on `Frame`.
6. ✅ `examples/audio.rs`.
7. ✅ `cargo fmt` + `cargo clippy` clean + user review.
