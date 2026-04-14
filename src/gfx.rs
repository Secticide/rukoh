use windows::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct3D::D3D_DRIVER_TYPE_HARDWARE,
        Direct3D11::{
            D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext,
            ID3D11RenderTargetView, ID3D11Texture2D, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
            D3D11_VIEWPORT,
        },
        Dxgi::Common::{
            DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
            DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
        },
        Dxgi::{
            IDXGISwapChain, DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD,
            DXGI_USAGE_RENDER_TARGET_OUTPUT,
        },
    },
};

use crate::{Colour, Error};

pub struct GfxDevice {
    pub(crate) device: ID3D11Device,
    pub(crate) context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain,
    pub(crate) rtv: ID3D11RenderTargetView,
}

impl GfxDevice {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self, Error> {
        let swap_desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC {
                Width: width,
                Height: height,
                RefreshRate: DXGI_RATIONAL {
                    Numerator: 0,
                    Denominator: 1,
                },
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
            },
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: hwnd,
            Windowed: windows::Win32::Foundation::BOOL(1),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        };

        let mut device: Option<ID3D11Device> = None;
        let mut swap_chain: Option<IDXGISwapChain> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

        unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG::default(),
                None,
                D3D11_SDK_VERSION,
                Some(&swap_desc),
                Some(&mut swap_chain),
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }

        let device = device.unwrap();
        let swap_chain = swap_chain.unwrap();
        let context = context.unwrap();

        let back_buffer: ID3D11Texture2D = unsafe { swap_chain.GetBuffer(0)? };

        let mut rtv: Option<ID3D11RenderTargetView> = None;
        unsafe { device.CreateRenderTargetView(&back_buffer, None, Some(&mut rtv))? };
        let rtv = rtv.unwrap();

        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        unsafe {
            context.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            context.RSSetViewports(Some(&[viewport]));
        }

        Ok(Self {
            device,
            context,
            swap_chain,
            rtv,
        })
    }

    pub(crate) fn clear(&self, colour: Colour) {
        unsafe {
            self.context
                .ClearRenderTargetView(&self.rtv, &colour.to_array());
        }
    }

    pub(crate) fn clear_rtv(&self, rtv: &ID3D11RenderTargetView, colour: Colour) {
        unsafe {
            self.context.ClearRenderTargetView(rtv, &colour.to_array());
        }
    }

    /// Bind an off-screen render target view and set the viewport to its size.
    pub(crate) fn bind_render_target(&self, rtv: &ID3D11RenderTargetView, width: u32, height: u32) {
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        unsafe {
            self.context
                .OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            self.context.RSSetViewports(Some(&[viewport]));
        }
    }

    /// Restore the swap-chain back buffer as the active render target.
    pub(crate) fn bind_back_buffer(&self, width: u32, height: u32) {
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: width as f32,
            Height: height as f32,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        unsafe {
            self.context
                .OMSetRenderTargets(Some(&[Some(self.rtv.clone())]), None);
            self.context.RSSetViewports(Some(&[viewport]));
        }
    }

    pub fn present(&self, sync_interval: u32) {
        unsafe {
            // Ignore DXGI_STATUS_OCCLUDED (window minimised/occluded).
            // Genuine device-lost errors surface on the next API call.
            let _ = self.swap_chain.Present(sync_interval, DXGI_PRESENT(0));
        }
    }
}
