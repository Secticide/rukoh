# SDF Shape Rendering — Future Architecture Notes

## Why this was considered

When planning `draw_circle_lines` and `draw_rect_rounded`, we evaluated replacing CPU
tessellation with Signed Distance Field (SDF) evaluation in the pixel shader.
We chose CPU tessellation for now to keep things simple, but the SDF approach is
worth revisiting when rounded rectangles are implemented — the maths are significantly
simpler and the output quality is better.

---

## Current architecture baseline

- **Vertex struct:** `pos [f32;2] + uv [f32;2] + colour [f32;4]` = 32 bytes. No spare fields.
- **Pixel shader:** `g_texture.Sample(g_sampler, input.uv) * input.colour` — texture sample × tint.
- **`draw_circle`** already breaks batching: flushes pending quads, then issues its own `Draw`
  via a separate `draw_triangles` path. SDF shapes would have the same batching cost.

---

## SDF approach summary

A new pipeline (`sdf.hlsl`) with an extended vertex format:

```
pos:    [f32; 2]   screen position (same transform as sprites)
local:  [f32; 2]   local-space coords for SDF evaluation (e.g. -radius..+radius)
colour: [f32; 4]   RGBA
params: [f32; 4]   [param0, param1, param2, shape_type_discriminant]
```
Total: 48 bytes. Entirely separate from the sprite pipeline — no change to existing code.

HLSL SDF expressions (all trivially short):

```hlsl
// Filled circle — params.x = radius
dist = length(local) - params.x;

// Circle outline — params.x = radius, params.y = half-thickness
dist = abs(length(local) - params.x) - params.y;

// Rounded rect (sdRoundedBox) — params.xy = half-extents, params.z = corner radius
float2 q = abs(local) - params.xy + params.z;
dist = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - params.z;

// Anti-aliased output
float alpha = 1.0 - smoothstep(-0.5, 0.5, dist);
return colour * float4(1, 1, 1, alpha);
```

---

## Quality comparison

| Shape | Tessellation | SDF |
|---|---|---|
| Filled circle | Faceted at large radii | Smooth at any scale |
| Circle outline | Approximate thickness, aliased | Exact thickness, smooth |
| Rounded rect | Many quads, aliased corners | 1 quad, perfect corners |
| Rects, lines, triangles | Fine as-is | No benefit |

---

## Build changes required

1. `src/shaders/sdf.hlsl` — new HLSL file.
2. `build.rs` — add compilation of `sdf_vs.dxbc` / `sdf_ps.dxbc`.
3. New `SdfBatch` struct (or second pipeline inside `SpriteBatch`) — own vertex buffer,
   input layout, PSO.
4. Migrate `draw_circle` to SDF; add `draw_circle_lines` and `draw_rect_rounded` as extra
   cases in the same pixel shader.

---

## Recommended trigger

Revisit when implementing `draw_rect_rounded` — the tessellation approach for rounded rects
is sufficiently complex that the SDF investment pays off there.
