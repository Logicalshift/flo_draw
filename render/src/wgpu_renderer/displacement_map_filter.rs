use super::texture::*;
use super::pipeline::*;
use super::to_buffer::*;

use crate::buffer::*;

use wgpu;

use std::mem;
use std::num::*;
use std::sync::*;

///
/// Runs tha displacement map filter against a texture
///
pub (crate) fn displacement_map(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, displacement_pipeline: &Pipeline, source_texture: &WgpuTexture, displacement_texture: &WgpuTexture, scale_factors: (f32, f32)) -> WgpuTexture {
    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),

        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ].to_buffer(device, wgpu::BufferUsages::VERTEX);

    // Create a target texture
    let mut target_descriptor   = source_texture.descriptor.clone();
    target_descriptor.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    let target_texture          = device.create_texture(&target_descriptor);

    // Create the displacement map sampler
    let displacement_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("displacement_sampler"),
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
    let displacement_view       = displacement_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout                  = &*displacement_pipeline.displacement_map_layout;
    let scale_buffer            = vec![scale_factors.0, scale_factors.1];
    let scale_buffer            = scale_buffer.to_buffer(device, wgpu::BufferUsages::UNIFORM);
    let scale_binding           = wgpu::BufferBinding {
        buffer: &scale_buffer,
        offset: 0,
        size:   NonZeroU64::new(8)
    };
    let scale_binding           = wgpu::BindingResource::Buffer(scale_binding);

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:      Some("displacement_map"),
        layout:     &layout,
        entries:    &[
            wgpu::BindGroupEntry {
                binding:    0,
                resource:   wgpu::BindingResource::TextureView(&source_view),
            },

            wgpu::BindGroupEntry {
                binding:    1,
                resource:   wgpu::BindingResource::TextureView(&displacement_view),
            },

            wgpu::BindGroupEntry {
                binding:    2,
                resource:   wgpu::BindingResource::Sampler(&displacement_sampler),
            },

            wgpu::BindGroupEntry {
                binding:    3,
                resource:   scale_binding,
            },
        ]
    });

    // Run a render pass to apply the filter
    {
        let target_view         = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let color_attachments   = vec![
            Some(wgpu::RenderPassColorAttachment {
                view:           &target_view,
                resolve_target: None,
                ops:            wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), store: true }
            })
        ];
        let mut render_pass     = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label:                      Some("displacement_map"),
            depth_stencil_attachment:   None,
            color_attachments:          &color_attachments,
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos   = (0 * vertex_size) as u64;
        let end_pos     = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*displacement_pipeline.pipeline);
        render_pass.set_bind_group(0, &filter_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(start_pos..end_pos));
        render_pass.draw(0..6, 0..1);
    }

    // Result is the new texture
    WgpuTexture {
        descriptor:         target_descriptor,
        texture:            Arc::new(target_texture),
        is_premultiplied:   source_texture.is_premultiplied,
    }
}
