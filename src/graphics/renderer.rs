use bytemuck::{Pod, Zeroable};
use windows::core::Interface;
use windows::Win32::{
    Foundation::BOOL,
    Graphics::{
        Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Direct3D11::{
            ID3D11BlendState, ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout,
            ID3D11PixelShader, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11VertexShader,
            D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
            D3D11_BLEND, D3D11_BLEND_DESC, D3D11_BLEND_DEST_COLOR, D3D11_BLEND_INV_SRC_ALPHA,
            D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BLEND_SRC_ALPHA, D3D11_BUFFER_DESC,
            D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_CPU_ACCESS_WRITE, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            D3D11_FILTER_MIN_MAG_MIP_POINT, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA,
            D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_WRITE_DISCARD, D3D11_RENDER_TARGET_BLEND_DESC,
            D3D11_SAMPLER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE_ADDRESS_CLAMP,
            D3D11_USAGE_DYNAMIC, D3D11_USAGE_IMMUTABLE,
        },
        Dxgi::Common::{
            DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT,
            DXGI_FORMAT_UNKNOWN,
        },
    },
};

use super::texture::{Texture2D, TextureFilter};
use crate::{Colour, Error, Rect, Vec2};

/// Vertex layout: 32 bytes per vertex.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    colour: [f32; 4],
}

/// Extended draw parameters for [`Frame::draw_texture_ex`](crate::Frame::draw_texture_ex).
#[derive(Clone, Copy, Debug)]
pub struct DrawParams {
    /// Destination rectangle (position + drawn size).
    pub dest_rect: Rect,
    /// Sub-region of the source texture to draw. `None` = full texture.
    pub source_rect: Option<Rect>,
    /// Rotation in radians, applied around `origin`.
    pub rotation: f32,
    /// Pivot point in texture-local pixels (relative to `dest_rect.xy`).
    /// `Vec2::ZERO` = top-left corner.
    pub origin: Vec2,
    /// Colour tint multiplied with each pixel. [`Colour::WHITE`] = no tint.
    pub tint: Colour,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            dest_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            source_rect: None,
            rotation: 0.0,
            origin: Vec2::ZERO,
            tint: Colour::WHITE,
        }
    }
}

/// Controls how source pixels are composited onto the render target.
///
/// Set with [`Frame::set_blend_mode`](crate::Frame::set_blend_mode).
/// The mode persists until changed; it does **not** reset automatically at
/// the start of each frame, so call `set_blend_mode(BlendMode::Alpha)` to
/// restore the default when done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Standard alpha compositing: `src * src_alpha + dst * (1 − src_alpha)`.
    ///
    /// This is the default.
    #[default]
    Alpha,
    /// Additive blending: `src * src_alpha + dst`.
    ///
    /// Useful for glows, particles, and lighting effects — pixels are always
    /// brightened, never darkened.
    Additive,
    /// Multiplicative blending: `src * dst_colour + dst * (1 − src_alpha)`.
    ///
    /// Darkens the destination using the source colour; useful for shadows
    /// and colour-filter overlays.
    Multiplied,
}

/// Batched 2D quad renderer.
///
/// All draw calls within a frame are collected, sorted by texture, and issued
/// as minimal `DrawIndexed` calls at flush time.
pub struct SpriteBatch {
    context: ID3D11DeviceContext,

    vertex_buf: ID3D11Buffer,
    index_buf: ID3D11Buffer,
    const_buf: ID3D11Buffer,

    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    input_layout: ID3D11InputLayout,
    sampler_point: ID3D11SamplerState,
    sampler_bilinear: ID3D11SamplerState,
    current_filter: TextureFilter,
    blend_alpha: ID3D11BlendState,
    blend_additive: ID3D11BlendState,
    blend_multiplied: ID3D11BlendState,
    current_blend: BlendMode,

    /// 1×1 white RGBA texture used for solid shapes.
    pub(crate) white_tex: Texture2D,

    /// CPU-side staging buffer. Pre-allocated to `max_quads * 4`, never grows.
    vertices: Vec<Vertex>,
    quad_count: usize,
    /// SRV of the texture bound to the current pending batch.
    current_srv: Option<ID3D11ShaderResourceView>,
    /// Maximum quads per batch — set from [`RukohConfig::batch_size`].
    max_quads: usize,
}

// ── Construction ──────────────────────────────────────────────────────────────

impl SpriteBatch {
    pub fn new(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        render_width: u32,
        render_height: u32,
        max_quads: usize,
    ) -> Result<Self, Error> {
        let vertex_buf = create_vertex_buffer(device, max_quads)?;
        let index_buf = create_index_buffer(device, max_quads)?;
        let const_buf = create_constant_buffer(device)?;

        let vs_bytecode: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sprite_vs.dxbc"));
        let ps_bytecode: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sprite_ps.dxbc"));

        let (vertex_shader, input_layout) = create_vertex_shader(device, vs_bytecode)?;
        let pixel_shader = create_pixel_shader(device, ps_bytecode)?;
        let sampler_point = create_sampler(device, D3D11_FILTER_MIN_MAG_MIP_POINT)?;
        let sampler_bilinear = create_sampler(device, D3D11_FILTER_MIN_MAG_MIP_LINEAR)?;
        let blend_alpha = make_blend_state(
            device,
            D3D11_BLEND_SRC_ALPHA,
            D3D11_BLEND_INV_SRC_ALPHA,
            D3D11_BLEND_ONE,
            D3D11_BLEND_INV_SRC_ALPHA,
        )?;
        let blend_additive = make_blend_state(
            device,
            D3D11_BLEND_SRC_ALPHA,
            D3D11_BLEND_ONE,
            D3D11_BLEND_ONE,
            D3D11_BLEND_ONE,
        )?;
        let blend_multiplied = make_blend_state(
            device,
            D3D11_BLEND_DEST_COLOR,
            D3D11_BLEND_INV_SRC_ALPHA,
            D3D11_BLEND_ONE,
            D3D11_BLEND_INV_SRC_ALPHA,
        )?;

        let white_tex =
            Texture2D::from_rgba8(device, &[255, 255, 255, 255], 1, 1, TextureFilter::Point)?;

        // Upload the initial projection matrix.
        upload_projection(context, &const_buf, render_width, render_height)?;

        // ALLOCATION: CPU vertex staging buffer — once at startup, exact capacity of
        // max_quads * 4 vertices; never grows or reallocates during normal use.
        let mut vertices = Vec::new();
        vertices.reserve_exact(max_quads * 4);

        Ok(Self {
            context: context.clone(),
            vertex_buf,
            index_buf,
            const_buf,
            vertex_shader,
            pixel_shader,
            input_layout,
            sampler_point,
            sampler_bilinear,
            current_filter: TextureFilter::Point,
            blend_alpha,
            blend_additive,
            blend_multiplied,
            current_blend: BlendMode::Alpha,
            white_tex,
            vertices,
            quad_count: 0,
            current_srv: None,
            max_quads,
        })
    }

    /// Call once per frame (in `next_frame`) to bind the pipeline.
    pub fn begin_frame(&self) {
        let ctx = &self.context;
        unsafe {
            let stride = std::mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            let vbs: [Option<ID3D11Buffer>; 1] = [Some(self.vertex_buf.clone())];
            let strides: [u32; 1] = [stride];
            let offsets: [u32; 1] = [offset];
            ctx.IASetVertexBuffers(
                0,
                1,
                Some(vbs.as_ptr()),
                Some(strides.as_ptr()),
                Some(offsets.as_ptr()),
            );
            ctx.IASetIndexBuffer(Some(&self.index_buf), DXGI_FORMAT_R32_UINT, 0);
            ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetInputLayout(Some(&self.input_layout));
            ctx.VSSetShader(Some(&self.vertex_shader), None);
            ctx.VSSetConstantBuffers(0, Some(&[Some(self.const_buf.clone())]));
            ctx.PSSetShader(Some(&self.pixel_shader), None);
            ctx.PSSetSamplers(0, Some(&[Some(self.sampler_point.clone())]));
            ctx.OMSetBlendState(Some(self.active_blend_state()), None, 0xFFFF_FFFF);
        }
    }

    /// Flush all pending quads and call this at frame end (in `Frame::drop`).
    pub fn end_frame(&mut self) {
        self.flush();
    }

    /// Flush the batch and upload a new projection matrix (e.g. from a camera).
    pub fn set_projection(&mut self, matrix: &[[f32; 4]; 4]) -> Result<(), Error> {
        self.flush();
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context.Map(
                &self.const_buf,
                0,
                D3D11_MAP_WRITE_DISCARD,
                0,
                Some(&mut mapped),
            )?;
            std::ptr::copy_nonoverlapping(
                matrix.as_ptr() as *const u8,
                mapped.pData as *mut u8,
                64,
            );
            self.context.Unmap(&self.const_buf, 0);
        }
        Ok(())
    }

    /// Flush and restore the default ortho projection.
    pub fn reset_projection(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.flush();
        upload_projection(&self.context, &self.const_buf, width, height)
    }

    /// Flush pending draws and switch to the given blend mode.
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        if self.current_blend == mode {
            return;
        }
        self.flush();
        self.current_blend = mode;
        unsafe {
            self.context
                .OMSetBlendState(Some(self.active_blend_state()), None, 0xFFFF_FFFF);
        }
    }

    fn active_blend_state(&self) -> &ID3D11BlendState {
        match self.current_blend {
            BlendMode::Alpha => &self.blend_alpha,
            BlendMode::Additive => &self.blend_additive,
            BlendMode::Multiplied => &self.blend_multiplied,
        }
    }

    fn active_sampler(&self) -> &ID3D11SamplerState {
        match self.current_filter {
            TextureFilter::Point => &self.sampler_point,
            TextureFilter::Bilinear => &self.sampler_bilinear,
        }
    }
}

// ── Public draw interface (called from Frame) ─────────────────────────────────

impl SpriteBatch {
    /// Draw a textured quad with full control over dest rect, source rect,
    /// rotation, origin pivot, and tint.
    pub fn draw_texture_ex(
        &mut self,
        srv: &ID3D11ShaderResourceView,
        tex_w: u32,
        tex_h: u32,
        filter: TextureFilter,
        params: &DrawParams,
    ) {
        // Compute normalised UV rect from source_rect (or full texture).
        let uv = match params.source_rect {
            Some(src) => Rect::new(
                src.x / tex_w as f32,
                src.y / tex_h as f32,
                src.w / tex_w as f32,
                src.h / tex_h as f32,
            ),
            None => Rect::new(0.0, 0.0, 1.0, 1.0),
        };

        self.push_quad(
            srv,
            params.dest_rect,
            uv,
            params.rotation,
            params.origin,
            filter,
            params.tint,
        );
    }

    /// Draw a filled solid-colour rectangle.
    pub fn draw_rect(&mut self, rect: Rect, colour: Colour) {
        let srv = self.white_tex.srv.clone();
        self.push_quad(
            &srv,
            rect,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            0.0,
            Vec2::ZERO,
            TextureFilter::Point,
            colour,
        );
    }

    /// Draw a filled rectangle with a rotation (in radians) around `origin`.
    ///
    /// `origin` is the pivot point in rect-local pixels relative to the
    /// rect's top-left corner. `Vec2::ZERO` rotates around the top-left;
    /// `Vec2::new(rect.w * 0.5, rect.h * 0.5)` rotates around the centre.
    pub fn draw_rect_ex(&mut self, rect: Rect, origin: Vec2, rotation: f32, colour: Colour) {
        let srv = self.white_tex.srv.clone();
        self.push_quad(
            &srv,
            rect,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            rotation,
            origin,
            TextureFilter::Point,
            colour,
        );
    }

    /// Draw a hollow rectangle outline.
    pub fn draw_rect_lines(&mut self, rect: Rect, thickness: f32, colour: Colour) {
        let t = thickness;
        let Rect { x, y, w, h } = rect;
        // Top, Bottom, Left, Right — sized to avoid corner overlap.
        self.draw_rect(Rect::new(x, y, w, t), colour);
        self.draw_rect(Rect::new(x, y + h - t, w, t), colour);
        self.draw_rect(Rect::new(x, y + t, t, h - 2.0 * t), colour);
        self.draw_rect(Rect::new(x + w - t, y + t, t, h - 2.0 * t), colour);
    }

    /// Draw a thick line between two points.
    pub fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, colour: Colour) {
        let diff = end - start;
        let len = diff.length();
        if len < f32::EPSILON {
            return;
        }

        let angle = diff.y.atan2(diff.x);
        let origin = Vec2::new(0.0, thickness * 0.5);
        let rect = Rect::new(start.x, start.y, len, thickness);
        let srv = self.white_tex.srv.clone();
        self.push_quad(
            &srv,
            rect,
            Rect::new(0.0, 0.0, 1.0, 1.0),
            angle,
            origin,
            TextureFilter::Point,
            colour,
        );
    }

    /// Draw a circle outline tessellated into 32 rotated quads.
    pub fn draw_circle_lines(&mut self, centre: Vec2, radius: f32, thickness: f32, colour: Colour) {
        const SEGMENTS: usize = 32;
        let step = std::f32::consts::TAU / SEGMENTS as f32;
        let chord = 2.0 * radius * (step * 0.5).sin();
        let half_chord = chord * 0.5;
        let half_thick = thickness * 0.5;
        let origin = Vec2::new(half_chord, half_thick);
        let srv = self.white_tex.srv.clone();
        for i in 0..SEGMENTS {
            let angle_mid = (i as f32 + 0.5) * step;
            let px = centre.x + angle_mid.cos() * radius;
            let py = centre.y + angle_mid.sin() * radius;
            let rect = Rect::new(px - half_chord, py - half_thick, chord, thickness);
            self.push_quad(
                &srv,
                rect,
                Rect::new(0.0, 0.0, 1.0, 1.0),
                angle_mid + std::f32::consts::FRAC_PI_2,
                origin,
                TextureFilter::Point,
                colour,
            );
        }
    }

    /// Draw a filled triangle. Vertices should be in counter-clockwise order.
    pub fn draw_triangle(&mut self, v1: Vec2, v2: Vec2, v3: Vec2, colour: Colour) {
        self.flush();
        let col = colour.to_array();
        let verts = [
            Vertex {
                pos: [v1.x, v1.y],
                uv: [0.5, 0.5],
                colour: col,
            },
            Vertex {
                pos: [v2.x, v2.y],
                uv: [0.5, 0.5],
                colour: col,
            },
            Vertex {
                pos: [v3.x, v3.y],
                uv: [0.5, 0.5],
                colour: col,
            },
        ];
        let white_srv = self.white_tex.srv.clone();
        self.draw_triangles(&verts, &white_srv);
    }

    /// Draw a filled circle tessellated into 32 triangle-list segments.
    pub fn draw_circle(&mut self, centre: Vec2, radius: f32, colour: Colour) {
        const SEGMENTS: usize = 32;
        // Flush any pending quads first — circle uses a separate draw call.
        self.flush();

        let col = colour.to_array();
        let zero = Vertex {
            pos: [0.0; 2],
            uv: [0.5; 2],
            colour: [0.0; 4],
        };
        let mut verts = [zero; SEGMENTS * 3];
        let step = std::f32::consts::TAU / SEGMENTS as f32;

        for i in 0..SEGMENTS {
            let a0 = i as f32 * step;
            let a1 = (i + 1) as f32 * step;
            let p0 = Vec2::new(centre.x + a0.cos() * radius, centre.y + a0.sin() * radius);
            let p1 = Vec2::new(centre.x + a1.cos() * radius, centre.y + a1.sin() * radius);
            let base = i * 3;
            verts[base] = Vertex {
                pos: [centre.x, centre.y],
                uv: [0.5, 0.5],
                colour: col,
            };
            verts[base + 1] = Vertex {
                pos: [p0.x, p0.y],
                uv: [0.5, 0.5],
                colour: col,
            };
            verts[base + 2] = Vertex {
                pos: [p1.x, p1.y],
                uv: [0.5, 0.5],
                colour: col,
            };
        }

        let white_srv = self.white_tex.srv.clone();
        self.draw_triangles(&verts, &white_srv);
    }
}

// ── Internal batch machinery ──────────────────────────────────────────────────

impl SpriteBatch {
    /// Push one quad into the batch, flushing first if needed (texture/filter changed or full).
    #[allow(clippy::too_many_arguments)]
    fn push_quad(
        &mut self,
        srv: &ID3D11ShaderResourceView,
        dest: Rect,
        uv: Rect,
        rotation: f32,
        origin: Vec2,
        filter: TextureFilter,
        colour: Colour,
    ) {
        // Flush if texture or filter changes, or batch is full.
        let texture_changed = self
            .current_srv
            .as_ref()
            .map(|c| c.as_raw() != srv.as_raw())
            .unwrap_or(true);
        let filter_changed = self.current_filter != filter;

        if texture_changed || filter_changed {
            self.flush();
            self.current_srv = Some(srv.clone());
            if filter_changed {
                self.current_filter = filter;
                unsafe {
                    self.context
                        .PSSetSamplers(0, Some(&[Some(self.active_sampler().clone())]));
                }
            }
        } else if self.quad_count >= self.max_quads {
            self.flush();
        }

        let col = colour.to_array();

        if rotation == 0.0 {
            // Fast path: no rotation — skip sin_cos and all pivot arithmetic.
            // Origin is irrelevant when there is no rotation; the sprite always
            // maps directly onto dest_rect.
            let x0 = dest.x;
            let y0 = dest.y;
            let x1 = dest.x + dest.w;
            let y1 = dest.y + dest.h;
            self.vertices.push(Vertex {
                pos: [x0, y0],
                uv: [uv.x, uv.y],
                colour: col,
            });
            self.vertices.push(Vertex {
                pos: [x1, y0],
                uv: [uv.x + uv.w, uv.y],
                colour: col,
            });
            self.vertices.push(Vertex {
                pos: [x1, y1],
                uv: [uv.x + uv.w, uv.y + uv.h],
                colour: col,
            });
            self.vertices.push(Vertex {
                pos: [x0, y1],
                uv: [uv.x, uv.y + uv.h],
                colour: col,
            });
        } else {
            // General path: rotate each corner around the pivot point.
            let corners_local = [
                Vec2::new(-origin.x, -origin.y),
                Vec2::new(dest.w - origin.x, -origin.y),
                Vec2::new(dest.w - origin.x, dest.h - origin.y),
                Vec2::new(-origin.x, dest.h - origin.y),
            ];
            let uvs = [
                Vec2::new(uv.x, uv.y),
                Vec2::new(uv.x + uv.w, uv.y),
                Vec2::new(uv.x + uv.w, uv.y + uv.h),
                Vec2::new(uv.x, uv.y + uv.h),
            ];
            let (sin, cos) = rotation.sin_cos();
            let pivot_world = Vec2::new(dest.x + origin.x, dest.y + origin.y);

            for (lc, uv) in corners_local.iter().zip(uvs.iter()) {
                let rotated = Vec2::new(lc.x * cos - lc.y * sin, lc.x * sin + lc.y * cos);
                let world = pivot_world + rotated;
                self.vertices.push(Vertex {
                    pos: [world.x, world.y],
                    uv: [uv.x, uv.y],
                    colour: col,
                });
            }
        }

        self.quad_count += 1;
    }

    /// Flush pending quads to the GPU using the static index buffer.
    fn flush(&mut self) {
        if self.quad_count == 0 {
            return;
        }

        let ctx = &self.context;

        // Map, copy, unmap.
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            ctx.Map(
                &self.vertex_buf,
                0,
                D3D11_MAP_WRITE_DISCARD,
                0,
                Some(&mut mapped),
            )
            .expect("Failed to map vertex buffer");

            let byte_len = self.vertices.len() * std::mem::size_of::<Vertex>();
            std::ptr::copy_nonoverlapping(
                self.vertices.as_ptr() as *const u8,
                mapped.pData as *mut u8,
                byte_len,
            );

            ctx.Unmap(&self.vertex_buf, 0);
        }

        // Bind texture and draw.
        if let Some(srv) = &self.current_srv {
            unsafe {
                ctx.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
            }
        }

        unsafe {
            ctx.DrawIndexed((self.quad_count * 6) as u32, 0, 0);
        }

        self.quad_count = 0;
        self.vertices.clear(); // retains capacity
    }

    /// Draw an arbitrary triangle list, bypassing the index buffer.
    /// Caller must have already flushed pending quads.
    fn draw_triangles(&self, verts: &[Vertex], srv: &ID3D11ShaderResourceView) {
        if verts.is_empty() {
            return;
        }
        debug_assert!(
            verts.len() <= self.max_quads * 4,
            "Circle vertex count exceeds buffer"
        );

        let ctx = &self.context;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            ctx.Map(
                &self.vertex_buf,
                0,
                D3D11_MAP_WRITE_DISCARD,
                0,
                Some(&mut mapped),
            )
            .expect("Failed to map vertex buffer for triangles");

            let byte_len = std::mem::size_of_val(verts);
            std::ptr::copy_nonoverlapping(
                verts.as_ptr() as *const u8,
                mapped.pData as *mut u8,
                byte_len,
            );

            ctx.Unmap(&self.vertex_buf, 0);

            // Unbind the index buffer so Draw() doesn't use it.
            ctx.IASetIndexBuffer(None, DXGI_FORMAT_UNKNOWN, 0);
            ctx.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
            ctx.Draw(verts.len() as u32, 0);
            // Restore the static index buffer for subsequent quad draws.
            ctx.IASetIndexBuffer(Some(&self.index_buf), DXGI_FORMAT_R32_UINT, 0);
        }
    }
}

// ── D3D11 resource helpers ────────────────────────────────────────────────────

fn create_vertex_buffer(device: &ID3D11Device, max_quads: usize) -> Result<ID3D11Buffer, Error> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (max_quads * 4 * std::mem::size_of::<Vertex>()) as u32,
        Usage: D3D11_USAGE_DYNAMIC,
        BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
        ..Default::default()
    };
    let mut buf: Option<ID3D11Buffer> = None;
    unsafe { device.CreateBuffer(&desc, None, Some(&mut buf))? };
    Ok(buf.unwrap())
}

fn create_index_buffer(device: &ID3D11Device, max_quads: usize) -> Result<ID3D11Buffer, Error> {
    // ALLOCATION: index pattern staging — once at startup to populate the immutable GPU index
    // buffer; freed immediately after CreateBuffer returns.
    // u32 indices are used so max_quads is not constrained by the u16 ceiling.
    // Pre-fill with the repeating quad pattern: 0,1,2, 0,2,3, 4,5,6, 4,6,7, ...
    let mut indices: Vec<u32> = Vec::with_capacity(max_quads * 6);
    for i in 0..max_quads as u32 {
        let base = i * 4;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (indices.len() * std::mem::size_of::<u32>()) as u32,
        Usage: D3D11_USAGE_IMMUTABLE,
        BindFlags: D3D11_BIND_INDEX_BUFFER.0 as u32,
        ..Default::default()
    };
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: indices.as_ptr() as *const _,
        SysMemPitch: 0,
        SysMemSlicePitch: 0,
    };
    let mut buf: Option<ID3D11Buffer> = None;
    unsafe { device.CreateBuffer(&desc, Some(&init), Some(&mut buf))? };
    Ok(buf.unwrap())
}

fn create_constant_buffer(device: &ID3D11Device) -> Result<ID3D11Buffer, Error> {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: 64, // 4×4 f32 matrix = 64 bytes (16-byte aligned)
        Usage: D3D11_USAGE_DYNAMIC,
        BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
        CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
        ..Default::default()
    };
    let mut buf: Option<ID3D11Buffer> = None;
    unsafe { device.CreateBuffer(&desc, None, Some(&mut buf))? };
    Ok(buf.unwrap())
}

pub(crate) fn upload_projection(
    context: &ID3D11DeviceContext,
    const_buf: &ID3D11Buffer,
    width: u32,
    height: u32,
) -> Result<(), Error> {
    // Row-major orthographic matrix: maps (0,0)–(w,h) to clip space.
    // Matches `row_major float4x4 projection` in sprite.hlsl.
    let w = width as f32;
    let h = height as f32;
    let proj: [[f32; 4]; 4] = [
        [2.0 / w, 0.0, 0.0, 0.0],
        [0.0, -2.0 / h, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ];

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe {
        context.Map(const_buf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped))?;
        std::ptr::copy_nonoverlapping(proj.as_ptr() as *const u8, mapped.pData as *mut u8, 64);
        context.Unmap(const_buf, 0);
    }
    Ok(())
}

fn create_vertex_shader(
    device: &ID3D11Device,
    bytecode: &[u8],
) -> Result<(ID3D11VertexShader, ID3D11InputLayout), Error> {
    let mut vs: Option<ID3D11VertexShader> = None;
    unsafe {
        device.CreateVertexShader(bytecode, None, Some(&mut vs))?;
    }
    let vs = vs.unwrap();

    let elements = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 8,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 16,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    let mut layout: Option<ID3D11InputLayout> = None;
    unsafe {
        device.CreateInputLayout(&elements, bytecode, Some(&mut layout))?;
    }

    Ok((vs, layout.unwrap()))
}

fn create_pixel_shader(device: &ID3D11Device, bytecode: &[u8]) -> Result<ID3D11PixelShader, Error> {
    let mut ps: Option<ID3D11PixelShader> = None;
    unsafe {
        device.CreatePixelShader(bytecode, None, Some(&mut ps))?;
    }
    Ok(ps.unwrap())
}

fn create_sampler(
    device: &ID3D11Device,
    filter: windows::Win32::Graphics::Direct3D11::D3D11_FILTER,
) -> Result<ID3D11SamplerState, Error> {
    let desc = D3D11_SAMPLER_DESC {
        Filter: filter,
        AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
        AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
        MaxLOD: f32::MAX,
        ..Default::default()
    };
    let mut s: Option<ID3D11SamplerState> = None;
    unsafe { device.CreateSamplerState(&desc, Some(&mut s))? };
    Ok(s.unwrap())
}

fn make_blend_state(
    device: &ID3D11Device,
    src_blend: D3D11_BLEND,
    dest_blend: D3D11_BLEND,
    src_blend_alpha: D3D11_BLEND,
    dest_blend_alpha: D3D11_BLEND,
) -> Result<ID3D11BlendState, Error> {
    let rt = D3D11_RENDER_TARGET_BLEND_DESC {
        BlendEnable: BOOL(1),
        SrcBlend: src_blend,
        DestBlend: dest_blend,
        BlendOp: D3D11_BLEND_OP_ADD,
        SrcBlendAlpha: src_blend_alpha,
        DestBlendAlpha: dest_blend_alpha,
        BlendOpAlpha: D3D11_BLEND_OP_ADD,
        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };
    let desc = D3D11_BLEND_DESC {
        RenderTarget: [
            rt,
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
            D3D11_RENDER_TARGET_BLEND_DESC::default(),
        ],
        ..Default::default()
    };
    let mut b: Option<ID3D11BlendState> = None;
    unsafe { device.CreateBlendState(&desc, Some(&mut b))? };
    Ok(b.unwrap())
}
