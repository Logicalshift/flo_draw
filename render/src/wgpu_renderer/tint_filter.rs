use super::texture::*;
use super::pipeline::*;
use super::to_buffer::*;
use super::wgpu_shader::*;

use crate::action::*;
use crate::buffer::*;

use wgpu;

use std::mem;
use std::num::*;
use std::sync::*;

///
/// Performs a tint render pass on a texture
///
pub (crate) fn tint(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, tint_pipeline: &Pipeline, source_texture: &WgpuTexture, color: Rgba8) -> WgpuTexture {
    // Ensure we have a suitable pipeline render pass
    debug_assert!(match tint_pipeline.shader_module { WgpuShader::Filter(FilterShader::Tint(..)) => true, _ => false }, "tint must be used with a pipeline configured for tinting");

    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),

        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ].to_buffer(device, wgpu::BufferUsages::VERTEX);

    let [r, g, b, a] = color.0;
    let r = (r as f32) / 255.0;
    let g = (g as f32) / 255.0;
    let b = (b as f32) / 255.0;
    let a = (a as f32) / 255.0;
    let color = vec![r, g, b, a].to_buffer(device, wgpu::BufferUsages::UNIFORM);

    // Create a target texture
    let mut target_descriptor   = source_texture.descriptor.clone();
    target_descriptor.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    let target_texture          = device.create_texture(&target_descriptor);

    // Bind the resources
    let source_view     = source_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout          = &*tint_pipeline.tint_layout;
    let tint_binding    = wgpu::BufferBinding {
        buffer: &color,
        offset: 0,
        size:   NonZeroU64::new(mem::size_of::<f32>() as u64 * 4)
    };
    let tint_binding = wgpu::BindingResource::Buffer(tint_binding);

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:      Some("tint"),
        layout:     &layout,
        entries:    &[
            wgpu::BindGroupEntry {
                binding:    0,
                resource:   wgpu::BindingResource::TextureView(&source_view),
            },

            wgpu::BindGroupEntry {
                binding:    1,
                resource:   tint_binding,
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
                depth_slice:    None,
            })
        ];
        let mut render_pass     = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label:                      Some("alpha_blend"),
            depth_stencil_attachment:   None,
            color_attachments:          &color_attachments,
            ..Default::default()
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos   = (0 * vertex_size) as u64;
        let end_pos     = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*tint_pipeline.pipeline);
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
