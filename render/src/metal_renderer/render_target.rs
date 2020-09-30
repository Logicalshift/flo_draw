use crate::action::*;

use metal;

///
/// Describes a Metal render target
///
#[derive(Clone)]
pub enum RenderTarget {
    /// Simple texture
    Texture {
        texture:    metal::Texture,
        width:      usize,
        height:     usize
    },

    /// MSAA render target
    Multisampled {
        samples:    metal::Texture,
        resolved:   Option<metal::Texture>,
        width:      usize,
        height:     usize
    }
}

impl RenderTarget {
    ///
    /// Creates a new render target
    ///
    pub fn new(device: &metal::Device, width: usize, height: usize, render_target_type: RenderTargetType) -> RenderTarget {
        // Create the texture descriptor
        let texture_descriptor = metal::TextureDescriptor::new();

        texture_descriptor.set_texture_type(metal::MTLTextureType::D2);
        texture_descriptor.set_width(width as u64);
        texture_descriptor.set_height(height as u64);
        texture_descriptor.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        texture_descriptor.set_usage(metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead);

        // Customise to the render target type
        match render_target_type {
            RenderTargetType::Standard              => { }
            RenderTargetType::StandardForReading    => {
                texture_descriptor.set_pixel_format(metal::MTLPixelFormat::RGBA8Unorm);
            }

            RenderTargetType::Multisampled          |
            RenderTargetType::MultisampledTexture   => { 
                texture_descriptor.set_texture_type(metal::MTLTextureType::D2Multisample);
                texture_descriptor.set_sample_count(4);
                texture_descriptor.set_storage_mode(metal::MTLStorageMode::Private);
            }

            RenderTargetType::Monochrome            => {
                texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
            }

            RenderTargetType::MonochromeMultisampledTexture => {
                texture_descriptor.set_texture_type(metal::MTLTextureType::D2Multisample);
                texture_descriptor.set_pixel_format(metal::MTLPixelFormat::R8Unorm);
                texture_descriptor.set_sample_count(4);
                texture_descriptor.set_storage_mode(metal::MTLStorageMode::Private);
            }
        }

        // Turn into a texture
        let render_texture              = device.new_texture(&texture_descriptor);

        // Create the render target
        match render_target_type {
            RenderTargetType::Standard | RenderTargetType::StandardForReading | RenderTargetType::Monochrome => {
                // Just create a normal texture
                RenderTarget::Texture { 
                    texture:    render_texture,
                    width:      width,
                    height:     height
                }
            },

            RenderTargetType::Multisampled | RenderTargetType::MultisampledTexture   => { 
                RenderTarget::Multisampled {
                    samples:    render_texture,
                    resolved:   None,
                    width:      width,
                    height:     height
                }
            }

            RenderTargetType::MonochromeMultisampledTexture => {
                RenderTarget::Multisampled {
                    samples:    render_texture,
                    resolved:   None,
                    width:      width,
                    height:     height
                }
            }
        }
    }

    ///
    /// Returns the width and height of this render target
    ///
    pub fn size(&self) -> (usize, usize) {
        match self {
            RenderTarget::Texture { texture: _, width, height }                     => (*width, *height),
            RenderTarget::Multisampled { samples: _, resolved: _, width, height }   => (*width, *height)
        }
    }

    ///
    /// Returns the texture that is used as the render target
    ///
    pub fn render_texture(&self) -> &metal::Texture {
        match self {
            RenderTarget::Texture { texture, width: _, height: _ }                      => texture,
            RenderTarget::Multisampled { samples, resolved: _, width: _, height: _ }    => samples
        }
    }

    ///
    /// True if this is a multisampled render target
    ///
    pub fn is_multisampled(&self) -> bool {
        match self {
            RenderTarget::Texture { texture: _, width: _, height: _ }                   => false,
            RenderTarget::Multisampled { samples: _, width: _, height: _, resolved: _ } => true
        }
    }
}
