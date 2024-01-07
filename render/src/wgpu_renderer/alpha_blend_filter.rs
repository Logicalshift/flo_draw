use super::texture::*;
use super::pipeline::*;
use super::to_buffer::*;
use super::wgpu_shader::*;

use crate::buffer::*;

use wgpu;

use std::mem;
use std::num::*;
use std::sync::*;

///
/// Performs an alpha-blending render pass on a texture
///
pub (crate) fn alpha_blend(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, alpha_blend_pipeline: &Pipeline, source_texture: &WgpuTexture, alpha: f32) -> WgpuTexture {
    // Ensure we have a suitable pipeline render pass
    debug_assert!(match alpha_blend_pipeline.shader_module { WgpuShader::Filter(FilterShader::AlphaBlend(..)) => true, _ => false }, "alpha_blend must be used with a pipeline configured for alpha blending");

    // Set up buffers
    let vertices = vec![
        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(-1.0, 1.0),
        Vertex2D::with_pos(1.0, 1.0),

        Vertex2D::with_pos(-1.0, -1.0),
        Vertex2D::with_pos(1.0, -1.0),
        Vertex2D::with_pos(1.0, 1.0),
    ].to_buffer(device, wgpu::BufferUsages::VERTEX);

    let alpha = alpha.to_buffer(device, wgpu::BufferUsages::UNIFORM);

    // Create a target texture
    let mut target_descriptor   = source_texture.descriptor.clone();
    target_descriptor.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    let target_texture          = device.create_texture(&target_descriptor);

    // Bind the resources
    let source_view     = source_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let layout          = &*alpha_blend_pipeline.alpha_blend_layout;
    let alpha_binding   = wgpu::BufferBinding {
        buffer: &alpha,
        offset: 0,
        size:   NonZeroU64::new(mem::size_of::<f32>() as u64)
    };
    let alpha_binding = wgpu::BindingResource::Buffer(alpha_binding);

    let filter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label:      Some("alpha_blend"),
        layout:     &layout,
        entries:    &[
            wgpu::BindGroupEntry {
                binding:    0,
                resource:   wgpu::BindingResource::TextureView(&source_view),
            },

            wgpu::BindGroupEntry {
                binding:    1,
                resource:   alpha_binding,
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
            label:                      Some("alpha_blend"),
            depth_stencil_attachment:   None,
            color_attachments:          &color_attachments,
            ..Default::default()
        });

        // Draw the vertices
        let vertex_size = mem::size_of::<Vertex2D>();
        let start_pos   = (0 * vertex_size) as u64;
        let end_pos     = (6 * vertex_size) as u64;

        render_pass.set_pipeline(&*alpha_blend_pipeline.pipeline);
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
