use crate::{
    audio::{Music, Sound, SoundParams},
    graphics::{BlendMode, Font, RenderTarget, Texture2D},
    Camera2D, Colour, DrawParams, Error, GamepadState, KeyCode, MouseButton, Rect, Rukoh, Vec2,
};

/// A single rendered frame. Created by [`Rukoh::next_frame`].
///
/// Drawing and input query methods are called on this type. When the `Frame`
/// is dropped it flushes all pending draw calls and presents the back buffer.
pub struct Frame<'a> {
    pub(crate) rukoh: &'a mut Rukoh,
    dt: f32,
}

impl<'a> Frame<'a> {
    pub(crate) fn new(rukoh: &'a mut Rukoh, dt: f32) -> Self {
        Self { rukoh, dt }
    }

    // ── Timing ───────────────────────────────────────────────────────────────

    /// Seconds elapsed since the previous frame. Returns `0.0` on the first frame.
    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.dt
    }

    // ── Dimensions ───────────────────────────────────────────────────────────

    /// The render target width in pixels (independent of window size).
    #[inline]
    pub fn width(&self) -> u32 {
        self.rukoh.render_width
    }

    /// The render target height in pixels (independent of window size).
    #[inline]
    pub fn height(&self) -> u32 {
        self.rukoh.render_height
    }

    // ── Drawing ───────────────────────────────────────────────────────────────

    /// Clear the active render surface to the given colour.
    ///
    /// Clears the off-screen render target if one is active, otherwise clears
    /// the back buffer.
    pub fn clear(&mut self, colour: Colour) {
        match &self.rukoh.current_rtv.clone() {
            Some(rtv) => self.rukoh.gfx.clear_rtv(rtv, colour),
            None => self.rukoh.gfx.clear(colour),
        }
    }

    /// Draw a texture at `pos` at its natural size, multiplied by `tint`.
    pub fn draw_texture(&mut self, texture: &Texture2D, pos: Vec2, tint: Colour) {
        let params = DrawParams {
            dest_rect: Rect::new(pos.x, pos.y, texture.width as f32, texture.height as f32),
            tint,
            ..Default::default()
        };
        self.rukoh
            .batch
            .draw_texture_ex(&texture.srv, texture.width, texture.height, &params);
    }

    /// Draw a texture with full control over destination rect, source rect,
    /// rotation, pivot origin, and tint.
    pub fn draw_texture_ex(&mut self, texture: &Texture2D, params: &DrawParams) {
        self.rukoh
            .batch
            .draw_texture_ex(&texture.srv, texture.width, texture.height, params);
    }

    /// Draw a filled solid-colour rectangle.
    pub fn draw_rect(&mut self, rect: Rect, colour: Colour) {
        self.rukoh.batch.draw_rect(rect, colour);
    }

    /// Draw a filled rectangle with a rotation (in radians) around `origin`.
    ///
    /// `origin` is the pivot point in rect-local pixels relative to the
    /// rect's top-left corner. `Vec2::ZERO` rotates around the top-left;
    /// `Vec2::new(rect.w * 0.5, rect.h * 0.5)` rotates around the centre.
    pub fn draw_rect_ex(&mut self, rect: Rect, origin: Vec2, rotation: f32, colour: Colour) {
        self.rukoh
            .batch
            .draw_rect_ex(rect, origin, rotation, colour);
    }

    /// Draw a hollow rectangle outline.
    pub fn draw_rect_lines(&mut self, rect: Rect, thickness: f32, colour: Colour) {
        self.rukoh.batch.draw_rect_lines(rect, thickness, colour);
    }

    /// Draw a thick line between two points.
    pub fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, colour: Colour) {
        self.rukoh.batch.draw_line(start, end, thickness, colour);
    }

    /// Draw a filled triangle. Vertices should be in counter-clockwise order.
    pub fn draw_triangle(&mut self, v1: Vec2, v2: Vec2, v3: Vec2, colour: Colour) {
        self.rukoh.batch.draw_triangle(v1, v2, v3, colour);
    }

    /// Show the OS cursor. No-op if the cursor is already visible.
    pub fn show_cursor(&mut self) {
        if !self.rukoh.cursor_visible {
            self.rukoh.cursor_visible = true;
            unsafe { windows::Win32::UI::WindowsAndMessaging::ShowCursor(true) };
        }
    }

    /// Hide the OS cursor. No-op if the cursor is already hidden.
    pub fn hide_cursor(&mut self) {
        if self.rukoh.cursor_visible {
            self.rukoh.cursor_visible = false;
            unsafe { windows::Win32::UI::WindowsAndMessaging::ShowCursor(false) };
        }
    }

    /// Draw a filled circle tessellated into 32 triangle segments.
    pub fn draw_circle(&mut self, centre: Vec2, radius: f32, colour: Colour) {
        self.rukoh.batch.draw_circle(centre, radius, colour);
    }

    /// Draw a circle outline tessellated into 32 quads.
    pub fn draw_circle_lines(&mut self, centre: Vec2, radius: f32, thickness: f32, colour: Colour) {
        self.rukoh
            .batch
            .draw_circle_lines(centre, radius, thickness, colour);
    }

    /// Switch the blend mode for all subsequent draw calls this frame.
    ///
    /// Flushes any pending draws before switching. The mode persists until
    /// changed again — it does **not** reset automatically at the start of the
    /// next frame, so restore [`BlendMode::Alpha`] when done:
    ///
    /// ```ignore
    /// frame.set_blend_mode(BlendMode::Additive);
    /// // ... draw particles ...
    /// frame.set_blend_mode(BlendMode::Alpha);
    /// ```
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.rukoh.batch.set_blend_mode(mode);
    }

    /// Draw a single line of text. `pos` is the top-left of the text bounding box.
    pub fn draw_text(&mut self, font: &Font, text: &str, pos: Vec2, colour: Colour) {
        font.render(&mut self.rukoh.batch, text, pos, colour);
    }

    // ── Camera ────────────────────────────────────────────────────────────────

    /// Apply a camera transform to all subsequent draw calls this frame.
    ///
    /// Flushes any pending draw calls before switching the projection matrix.
    pub fn set_camera(&mut self, camera: &Camera2D) -> Result<(), Error> {
        let matrix = camera.view_proj_matrix(self.rukoh.render_width, self.rukoh.render_height);
        self.rukoh.batch.set_projection(&matrix)
    }

    /// Reset to the default screen-space projection (no camera).
    ///
    /// Flushes any pending draw calls before restoring the projection matrix.
    pub fn reset_camera(&mut self) -> Result<(), Error> {
        self.rukoh
            .batch
            .reset_projection(self.rukoh.render_width, self.rukoh.render_height)
    }

    // ── Render targets ────────────────────────────────────────────────────────

    /// Start rendering into `target` instead of the back buffer.
    ///
    /// Returns `Err` if a render target is already active — nesting is not
    /// supported. Call [`end_texture_mode`](Self::end_texture_mode) first.
    pub fn begin_texture_mode(&mut self, target: &RenderTarget) -> Result<(), Error> {
        if self.rukoh.current_rtv.is_some() {
            return Err(Error::InvalidState(
                "begin_texture_mode called while a render target is already active",
            ));
        }
        self.rukoh.batch.end_frame(); // flush before switching target
        self.rukoh
            .gfx
            .bind_render_target(&target.rtv, target.width, target.height);
        self.rukoh.current_rtv = Some(target.rtv.clone());
        self.rukoh.batch.begin_frame();
        Ok(())
    }

    /// Stop rendering into the active render target and restore the back buffer.
    ///
    /// Does nothing if no render target is currently active.
    pub fn end_texture_mode(&mut self) {
        if self.rukoh.current_rtv.is_none() {
            return;
        }
        self.rukoh.batch.end_frame(); // flush RT draw calls
        self.rukoh
            .gfx
            .bind_back_buffer(self.rukoh.render_width, self.rukoh.render_height);
        self.rukoh.current_rtv = None;
        self.rukoh.batch.begin_frame();
    }

    // ── Keyboard ─────────────────────────────────────────────────────────────

    /// `true` while the key is physically held.
    #[inline]
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.rukoh.input.keys_current[key.vk()]
    }

    /// `true` on the first frame the key is pressed (rising edge).
    #[inline]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        let vk = key.vk();
        self.rukoh.input.keys_current[vk] && !self.rukoh.input.keys_prev[vk]
    }

    /// `true` on the first frame after the key is released (falling edge).
    #[inline]
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        let vk = key.vk();
        !self.rukoh.input.keys_current[vk] && self.rukoh.input.keys_prev[vk]
    }

    // ── Mouse ─────────────────────────────────────────────────────────────────

    /// Cursor position in render-space pixels.
    #[inline]
    pub fn mouse_pos(&self) -> Vec2 {
        self.rukoh.input.mouse_pos
    }

    /// Cursor movement since the previous frame, in render-space pixels.
    #[inline]
    pub fn mouse_delta(&self) -> Vec2 {
        self.rukoh.input.mouse_delta
    }

    /// Scroll wheel delta this frame. Positive = scroll up.
    #[inline]
    pub fn mouse_scroll(&self) -> f32 {
        self.rukoh.input.mouse_scroll
    }

    /// `true` while the mouse button is held.
    #[inline]
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.rukoh.input.mouse_buttons_curr[button as usize]
    }

    /// `true` on the first frame the button is pressed (rising edge).
    #[inline]
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        let i = button as usize;
        self.rukoh.input.mouse_buttons_curr[i] && !self.rukoh.input.mouse_buttons_prev[i]
    }

    /// `true` on the first frame after the button is released (falling edge).
    #[inline]
    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        let i = button as usize;
        !self.rukoh.input.mouse_buttons_curr[i] && self.rukoh.input.mouse_buttons_prev[i]
    }

    // ── Gamepad ───────────────────────────────────────────────────────────────

    /// Returns the state of the first connected gamepad, or `None` if no
    /// controller is connected.
    #[inline]
    pub fn gamepad(&self) -> Option<GamepadState> {
        self.rukoh.input.gamepad
    }

    // ── Audio ─────────────────────────────────────────────────────────────────

    /// Play `sound` using the given `params`.
    ///
    /// Each call starts an independent concurrent playback from an idle pool
    /// voice. If all voices are busy the request is silently ignored.
    pub fn play_sound(&mut self, sound: &Sound, params: SoundParams) {
        self.rukoh.audio.play_sound(sound, params);
    }

    /// Start playing `music`, looping indefinitely. Replaces any track that is
    /// currently playing.
    pub fn play_music(&mut self, music: &Music) {
        self.rukoh.audio.play_music(music);
    }

    /// Pause music playback. Position is preserved; call [`resume_music`](Self::resume_music)
    /// to continue.
    pub fn pause_music(&mut self) {
        self.rukoh.audio.pause_music();
    }

    /// Resume paused music from where it was paused.
    pub fn resume_music(&mut self) {
        self.rukoh.audio.resume_music();
    }

    /// Stop music and reset position to the beginning.
    pub fn stop_music(&mut self) {
        self.rukoh.audio.stop_music();
    }

    /// Set the music volume (0.0 = silent, 1.0 = original level).
    pub fn set_music_volume(&mut self, volume: f32) {
        self.rukoh.audio.set_music_volume(volume);
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        self.rukoh.batch.end_frame();
        let sync_interval = u32::from(self.rukoh.vsync);
        self.rukoh.gfx.present(sync_interval);
    }
}
