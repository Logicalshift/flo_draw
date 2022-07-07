use flo_render::*;

use winit::window;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use wgpu;

use futures::executor;

use std::sync::*;

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

        // Create the WGPU renderer
        let device          = Arc::new(device);
        let queue           = Arc::new(queue);
        let surface         = Arc::new(surface);
        let adapter         = Arc::new(adapter);
        let mut renderer    = WgpuRenderer::new(Arc::clone(&device), Arc::clone(&queue), Arc::clone(&surface), Arc::clone(&adapter));

        // Surface configuration
        let size                = window.inner_size();

        renderer.prepare_to_render(size.width, size.height);

        // Run the main event loop (which is not async)
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => { 
                    *control_flow = ControlFlow::Exit;
                }

                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    use RenderAction::*;

                    // Configure the surface to the new size
                    renderer.prepare_to_render(size.width, size.height);

                    renderer.render_to_surface(vec![
                        RenderToFrameBuffer,

                        Clear(Rgba8([255, 255, 255, 255])),
                        CreateVertex2DBuffer(VertexBufferId(0), vec![
                            Vertex2D::with_pos(-0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                            Vertex2D::with_pos(-0.0, 0.5).with_color(1.0, 0.0, 0.0, 1.0),
                            Vertex2D::with_pos(0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                        ]),
                        DrawTriangles(VertexBufferId(0), 0..3),

                        ShowFrameBuffer
                    ]);

                    surface.get_current_texture().unwrap().present();
                }

                Event::RedrawRequested(_)   => {
                    use RenderAction::*;

                    renderer.render_to_surface(vec![
                        RenderToFrameBuffer,

                        Clear(Rgba8([255, 255, 255, 255])),
                        CreateVertex2DBuffer(VertexBufferId(0), vec![
                            Vertex2D::with_pos(-0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                            Vertex2D::with_pos(-0.0, 0.5).with_color(1.0, 0.0, 0.0, 1.0),
                            Vertex2D::with_pos(0.5, -0.5).with_color(1.0, 0.0, 0.0, 1.0),
                        ]),
                        DrawTriangles(VertexBufferId(0), 0..3),

                        ShowFrameBuffer
                    ]);

                    surface.get_current_texture().unwrap().present();
                }

                _ => {}
            }
        });
    });
}
