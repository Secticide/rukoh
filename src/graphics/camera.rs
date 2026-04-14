use crate::Vec2;

/// A 2D camera that applies position, zoom, and rotation to all subsequent draw calls.
///
/// `position` is the world-space coordinate of the **top-left corner** of the visible area
/// at zoom 1.0. Use [`Camera2D::centred`] to construct a camera that centres on a world
/// point.
#[derive(Clone, Copy, Debug)]
pub struct Camera2D {
    /// World-space coordinate of the top-left of the visible area (at zoom 1.0).
    pub position: Vec2,
    /// Zoom factor. Values > 1 zoom in; values < 1 zoom out.
    pub zoom: f32,
    /// Rotation in radians, applied around the centre of the screen.
    pub rotation: f32,
}

impl Camera2D {
    /// Create a camera with the given top-left world position and zoom.
    pub fn new(position: Vec2, zoom: f32) -> Self {
        Self {
            position,
            zoom,
            rotation: 0.0,
        }
    }

    /// Create a camera centred on `world_centre`, filling a screen of the given dimensions.
    pub fn centred(world_centre: Vec2, screen_w: u32, screen_h: u32, zoom: f32) -> Self {
        let half_w = screen_w as f32 / (2.0 * zoom);
        let half_h = screen_h as f32 / (2.0 * zoom);
        Self {
            position: Vec2::new(world_centre.x - half_w, world_centre.y - half_h),
            zoom,
            rotation: 0.0,
        }
    }

    /// Convert a screen-space position to world-space.
    pub fn screen_to_world(&self, screen_pos: Vec2, screen_w: u32, screen_h: u32) -> Vec2 {
        let cx = screen_w as f32 * 0.5;
        let cy = screen_h as f32 * 0.5;
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        // Translate to screen centre, then apply inverse rotation.
        let dx = screen_pos.x - cx;
        let dy = screen_pos.y - cy;
        let rx = dx * cos + dy * sin;
        let ry = -dx * sin + dy * cos;
        // Scale back and offset by camera position + half-screen world extent.
        Vec2::new(
            rx / self.zoom + self.position.x + cx / self.zoom,
            ry / self.zoom + self.position.y + cy / self.zoom,
        )
    }

    /// Convert a world-space position to screen-space.
    pub fn world_to_screen(&self, world_pos: Vec2, screen_w: u32, screen_h: u32) -> Vec2 {
        let cx = screen_w as f32 * 0.5;
        let cy = screen_h as f32 * 0.5;
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        // Scale and translate relative to the screen centre.
        let dx = (world_pos.x - self.position.x) * self.zoom - cx;
        let dy = (world_pos.y - self.position.y) * self.zoom - cy;
        // Apply rotation.
        Vec2::new(dx * cos - dy * sin + cx, dx * sin + dy * cos + cy)
    }

    /// Compute the combined view-projection matrix for uploading to the GPU.
    ///
    /// The matrix maps world space directly to clip space, combining camera
    /// translation/zoom/rotation with the orthographic projection.
    pub(crate) fn view_proj_matrix(&self, screen_w: u32, screen_h: u32) -> [[f32; 4]; 4] {
        let w = screen_w as f32;
        let h = screen_h as f32;
        let z = self.zoom;
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let cx = w * 0.5;
        let cy = h * 0.5;
        let px = self.position.x;
        let py = self.position.y;

        // View translation terms (derived from T(-pos) * S(z) * R(rot, cx, cy)).
        let tx = cx * (1.0 - cos) + cy * sin - px * z * cos + py * z * sin;
        let ty = cy * (1.0 - cos) - cx * sin - px * z * sin - py * z * cos;

        // Row-major combined view * ortho matrix.
        // Matches `row_major float4x4 projection` in sprite.hlsl.
        [
            [2.0 * z * cos / w, -2.0 * z * sin / h, 0.0, 0.0],
            [-2.0 * z * sin / w, -2.0 * z * cos / h, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [2.0 * tx / w - 1.0, -2.0 * ty / h + 1.0, 0.0, 1.0],
        ]
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}
