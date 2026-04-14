use std::collections::HashMap;

use super::{renderer::SpriteBatch, texture::Texture2D};
use crate::{Colour, Error, Rect, Rukoh, Vec2};

/// Default printable ASCII range rasterized when using [`Font::load`].
const DEFAULT_CHARS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ\
     [\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

/// Atlas dimensions in pixels. Large enough for 128 glyphs at up to ~64px.
const ATLAS_SIZE: u32 = 1024;

/// Padding around each glyph in the atlas to prevent bleeding.
const GLYPH_PAD: u32 = 1;

// ── Per-glyph data ────────────────────────────────────────────────────────────

struct GlyphInfo {
    /// Normalised UV rect inside the atlas texture.
    uv: Rect,
    /// Horizontal offset from the pen position to the glyph's left edge.
    xmin: i32,
    /// Vertical offset from the baseline to the glyph's bottom edge.
    ymin: i32,
    /// Glyph bitmap size in pixels.
    width: u32,
    height: u32,
    /// Horizontal advance (how far to move the pen after this glyph).
    advance_width: f32,
}

// ── Font ──────────────────────────────────────────────────────────────────────

/// A rasterized font atlas ready for GPU text rendering.
///
/// Created via [`Font::load`] or [`Font::load_chars`]. Freed when dropped.
pub struct Font {
    glyphs: HashMap<char, GlyphInfo>,
    pub(crate) atlas: Texture2D,
    /// Distance from the baseline to the top of the tallest glyph (px).
    ascent: f32,
}

impl Font {
    /// Load a font, rasterizing all printable ASCII characters (32–126).
    pub fn load(rukoh: &Rukoh, bytes: &[u8], size: f32) -> Result<Self, Error> {
        Self::load_chars(rukoh, bytes, size, DEFAULT_CHARS)
    }

    /// Load a font, rasterizing only the characters in `chars`.
    ///
    /// Duplicate characters are silently ignored.
    pub fn load_chars(rukoh: &Rukoh, bytes: &[u8], size: f32, chars: &str) -> Result<Self, Error> {
        let settings = fontdue::FontSettings {
            scale: size,
            ..Default::default()
        };
        // ALLOCATION: error path — fontdue error (&str) converted to String.
        let fd_font =
            fontdue::Font::from_bytes(bytes, settings).map_err(|e| Error::Font(e.to_string()))?;

        build_font_atlas(rukoh, fd_font, size, chars)
    }

    /// The recommended line height in pixels for this font and size.
    pub fn line_height(&self) -> f32 {
        self.ascent
    }
}

// ── Internal atlas builder ────────────────────────────────────────────────────

fn build_font_atlas(
    rukoh: &Rukoh,
    fd_font: fontdue::Font,
    size: f32,
    chars: &str,
) -> Result<Font, Error> {
    let atlas_w = ATLAS_SIZE;
    let atlas_h = ATLAS_SIZE;
    // ALLOCATION: 4 MiB CPU pixel staging buffer (1024×1024×4) — once per Font::load;
    // freed after Texture2D::from_rgba8 uploads it to the GPU.
    let mut atlas_pixels = vec![0u8; (atlas_w * atlas_h * 4) as usize];

    // ALLOCATION: glyph lookup table — once per Font::load; lives for the lifetime of the Font.
    // Could be replaced with a fixed [Option<GlyphInfo>; 96] array for ASCII-only fonts.
    let mut glyphs: HashMap<char, GlyphInfo> = HashMap::new();
    let mut cursor_x: u32 = GLYPH_PAD;
    let mut cursor_y: u32 = GLYPH_PAD;
    let mut row_h: u32 = 0;
    let mut ascent: f32 = 0.0;

    for c in chars.chars() {
        if glyphs.contains_key(&c) {
            continue;
        }

        // ALLOCATION: per-glyph coverage bitmap from fontdue — one Vec<u8> per character at load
        // time; freed after blitting into atlas_pixels. No way to avoid with the fontdue API.
        let (metrics, bitmap) = fd_font.rasterize(c, size);

        // Track the tallest ascent (distance from baseline to top of glyph).
        let glyph_ascent = (metrics.ymin + metrics.height as i32) as f32;
        if glyph_ascent > ascent {
            ascent = glyph_ascent;
        }

        let gw = metrics.width as u32;
        let gh = metrics.height as u32;

        // Skip glyphs with no visible pixels (e.g. space).
        if gw == 0 || gh == 0 {
            glyphs.insert(
                c,
                GlyphInfo {
                    uv: Rect::new(0.0, 0.0, 0.0, 0.0),
                    xmin: metrics.xmin,
                    ymin: metrics.ymin,
                    width: 0,
                    height: 0,
                    advance_width: metrics.advance_width,
                },
            );
            continue;
        }

        // Move to next row if this glyph doesn't fit.
        if cursor_x + gw + GLYPH_PAD > atlas_w {
            cursor_y += row_h + GLYPH_PAD;
            cursor_x = GLYPH_PAD;
            row_h = 0;
        }

        if cursor_y + gh + GLYPH_PAD > atlas_h {
            return Err(Error::Font(
                // ALLOCATION: error path — &'static str promoted to String.
                "font atlas full — reduce size or character set".into(),
            ));
        }

        // Blit glyph coverage into the atlas as white + alpha.
        for row in 0..gh {
            for col in 0..gw {
                let src = (row * gw + col) as usize;
                let dst = ((cursor_y + row) * atlas_w + (cursor_x + col)) as usize * 4;
                let alpha = bitmap[src];
                atlas_pixels[dst] = 255;
                atlas_pixels[dst + 1] = 255;
                atlas_pixels[dst + 2] = 255;
                atlas_pixels[dst + 3] = alpha;
            }
        }

        let uv = Rect::new(
            cursor_x as f32 / atlas_w as f32,
            cursor_y as f32 / atlas_h as f32,
            gw as f32 / atlas_w as f32,
            gh as f32 / atlas_h as f32,
        );

        glyphs.insert(
            c,
            GlyphInfo {
                uv,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                width: gw,
                height: gh,
                advance_width: metrics.advance_width,
            },
        );

        cursor_x += gw + GLYPH_PAD;
        if gh > row_h {
            row_h = gh;
        }
    }

    let atlas = Texture2D::from_rgba8(rukoh.d3d_device(), &atlas_pixels, atlas_w, atlas_h)?;
    Ok(Font {
        glyphs,
        atlas,
        ascent,
    })
}

// ── Drawing (called from Frame) ───────────────────────────────────────────────

impl Font {
    /// Render `text` into `batch` at `pos` (top-left of the text line) with `colour`.
    pub(crate) fn render(&self, batch: &mut SpriteBatch, text: &str, pos: Vec2, colour: Colour) {
        // Baseline is offset down from `pos` by the ascent.
        let baseline_y = pos.y + self.ascent;
        let mut pen_x = pos.x;

        for c in text.chars() {
            let Some(g) = self.glyphs.get(&c) else {
                continue;
            };

            if g.width > 0 && g.height > 0 {
                // Top of the glyph in screen space (Y-down).
                let glyph_x = pen_x + g.xmin as f32;
                let glyph_y = baseline_y - (g.ymin as f32 + g.height as f32);

                use crate::DrawParams;
                let params = DrawParams {
                    dest_rect: Rect::new(glyph_x, glyph_y, g.width as f32, g.height as f32),
                    source_rect: Some(Rect::new(
                        g.uv.x * self.atlas.width as f32,
                        g.uv.y * self.atlas.height as f32,
                        g.uv.w * self.atlas.width as f32,
                        g.uv.h * self.atlas.height as f32,
                    )),
                    tint: colour,
                    ..Default::default()
                };
                batch.draw_texture_ex(
                    &self.atlas.srv,
                    self.atlas.width,
                    self.atlas.height,
                    &params,
                );
            }

            pen_x += g.advance_width;
        }
    }
}
