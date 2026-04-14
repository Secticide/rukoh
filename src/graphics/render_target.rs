use std::ops::Deref;

use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Device, ID3D11RenderTargetView, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

use super::texture::Texture2D;
use crate::{Error, Rukoh};

/// An off-screen render target that can also be drawn as a texture.
///
/// Created via [`RenderTarget::new`]. Freed when dropped.
///
/// Implements [`Deref<Target = Texture2D>`] so you can pass it directly to
/// [`Frame::draw_texture`](crate::Frame::draw_texture) and
/// [`Frame::draw_texture_ex`](crate::Frame::draw_texture_ex).
pub struct RenderTarget {
    pub(crate) rtv: ID3D11RenderTargetView,
    texture: Texture2D,
}

impl RenderTarget {
    /// Create a new off-screen render target of the given size in pixels.
    pub fn new(rukoh: &Rukoh, width: u32, height: u32) -> Result<Self, Error> {
        let device = rukoh.d3d_device();

        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            ..Default::default()
        };

        let mut tex2d = None;
        unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex2d))? };
        let tex2d = tex2d.unwrap();

        let mut rtv = None;
        unsafe { device.CreateRenderTargetView(&tex2d, None, Some(&mut rtv))? };
        let rtv = rtv.unwrap();

        // Build the Texture2D (SRV) for use as a draw source.
        let texture = build_srv(device, &tex2d, width, height)?;

        Ok(Self { rtv, texture })
    }
}

impl Deref for RenderTarget {
    type Target = Texture2D;

    fn deref(&self) -> &Texture2D {
        &self.texture
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn build_srv(
    device: &ID3D11Device,
    tex: &windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,
    width: u32,
    height: u32,
) -> Result<Texture2D, Error> {
    use windows::Win32::Graphics::{
        Direct3D::D3D_SRV_DIMENSION_TEXTURE2D,
        Direct3D11::{
            ID3D11ShaderResourceView, D3D11_SHADER_RESOURCE_VIEW_DESC,
            D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_SRV,
        },
    };

    let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: D3D11_TEX2D_SRV {
                MostDetailedMip: 0,
                MipLevels: 1,
            },
        },
    };

    let mut srv: Option<ID3D11ShaderResourceView> = None;
    unsafe { device.CreateShaderResourceView(tex, Some(&srv_desc), Some(&mut srv))? };

    Ok(Texture2D {
        srv: srv.unwrap(),
        width,
        height,
    })
}
