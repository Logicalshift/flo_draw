use super::texture::*;
use super::pipeline::*;
use super::to_buffer::*;

use crate::buffer::*;

use wgpu;

use std::mem;
use std::sync::*;

///
/// Runs tha mask filter against a texture
///
pub (crate) fn mask(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, mask_pipeline: &Pipeline, source_texture: &WgpuTexture, mask_texture: &WgpuTexture) -> WgpuTexture {
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

    // Create the masking sampler
    let mask_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("mask_sampler"),
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

    // Bind the resources
    let source_view             = source_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mask_view               = mask_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout                  = &*mask_pipeline.mask_layout;

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:      Some("mask"),
        layout:     &layout,
        entries:    &[
            wgpu::BindGroupEntry {
                binding:    0,
                resource:   wgpu::BindingResource::TextureView(&source_view),
            },

            wgpu::BindGroupEntry {
                binding:    1,
                resource:   wgpu::BindingResource::TextureView(&mask_view),
            },

            wgpu::BindGroupEntry {
                binding:    2,
                resource:   wgpu::BindingResource::Sampler(&mask_sampler),
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
                ops:            wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), store: wgpu::StoreOp::Store },
            })
        ];
        let mut render_pass     = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label:                      Some("mask"),
            depth_stencil_attachment:   None,
            color_attachments:          &color_attachments,
            ..Default::default()
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos   = (0 * vertex_size) as u64;
        let end_pos     = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*mask_pipeline.pipeline);
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
