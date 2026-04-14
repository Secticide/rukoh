use windows::Win32::Graphics::{
    Direct3D::D3D_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE,
        D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
        D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

use crate::{Error, Rukoh};

/// A GPU texture loaded from image data.
///
/// Created via [`Texture2D::load`]. Freed when dropped.
pub struct Texture2D {
    pub(crate) srv: ID3D11ShaderResourceView,
    pub width: u32,
    pub height: u32,
}

impl Texture2D {
    /// Load a texture from embedded bytes (use `include_bytes!`).
    ///
    /// Supports PNG, JPEG, BMP, and TGA.
    pub fn load(rukoh: &Rukoh, bytes: &[u8]) -> Result<Self, Error> {
        let device = rukoh.d3d_device();

        // ALLOCATION: decoded image pixel buffer — one Vec<u8> allocated by the `image` crate
        // at load time; freed after CreateTexture2D uploads it to the GPU. No way to avoid
        // with the image crate's public API (it always decodes to an owned buffer).
        // ALLOCATION: error path — image error converted to String.
        let img = image::load_from_memory(bytes)
            .map_err(|e| Error::Image(e.to_string()))?
            .into_rgba8();
        let (w, h) = img.dimensions();

        Self::from_rgba8(device, img.as_raw(), w, h)
    }

    /// Create a texture directly from raw RGBA8 pixel data.
    ///
    /// Useful for procedurally-generated textures. Each pixel is four bytes:
    /// `[R, G, B, A]`.
    pub fn from_pixels(rukoh: &crate::Rukoh, pixels: &[u8], w: u32, h: u32) -> Result<Self, Error> {
        Self::from_rgba8(rukoh.d3d_device(), pixels, w, h)
    }

    pub(crate) fn from_rgba8(
        device: &ID3D11Device,
        pixels: &[u8],
        w: u32,
        h: u32,
    ) -> Result<Self, Error> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            ..Default::default()
        };

        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: pixels.as_ptr() as *const _,
            SysMemPitch: w * 4,
            SysMemSlicePitch: 0,
        };

        let mut tex: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&desc, Some(&init), Some(&mut tex))? };
        let tex = tex.unwrap();

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
        unsafe { device.CreateShaderResourceView(&tex, Some(&srv_desc), Some(&mut srv))? };
        let srv = srv.unwrap();

        Ok(Self {
            srv,
            width: w,
            height: h,
        })
    }
}
