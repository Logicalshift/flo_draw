use crate::action::*;

use wgpu;

use std::sync::*;

///
/// Represents a WGPU render target
///
pub enum RenderTarget {
    /// Simple texture
    Texture {
        texture:        Arc<wgpu::Texture>,
        texture_format: wgpu::TextureFormat,
        width:          u32,
        height:         u32,
    },

    /// Multisampled texture
    Multisampled {
        texture:        Arc<wgpu::Texture>,
        texture_format: wgpu::TextureFormat,
        resolved:       Option<Arc<wgpu::Texture>>,
        width:          u32,
        height:         u32,
    },
}

impl RenderTarget {
    ///
    /// Creates a new render target
    ///
    pub fn new(device: &wgpu::Device, width: u32, height: u32, render_target_type: RenderTargetType) -> RenderTarget {
        // Set up the texture descriptor (basic width and height and standard format)
        let mut descriptor = wgpu::TextureDescriptor {
            label:  Some("render_target"),
            size:   wgpu::Extent3d {
                width:                  width,
                height:                 height,
                depth_or_array_layers:  1,
            },
            mip_level_count:    1,
            sample_count:       1,
            dimension:          wgpu::TextureDimension::D2,
            format:             wgpu::TextureFormat::Bgra8Unorm,
            usage:              wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        };

        // Adjust according to the render target type
        use self::RenderTargetType::*;
        match render_target_type {
            Standard                        => { }
            StandardForReading              => { }
            Multisampled                    |
            MultisampledTexture             => { descriptor.sample_count = 4; }
            Monochrome                      => { descriptor.format = wgpu::TextureFormat::R8Unorm; },
            MonochromeMultisampledTexture   => { descriptor.format = wgpu::TextureFormat::R8Unorm; descriptor.sample_count = 1; }
        }

        // Create the texture for this render target
        let texture = device.create_texture(&descriptor);

        // Return the resulting render target
        match render_target_type {
            Standard                        |
            StandardForReading              |
            Monochrome                      => {
                RenderTarget::Texture {
                    texture:        Arc::new(texture),
                    texture_format: descriptor.format, 
                    width:          width,
                    height:         height,
                }
            },

            Multisampled                    |
            MultisampledTexture             |
            MonochromeMultisampledTexture   => {
                RenderTarget::Multisampled {
                    texture:        Arc::new(texture),
                    texture_format: descriptor.format,
                    resolved:       None,
                    width:          width,
                    height:         height,
                }
            },
        }
    }

    ///
    /// Retrieves the texture attached to this render target
    ///
    pub fn texture(&self) -> Arc<wgpu::Texture> {
        match self {
            RenderTarget::Texture { texture, .. }       => Arc::clone(texture),
            RenderTarget::Multisampled { texture, .. }  => Arc::clone(texture),
        }
    }
}