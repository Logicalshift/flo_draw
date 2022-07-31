use super::texture::*;
use super::pipeline::*;
use super::to_buffer::*;
use super::wgpu_shader::*;

use crate::buffer::*;

use wgpu;
use wgpu::util::{DeviceExt};

use std::mem;
use std::num::*;
use std::sync::*;

///
/// Runs tha reduce filter against a texture (designed to filter it to half its current size)
///
pub (crate) fn reduce_filter(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, reduce_pipeline: &Pipeline, source_texture: &WgpuTexture, target_texture: &WgpuTexture, mip_level: u32) {
    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),

        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ].to_buffer(device, wgpu::BufferUsages::VERTEX);

    // Create the reduce sampler
    let reduce_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("reduce_sampler"),
        address_mode_u:     wgpu::AddressMode::ClampToEdge,
        address_mode_v:     wgpu::AddressMode::ClampToEdge,
        address_mode_w:     wgpu::AddressMode::ClampToEdge,
        mag_filter:         wgpu::FilterMode::Linear,
        min_filter:         wgpu::FilterMode::Linear,
        mipmap_filter:      wgpu::FilterMode::Linear,
        lod_min_clamp:      0.0,
        lod_max_clamp:      0.0,
        compare:            None,
        anisotropy_clamp:   None,
        border_color:       None,
    });

    // Bind the resources
    let source_view             = source_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout                  = &*reduce_pipeline.reduce_layout;

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:      Some("reduce_filter"),
        layout:     &layout,
        entries:    &[
            wgpu::BindGroupEntry {
                binding:    0,
                resource:   wgpu::BindingResource::TextureView(&source_view),
            },

            wgpu::BindGroupEntry {
                binding:    1,
                resource:   wgpu::BindingResource::Sampler(&reduce_sampler),
            },
        ]
    });

    // Run a render pass to apply the filter
    {
        let view_descriptor     = wgpu::TextureViewDescriptor {
            label:              Some("reduce_filter"),
            format:             None,
            dimension:          None,
            aspect:             wgpu::TextureAspect::All,
            base_mip_level:     mip_level,
            mip_level_count:    NonZeroU32::new(1),
            base_array_layer:   0,
            array_layer_count:  None
        };
        let target_view         = target_texture.texture.create_view(&view_descriptor);
        let color_attachments   = vec![
            Some(wgpu::RenderPassColorAttachment {
                view:           &target_view,
                resolve_target: None,
                ops:            wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), store: true }
            })
        ];
        let mut render_pass     = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label:                      Some("reduce_filter"),
            depth_stencil_attachment:   None,
            color_attachments:          &color_attachments,
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos   = (0 * vertex_size) as u64;
        let end_pos     = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*reduce_pipeline.pipeline);
        render_pass.set_bind_group(0, &filter_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(start_pos..end_pos));
        render_pass.draw(0..6, 0..1);
    }
}

///
/// Creates a mip-mapped texture from another texture
///
pub (crate) fn create_mipmaps(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, reduce_pipeline: &Pipeline, source_texture: &WgpuTexture) -> WgpuTexture {
    let num_mips = ((source_texture.descriptor.size.width.min(source_texture.descriptor.size.height)) as f32).log2();
    let num_mips = if num_mips < 2.0 { 1.0 } else { num_mips - 1.0 };
    let num_mips = num_mips as u32;

    // Create a target texture
    let mut target_descriptor           = source_texture.descriptor.clone();
    target_descriptor.usage             |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    target_descriptor.mip_level_count   = num_mips;
    let target_texture                  = device.create_texture(&target_descriptor);

    // Copy the top-level of the source texture to the top level of the new texture
    encoder.copy_texture_to_texture(wgpu::ImageCopyTexture {
        texture:    &source_texture.texture,
        mip_level:  0,
        origin:     wgpu::Origin3d::default(),
        aspect:     wgpu::TextureAspect::All
    }, wgpu::ImageCopyTexture {
        texture:    &target_texture,
        mip_level:  0,
        origin:     wgpu::Origin3d::default(),
        aspect:     wgpu::TextureAspect::All
    }, source_texture.descriptor.size);

    // Create the new texture
    let target_texture          = WgpuTexture {
        descriptor:         target_descriptor,
        texture:            Arc::new(target_texture),
        is_premultiplied:   source_texture.is_premultiplied,
    };

    // Reduce the original texture repeatedly to the taget texture
    for mip_level in 1..num_mips {
        reduce_filter(device, encoder, reduce_pipeline, source_texture, &target_texture, mip_level);
    }

    // Return the resulting texture
    target_texture
}
