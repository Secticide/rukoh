# rukoh

A Raylib-inspired 2D game and multimedia library for Windows, written in idiomatic Rust.

rukoh gives you a window, a game loop, 2D rendering, input, and audio — all through a clean,
allocation-free-per-frame API backed by Direct3D 11 and XAudio2. There is no global state;
everything flows through a single `Rukoh` instance and the `Frame` handle it produces each tick.

## Platform

Windows only. Requires the Windows SDK (for `fxc.exe`, used by `build.rs` to compile HLSL shaders).

## Getting started

Add rukoh as a git dependency:

```toml
[dependencies]
rukoh = { git = "https://github.com/Secticide/rukoh.git" }
```

### Opening a window

```rust
use rukoh::{Colour, KeyCode, Rukoh, RukohConfig, WindowMode};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "My Game",
        window_mode: WindowMode::Windowed { width: 800, height: 600 },
        ..Default::default()
    })?;

    while let Some(mut frame) = app.next_frame() {
        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        frame.clear(Colour::DARKBLUE);
    }

    Ok(())
}
```

`next_frame` pumps the Win32 message queue and returns `None` when the window is closed.
Dropping a `Frame` flushes all pending draw calls and presents the back buffer.

The back-buffer resolution defaults to the window client area. Override it with
`render_size: Some((320, 240))` for low-resolution rendering scaled up to the window.
Use `WindowMode::BorderlessWindowed` for a fullscreen window at the monitor's native resolution.

### Drawing

All drawing happens on `Frame`. Textures are loaded from byte slices — use `include_bytes!`
to embed assets at compile time.

```rust
use rukoh::{Colour, DrawParams, Font, Rect, Rukoh, RukohConfig, Texture2D, TextureFilter, Vec2};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "Drawing",
        ..Default::default()
    })?;

    // TextureFilter::Point = nearest-neighbour (default, good for pixel art).
    // TextureFilter::Bilinear = smooth interpolation (good for UI textures).
    let tex  = Texture2D::load(&app, include_bytes!("assets/player.png"), TextureFilter::Point)?;
    let font = Font::load(&app, include_bytes!("assets/font.ttf"), 24.0)?;

    while let Some(mut frame) = app.next_frame() {
        frame.clear(Colour::BLACK);

        // Texture at natural size.
        frame.draw_texture(&tex, Vec2::new(100.0, 200.0), Colour::WHITE);

        // Texture with rotation and a custom destination rect.
        frame.draw_texture_ex(&tex, &DrawParams {
            dest_rect: Rect::new(300.0, 200.0, 64.0, 64.0),
            rotation: 0.5,
            tint: Colour::new(1.0, 0.8, 0.8, 1.0),
            ..Default::default()
        });

        // Shapes — filled.
        frame.draw_rect(Rect::new(10.0, 10.0, 80.0, 40.0), Colour::RED);
        frame.draw_rect_rounded(Rect::new(10.0, 60.0, 80.0, 40.0), 8.0, Colour::ORANGE);
        frame.draw_rect_ex(Rect::new(200.0, 100.0, 80.0, 40.0), Vec2::new(40.0, 20.0), 0.5, Colour::PURPLE);
        frame.draw_circle(Vec2::new(400.0, 300.0), 50.0, Colour::CYAN);
        frame.draw_triangle(Vec2::new(500.0, 200.0), Vec2::new(450.0, 280.0), Vec2::new(550.0, 280.0), Colour::YELLOW);

        // Shapes — outlines.
        frame.draw_rect_lines(Rect::new(10.0, 110.0, 80.0, 40.0), 2.0, Colour::GREEN);
        frame.draw_circle_lines(Vec2::new(400.0, 300.0), 60.0, 2.0, Colour::WHITE);
        frame.draw_line(Vec2::ZERO, Vec2::new(200.0, 200.0), 2.0, Colour::WHITE);

        // Text — measure before drawing for layout.
        let label = "Hello, rukoh!";
        let size = font.measure_text(label); // Vec2 { x: pixel width, y: line height }
        frame.draw_text(&font, label, Vec2::new(10.0, 10.0), Colour::WHITE);
        let _ = size;
    }

    Ok(())
}
```

### Blend modes

Switch the blend mode for particle effects, lighting, or colour overlays. The mode persists
until changed, so restore `Alpha` when done.

```rust
use rukoh::BlendMode;

// Additive — pixels are added to the destination (good for glows and particles).
frame.set_blend_mode(BlendMode::Additive);
frame.draw_circle(light_pos, 80.0, Colour::new(1.0, 0.9, 0.4, 0.6));
frame.set_blend_mode(BlendMode::Alpha); // restore

// Multiplied — pixels multiply the destination (good for shadows and colour filters).
frame.set_blend_mode(BlendMode::Multiplied);
frame.draw_rect(shadow_rect, Colour::new(0.0, 0.0, 0.0, 0.5));
frame.set_blend_mode(BlendMode::Alpha);
```

### Input

Keyboard, mouse, and gamepad state are snapshotted once per frame and queried on `Frame`.

```rust
while let Some(mut frame) = app.next_frame() {
    let dt = frame.delta_time(); // seconds since last frame

    // Keyboard — down (held), pressed (rising edge), released (falling edge).
    if frame.is_key_down(KeyCode::Left)    { /* move */ }
    if frame.is_key_pressed(KeyCode::Space) { /* jump */ }

    // First key pressed this frame — useful for rebind UIs and "press any key" prompts.
    if let Some(key) = frame.last_key_pressed() {
        println!("pressed: {key:?}");
    }

    // Cursor visibility.
    frame.hide_cursor(); // hide until shown again
    frame.show_cursor();

    // Mouse.
    let pos    = frame.mouse_pos();    // Vec2, in render-space pixels
    let delta  = frame.mouse_delta();
    let scroll = frame.mouse_scroll(); // positive = up

    if frame.is_mouse_pressed(MouseButton::Left) { /* click */ }

    // Gamepad (XInput primary, HID fallback for non-XInput controllers).
    if let Some(gp) = frame.gamepad() {
        let stick = gp.left_stick();   // Vec2 in [-1, 1], radial dead zone applied
        let lt    = gp.left_trigger(); // f32 in [0, 1]

        if gp.is_button_pressed(GamepadButton::South) { /* A / Cross */ }

        // First button pressed this frame — useful for rebind UIs.
        if let Some(btn) = gp.last_button_pressed() {
            println!("pressed: {btn:?}");
        }

        // Which driver read this controller.
        println!("backend: {:?}", gp.backend()); // XInput or Hid
    }
}
```

### Camera

`Camera2D` applies position, zoom, and rotation to all subsequent draw calls. Call
`set_camera` before drawing world-space content and `reset_camera` before drawing UI.

```rust
let mut camera = Camera2D::centred(Vec2::new(400.0, 300.0), 800, 600, 1.0);

while let Some(mut frame) = app.next_frame() {
    frame.set_camera(&camera)?;
    frame.draw_texture(&world_tex, player_pos, Colour::WHITE);

    frame.reset_camera()?;
    frame.draw_text(&font, "Score: 0", Vec2::new(8.0, 8.0), Colour::WHITE);
}
```

### Render targets

Render to an off-screen texture and then draw the result to the screen.

```rust
let rt = RenderTarget::new(&app, 400, 300)?;

while let Some(mut frame) = app.next_frame() {
    frame.begin_texture_mode(&rt)?;
    frame.clear(Colour::DARK_GREY);
    frame.draw_texture(&scene_tex, Vec2::ZERO, Colour::WHITE);
    frame.end_texture_mode();

    frame.clear(Colour::BLACK);
    frame.draw_texture(&rt, Vec2::new(200.0, 150.0), Colour::WHITE); // RenderTarget derefs to Texture2D
}
```

### Audio

Sounds are played from a shared voice pool (default 32 voices). Music plays on a separate
dedicated voice and loops indefinitely. Both `Sound` and `Music` are loaded from OGG Vorbis.

```rust
let snd   = Sound::load(&app, include_bytes!("assets/jump.ogg"))?;
let music = Music::load(&app, include_bytes!("assets/theme.ogg"))?;

let mut music_started = false;

while let Some(mut frame) = app.next_frame() {
    if !music_started {
        frame.play_music(&music);
        music_started = true;
    }

    if frame.is_key_pressed(KeyCode::Space) {
        frame.play_sound(&snd, SoundParams { volume: 0.8, pitch: 1.0 });
    }
}
```

## Examples

Each example can be run with `cargo run --example <name>`.

| Example        | What it demonstrates                                        |
|----------------|-------------------------------------------------------------|
| `hello_window` | Window, game loop, delta time                               |
| `input`        | Keyboard, mouse, and gamepad state — on-screen display      |
| `sprites`      | Texture loading, sprite drawing, shapes, blend modes        |
| `camera`       | Camera transforms, render targets, text rendering           |
| `audio`        | Background music, sound effects, volume and pitch control   |
| `gamepad`      | Visual controller layout showing all buttons and axes live  |
| `breakout`     | Full Breakout game (music, sound effects, gamepad support)  |
| `bunnymark`    | Rendering stress test — hold LMB to spawn bunnies           |

All examples load assets from `examples/assets/` using `include_bytes!`.

## Design notes

- **Zero per-frame allocations.** The sprite batch uses a pre-allocated vertex buffer
  (`D3D11_MAP_WRITE_DISCARD`). Draw calls accumulate in a CPU-side staging buffer and are
  flushed as a single `DrawIndexed` call per texture change. Batch size is configurable via
  `RukohConfig::batch_size` (default 2 048; set higher for particle systems or stress tests).
- **No global state.** Everything is owned by `Rukoh` and borrowed through `Frame`.
- **No unsafe in the public API.** All FFI is behind private module boundaries.
- **Gamepad support.** XInput is the primary path for Xbox-compatible controllers. A Raw
  Input / HID fallback handles non-XInput controllers (DualSense, Switch Pro, etc.) with
  per-device button mapping profiles.

## Dependencies

| Crate      | Purpose                                  |
|------------|------------------------------------------|
| `windows`  | Win32, Direct3D 11, XAudio2, XInput      |
| `bytemuck` | Zero-copy GPU buffer casting             |
| `fontdue`  | Font rasterization and glyph atlas       |
| `image`    | PNG, JPEG, BMP, TGA decoding             |
| `lewton`   | OGG Vorbis decoding                      |
