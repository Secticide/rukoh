# rukoh

Raylib-inspired 2D game/multimedia library for Windows, written in idiomatic Rust.

## Stack

- **Graphics:** Direct3D 11 via `windows-rs`
- **Windowing:** Raw Win32 via `windows-rs` (no winit)
- **Audio:** XAudio2 via `windows-rs` + `lewton` (OGG decoding)
- **Font rendering:** `fontdue`
- **Image loading:** `image` crate
- **Platform:** Windows only (for now)

## Project layout

```
src/
  lib.rs              <- Rukoh entry point, RukohConfig, WindowMode, re-exports
  error.rs            <- rukoh::Error (Windows, Image, Font, Audio, InvalidState)
  frame.rs            <- Frame<'_> (RAII, drop = flush + present)
  gfx.rs              <- GfxDevice: D3D11 device + IDXGISwapChain
  maths.rs            <- Vec2, Rect
  colour.rs           <- Colour { r, g, b, a: f32 } + named constants
  window.rs           <- Win32Window, WindowState, message pump
  graphics/
    mod.rs
    renderer.rs       <- SpriteBatch (batched quad renderer), DrawParams, BlendMode
    texture.rs        <- Texture2D (RAII, load from &[u8] or raw RGBA), TextureFilter
    camera.rs         <- Camera2D (position, zoom, rotation)
    render_target.rs  <- RenderTarget (Deref<Target=Texture2D>)
    text.rs           <- Font (fontdue glyph atlas)
  input/
    mod.rs            <- InputState, MouseButton
    keyboard.rs       <- KeyCode (VK_ mapped enum)
    gamepad.rs        <- XInput wrapper, GamepadState, GamepadBackend
    hid/
      mod.rs          <- HidManager, Raw Input / HID gamepad fallback
      profiles.rs     <- per-device button mapping profiles (PS4, Switch Pro, generic)
  shaders/            <- sprite.hlsl + compiled sprite_vs.dxbc / sprite_ps.dxbc
  audio/
    mod.rs            <- AudioDevice, SoundParams, decode_ogg helper
    sound.rs          <- Sound (fully-decoded OGG, shared-pool playback)
    music.rs          <- Music (fully-decoded OGG, single dedicated voice)
examples/
  hello_window.rs     <- Phase 1: window, game loop, delta time
  input.rs            <- Phase 2: keyboard, mouse, gamepad — on-screen display
  sprites.rs          <- Phase 3: texture loading, sprite drawing, shapes, blend modes
  camera.rs           <- Phase 4: camera transforms, render targets, text
  audio.rs            <- Phase 5: background music, sound effects
  gamepad.rs          <- visual controller layout (all buttons, axes, backend label)
  breakout.rs         <- full Breakout game (music + sound effects)
  bunnymark.rs        <- rendering stress test (batch_size: 65_536)
  assets/             <- lexend.ttf, music.ogg, impact.ogg, bunny.png, uv-texture.png
planning/
  sdf-shapes.md       <- future architecture notes: SDF shape rendering
```

## API conventions

- Single `Rukoh` entry point — constructed via `Rukoh::new(RukohConfig)`.
- Game loop: `while let Some(mut frame) = app.next_frame() { ... }`
- `Frame` drop = flush SpriteBatch + `IDXGISwapChain::Present`. No explicit end_frame().
- Input and audio on `Frame`: `frame.is_key_pressed(...)`, `frame.play_sound(...)`.
- All assets loaded from `&[u8]` (use `include_bytes!`). No runtime file I/O in library code.
- All resource types (`Texture2D`, `Font`, `Sound`, `Music`) are RAII.
- `Result<T, rukoh::Error>` for fallible operations. No panics in library code.
- All `unsafe` is private — never in the public API surface.
- D3D11 / Win32 / XAudio2 FFI stays inside crate internals.
- **UK English spelling throughout:** `Colour` not `Color`.
- Constructor params: prefer adding customisation points as named parameters. If a constructor
  accumulates more than ~3 parameters, propose migrating to a config/builder struct instead.

## Key types (current)

| Type | File | Notes |
|---|---|---|
| `Rukoh` | `src/lib.rs` | Owns window, D3D11 device, SpriteBatch, AudioDevice |
| `RukohConfig` | `src/lib.rs` | See config section below |
| `Frame<'_>` | `src/frame.rs` | Borrows `&mut Rukoh`. Drop = flush + present |
| `Texture2D` | `src/graphics/texture.rs` | `load(&app, &[u8], filter)`, `from_pixels(&app, &[u8], w, h, filter)` |
| `TextureFilter` | `src/graphics/texture.rs` | `Point` (default, nearest-neighbour) or `Bilinear` (smooth) |
| `DrawParams` | `src/graphics/renderer.rs` | dest_rect, source_rect, rotation, origin, tint |
| `BlendMode` | `src/graphics/renderer.rs` | `Alpha` (default), `Additive`, `Multiplied` |
| `Camera2D` | `src/graphics/camera.rs` | position (top-left world), zoom, rotation; `centred()`, coord helpers |
| `RenderTarget` | `src/graphics/render_target.rs` | `Deref<Target=Texture2D>`; `begin/end_texture_mode` |
| `Font` | `src/graphics/text.rs` | fontdue atlas; `load()` (ASCII) or `load_chars()`; `measure_text()` |
| `Sound` | `src/audio/sound.rs` | OGG decoded at load; played via shared voice pool |
| `Music` | `src/audio/music.rs` | OGG decoded at load; dedicated looping voice |
| `SoundParams` | `src/audio/mod.rs` | `{ volume: f32, pitch: f32 }`, Copy, Default (1.0, 1.0) |
| `GamepadState` | `src/input/gamepad.rs` | Per-frame snapshot; `is_button_down/pressed/released`, `last_button_pressed`, `backend` |
| `GamepadBackend` | `src/input/gamepad.rs` | `XInput` or `Hid` — which driver read this controller |
| `KeyCode` | `src/input/keyboard.rs` | VK_-mapped enum; `from_vk` reverse-maps for `last_key_pressed` |
| `Colour` | `src/colour.rs` | `{ r, g, b, a: f32 }`, `Copy`, named constants |
| `Vec2` | `src/maths.rs` | `{ x, y: f32 }`, ops, constants |
| `Rect` | `src/maths.rs` | `{ x, y, w, h: f32 }` |

## RukohConfig

```rust
RukohConfig {
    title: "My Game",           // &'static str

    // Window mode — determines the OS window size.
    window_mode: WindowMode::Windowed { width: 800, height: 600 },
    // WindowMode::BorderlessWindowed  ← covers the full monitor, no chrome

    // Back-buffer resolution. None (default) = match window client area.
    // Set explicitly for low-resolution rendering scaled up to the window,
    // e.g. render_size: Some((320, 240)) inside an 800×600 window.
    render_size: None,

    vsync: true,                // sync Present to monitor refresh
    sound_voices: 32,           // shared SFX voice pool size
    batch_size: 2048,           // max quads per DrawIndexed call
                                // increase for high-sprite workloads (bunnymark uses 65_536)
}
```

`render_size` and `window_mode` are independent: the window can be any size regardless of
the render resolution. The render resolution is what `frame.width()` / `frame.height()` return.

## Drawing API

```rust
// Textures
frame.draw_texture(&tex, pos, tint);
frame.draw_texture_ex(&tex, &DrawParams { dest_rect, rotation, origin, tint, .. });

// Shapes — filled
frame.draw_rect(rect, colour);
frame.draw_rect_ex(rect, origin, rotation, colour);   // rotated rect
frame.draw_rect_rounded(rect, radius, colour);         // rounded corners
frame.draw_circle(centre, radius, colour);
frame.draw_triangle(v1, v2, v3, colour);               // counter-clockwise

// Shapes — outlines
frame.draw_rect_lines(rect, thickness, colour);
frame.draw_circle_lines(centre, radius, thickness, colour);
frame.draw_line(start, end, thickness, colour);

// Text
frame.draw_text(&font, text, pos, colour);
let size: Vec2 = font.measure_text(text);  // x = pixel width, y = line height

// Blend modes (persist until changed; restore Alpha when done)
frame.set_blend_mode(BlendMode::Additive);
frame.set_blend_mode(BlendMode::Alpha);    // restore
```

## Input API

```rust
// Keyboard
frame.is_key_down(KeyCode::W)
frame.is_key_pressed(KeyCode::Space)
frame.is_key_released(KeyCode::Escape)
frame.last_key_pressed() -> Option<KeyCode>   // first key with rising edge this frame

// Mouse
frame.mouse_pos() -> Vec2          // render-space pixels
frame.mouse_delta() -> Vec2
frame.mouse_scroll() -> f32        // positive = scroll up
frame.is_mouse_down(MouseButton::Left)
frame.is_mouse_pressed(MouseButton::Right)
frame.is_mouse_released(MouseButton::Middle)

// Cursor
frame.show_cursor()
frame.hide_cursor()

// Gamepad (XInput primary, HID fallback)
if let Some(gp) = frame.gamepad() {
    gp.left_stick() -> Vec2         // radial dead zone applied
    gp.right_stick() -> Vec2
    gp.left_trigger() -> f32        // [0, 1]
    gp.right_trigger() -> f32
    gp.is_button_down(GamepadButton::South)
    gp.is_button_pressed(GamepadButton::Start)
    gp.last_button_pressed() -> Option<GamepadButton>
    gp.backend() -> GamepadBackend  // XInput or Hid
}
```

## Audio API

```rust
// Resources (before loop) — OGG only
let snd   = Sound::load(&app, bytes)?;
let music = Music::load(&app, bytes)?;

// In loop (all on Frame)
frame.play_sound(&snd, SoundParams { volume: 0.8, pitch: 1.0 });
frame.play_sound(&snd, SoundParams::default());  // volume 1.0, pitch 1.0

frame.play_music(&music);          // start / restart (loops indefinitely)
frame.pause_music();               // freeze position
frame.resume_music();              // continue from pause point
frame.stop_music();                // stop + reset to start
frame.set_music_volume(0.8);       // 0.0–1.0
```

**Design notes:**
- Sounds play concurrently from a shared `IXAudio2SourceVoice` pool (default 32 voices,
  configurable via `RukohConfig::sound_voices`). Requests are silently dropped when the pool is full.
- Music plays on a separate dedicated voice, not counted against the pool.
- Both `Sound` and `Music` store fully-decoded PCM in an `Arc<[u8]>` (single fat-pointer
  allocation). The data is kept alive by the pool slot for as long as XAudio2 may be reading it.
- XAudio2 constants that windows-rs does not expose directly are defined locally as
  `const` literals (`END_OF_STREAM = 0x0040`, `LOOP_INFINITE = 255`, `COMMIT_NOW = 0`,
  `DEFAULT_PROCESSOR = 0x0000_0001`).

## Camera API

```rust
let mut camera = Camera2D::centred(Vec2::new(400.0, 300.0), 800, 600, 1.0);
frame.set_camera(&camera)?;   // flush + upload view-proj matrix
frame.reset_camera()?;        // flush + restore default ortho
camera.screen_to_world(pos, w, h)
camera.world_to_screen(pos, w, h)
```

## Render target API

```rust
let rt = RenderTarget::new(&app, width, height)?;
frame.begin_texture_mode(&rt)?;  // Err if one already active
frame.clear(Colour::BLACK);       // clears RT, not back buffer
// ... draw into rt ...
frame.end_texture_mode();
frame.draw_texture(&rt, pos, Colour::WHITE); // Deref to Texture2D
```

## Performance rules (apply everywhere)

- Use `c""` literals and `w!()` macros for Win32 strings — never allocate `CString`/`String` for API calls.
- Sprite batch uses a pre-allocated vertex buffer (`D3D11_MAP_WRITE_DISCARD`). No allocations per draw call.
- `Colour`, `Vec2`, `Rect`, `DrawParams`, `SoundParams` are `Copy` structs — always passed by value.
- Index buffer is static (pre-filled u32 indices). One `DrawIndexed` per batch flush.
- Flush only on texture/filter switch or batch full — minimise `PSSetShaderResources` and `PSSetSamplers` calls.
- `push_quad` has a zero-rotation fast path: when `rotation == 0.0`, `sin_cos` and all pivot
  arithmetic are skipped. This covers the vast majority of draw calls (sprites, rects, text glyphs).
- HLSL shaders compiled to DXBC offline (via `build.rs` + `fxc.exe`) and embedded with `include_bytes!`.
- D3D11 flag type mismatches in windows-rs 0.58: use `.0 as u32` casts (e.g. `D3D11_BIND_SHADER_RESOURCE.0 as u32`).
- Sound pool reuses idle voices (same format) before creating new ones — no per-play allocation once warmed up.

## Development workflow

1. **API design first** — ask questions one at a time until the public API is fully agreed.
2. Implement, then write and run the example.
3. `cargo fmt` for consistent formatting.
4. `cargo clippy` must pass (zero warnings) before a phase is done.
5. Wait for review before starting the next phase.

See `PLAN.md` for the full phase breakdown and status.
