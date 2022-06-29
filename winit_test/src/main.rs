use winit::window;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use wgpu;

use futures::executor;

use std::borrow::{Cow};

const SHADER: &'static str = "
[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] idx: u32) -> [[builtin(position)]] vec4<f32> {
    let x = f32(i32(idx) - 1);
    let y = f32(i32(idx & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
";

fn main() {
    // Set up an event loop and a window that reports to it
    let event_loop  = EventLoop::new();
    let window      = window::Window::new(&event_loop).unwrap();

    // Bits of wgpu are async so we need an async blocker here
    executor::block_on(async move {
        // Create a new WGPU instance, surface and adapter
        let instance    = wgpu::Instance::new(wgpu::Backends::all());
        let surface     = unsafe { instance.create_surface(&window) };
        let adapter     = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference:       wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface:     Some(&surface),
        }).await.unwrap();

        // Fetch the device and the queue
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label:      None,
            features:   wgpu::Features::empty(),
            limits:     wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        }, None).await.unwrap();

        // Load the shader
        let shader              = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label:  None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER))
        });

        // Create the render pipeline
        let swapchain_format    = surface.get_preferred_format(&adapter).unwrap();
        let pipeline_layout     = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:                  None,
            bind_group_layouts:     &[],
            push_constant_ranges:   &[],
        });

        let render_pipeline     = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:          None,
            layout:         Some(&pipeline_layout),
            vertex:         wgpu::VertexState {
                module:         &shader,
                entry_point:    "vs_main",
                buffers:        &[],
            },
            fragment:       Some(wgpu::FragmentState {
                module:         &shader,
                entry_point:    "fs_main",
                targets:        &[swapchain_format.into()],
            }),
            primitive:      wgpu::PrimitiveState::default(),
            depth_stencil:  None,
            multisample:    wgpu::MultisampleState::default(),
            multiview:      None,
        });

        // Surface configuration
        let size                = window.inner_size();
        let mut surface_config  = wgpu::SurfaceConfiguration {
            usage:          wgpu::TextureUsages::RENDER_ATTACHMENT,
            format:         swapchain_format,
            width:          size.width,
            height:         size.height,
            present_mode:   wgpu::PresentMode::Mailbox,
        };

        surface.configure(&device, &surface_config);

        // Run the main event loop (which is not async)
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => { 
                    *control_flow = ControlFlow::Exit;
                }

                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    // Configure the surface to the new size
                    surface_config.width    = size.width;
                    surface_config.height   = size.height;
                    surface.configure(&device, &surface_config);

                    // Start a frame
                    let frame   = surface.get_current_texture().unwrap();
                    let view    = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                    // Encoder to send commands
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    // Render the triangle
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label:                      None,
                            color_attachments:          &[wgpu::RenderPassColorAttachment {
                                view:           &view,
                                resolve_target: None,
                                ops:            wgpu::Operations {
                                    load:   wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                    store:  true,
                                },
                            }],
                            depth_stencil_attachment:   None,
                        });

                        render_pass.set_pipeline(&render_pipeline);
                        render_pass.draw(0..3, 0..1);
                    }

                    // Present the triangle to the screen
                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }

                Event::RedrawRequested(_)   => {
                    // Start a frame
                    let frame   = surface.get_current_texture().unwrap();
                    let view    = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                    // Encoder to send commands
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    // Render the triangle
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label:                      None,
                            color_attachments:          &[wgpu::RenderPassColorAttachment {
                                view:           &view,
                                resolve_target: None,
                                ops:            wgpu::Operations {
                                    load:   wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                    store:  true,
                                },
                            }],
                            depth_stencil_attachment:   None,
                        });

                        render_pass.set_pipeline(&render_pipeline);
                        render_pass.draw(0..3, 0..1);
                    }

                    // Present the triangle to the screen
                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }

                _ => {}
            }
        });
    });
}
