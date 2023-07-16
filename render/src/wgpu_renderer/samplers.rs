use wgpu;

use std::sync::*;

///
/// The samplers used by the WGPU renderer
///
pub (crate) struct Samplers {
    /// The default sampler used when no others are in effect
    default_sampler: Arc<wgpu::Sampler>,

    /// Sampler that doesn't repeat
    non_repeating_sampler: Arc<wgpu::Sampler>,

    /// The sampler used for rendering gradients
    gradient_sampler: Arc<wgpu::Sampler>,

    /// The sampler used for rendering gradients, which clamps rather than repeating
    non_repeating_gradient_sampler: Arc<wgpu::Sampler>,
}

impl Samplers {
    ///
    /// Creates the samplers for a device
    ///
    pub (crate) fn new(device: &wgpu::Device) -> Samplers {
        let default_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u:     wgpu::AddressMode::Repeat,
            address_mode_v:     wgpu::AddressMode::Repeat,
            address_mode_w:     wgpu::AddressMode::Repeat,
            mag_filter:         wgpu::FilterMode::Linear,
            min_filter:         wgpu::FilterMode::Linear,
            mipmap_filter:      wgpu::FilterMode::Linear,
            lod_min_clamp:      0.0,
            lod_max_clamp:      8.0,
            compare:            None,
            anisotropy_clamp:   1,
            border_color:       None,
        });

        let non_repeating_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u:     wgpu::AddressMode::ClampToEdge,
            address_mode_v:     wgpu::AddressMode::ClampToEdge,
            address_mode_w:     wgpu::AddressMode::ClampToEdge,
            mag_filter:         wgpu::FilterMode::Linear,
            min_filter:         wgpu::FilterMode::Linear,
            mipmap_filter:      wgpu::FilterMode::Linear,
            lod_min_clamp:      0.0,
            lod_max_clamp:      8.0,
            compare:            None,
            anisotropy_clamp:   1,
            border_color:       None,
        });

        let gradient_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gradient_sampler"),
            address_mode_u:     wgpu::AddressMode::MirrorRepeat,
            address_mode_v:     wgpu::AddressMode::MirrorRepeat,
            address_mode_w:     wgpu::AddressMode::MirrorRepeat,
            mag_filter:         wgpu::FilterMode::Linear,
            min_filter:         wgpu::FilterMode::Linear,
            mipmap_filter:      wgpu::FilterMode::Linear,
            lod_min_clamp:      0.0,
            lod_max_clamp:      0.0,
            compare:            None,
            anisotropy_clamp:   1,
            border_color:       None,
        });

        let non_repeating_gradient_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gradient_sampler"),
            address_mode_u:     wgpu::AddressMode::ClampToEdge,
            address_mode_v:     wgpu::AddressMode::ClampToEdge,
            address_mode_w:     wgpu::AddressMode::ClampToEdge,
            mag_filter:         wgpu::FilterMode::Linear,
            min_filter:         wgpu::FilterMode::Linear,
            mipmap_filter:      wgpu::FilterMode::Linear,
            lod_min_clamp:      0.0,
            lod_max_clamp:      0.0,
            compare:            None,
            anisotropy_clamp:   1,
            border_color:       None,
        });

        Samplers {
            default_sampler:                Arc::new(default_sampler),
            non_repeating_sampler:          Arc::new(non_repeating_sampler),
            gradient_sampler:               Arc::new(gradient_sampler),
            non_repeating_gradient_sampler: Arc::new(non_repeating_gradient_sampler),
        }
    }

    #[inline] pub fn default_sampler(&self) -> Arc<wgpu::Sampler> {
        Arc::clone(&self.default_sampler)
    } 

    #[inline] pub fn non_repeating_sampler(&self) -> Arc<wgpu::Sampler> {
        Arc::clone(&self.non_repeating_sampler)
    } 

    #[inline] pub fn gradient_sampler(&self) -> Arc<wgpu::Sampler> {
        Arc::clone(&self.gradient_sampler)
    } 

    #[inline] pub fn non_repeating_gradient_sampler(&self) -> Arc<wgpu::Sampler> {
        Arc::clone(&self.non_repeating_gradient_sampler)
    } 
}
