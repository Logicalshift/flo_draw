use crate::draw::*;
use crate::color::*;
use crate::context::*;
use crate::draw_stream::*;

use std::collections::{HashSet};
use std::sync::*;
use std::mem;
use std::iter;

use desync::{Desync};
use futures::{Stream};
use futures::task::{Waker};

///
/// The core of the canvas data structure
///
struct CanvasCore {
    /// The main core contains the drawing instructions in this canvas: while DrawStreamCore is usually used for streaming
    /// it can also be used to store the actions long-term (where the features that strip out unused actions and resources
    /// are particularly useful)
    main_core: DrawStreamCore,

    /// Each stream created from the canvas has its own core (weak so we don't track the stream after it's been dropped)
    streams: Vec<Weak<Desync<DrawStreamCore>>>
}

impl CanvasCore {
    ///
    /// Writes to the canvas core
    ///
    pub fn write(&mut self, actions: Vec<Draw>) -> Vec<Waker> {
        // Write to the main core
        self.main_core.write(actions.iter().cloned());

        // Write to each of the streams
        let mut remove_idx  = vec![];
        let mut wakers      = vec![];

        for (idx, stream) in self.streams.iter().enumerate() {
            if let Some(stream) = stream.upgrade() {
                wakers.push(stream.sync(|stream| {
                    stream.write(iter::once(Draw::StartFrame));
                    stream.write(actions.iter().cloned());
                    stream.write(iter::once(Draw::ShowFrame));
                    stream.take_waker()
                }));
            } else {
                remove_idx.push(idx);
            }
        }

        // Tidy any streams that are no longer listening
        if remove_idx.len() > 0 {
            let remove_idx  = remove_idx.into_iter().collect::<HashSet<_>>();
            let old_streams = mem::take(&mut self.streams);

            self.streams    = old_streams.into_iter()
                .enumerate()
                .filter(|(idx, _item)| !remove_idx.contains(idx))
                .map(|(_idx, item)| item)
                .collect();
        }

        // Return the wakers
        wakers.into_iter().flatten().collect()
    }
}

///
/// A canvas is an abstract interface for drawing graphics. It doesn't actually provide a means to
/// render anything, but rather a way to describe how things should be drawn and pass those on to
/// a renderer elsewhere.
///
/// A canvas can be cloned and sent between threads, so it's possible for multiple sources to write
/// to the same drawing target.
///
/// Canvases maintain a copy of enough of the drawing instructions sent to them to reproduce the
/// rendering on a new render target. 
///
pub struct Canvas {
    /// The canvas represents its own data using a draw stream core that's never used to generate a stream
    core: Arc<Desync<CanvasCore>>
}

impl Canvas {
    ///
    /// Creates a new, blank, canvas
    ///
    pub fn new() -> Canvas {
        // A canvas is initially just a clear command
        let mut core = CanvasCore {
            main_core:  DrawStreamCore::new(),
            streams:    vec![]
        };

        core.main_core.add_usage();
        core.main_core.write(vec![
            Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))
        ].into_iter());

        Canvas {
            core: Arc::new(Desync::new(core))
        }
    }

    ///
    /// Sends some new drawing commands to this canvas
    ///
    pub fn write(&self, to_draw: Vec<Draw>) {
        // Only draw if there are any drawing commands
        if to_draw.len() != 0 {
            let wakers = self.core.sync(move |core| core.write(to_draw));
            wakers.into_iter().for_each(|waker| waker.wake());
        }
    }

    ///
    /// Provides a way to draw on this canvas via a GC
    ///
    pub fn draw<FnAction>(&self, action: FnAction)
    where FnAction: Send+FnOnce(&mut CanvasGraphicsContext) -> () {
        self.core.sync(move |core| {
            let mut graphics_context = CanvasGraphicsContext {
                core:       core,
                pending:    vec![]
            };

            action(&mut graphics_context);
        })
    }

    ///
    /// Creates a stream for reading the instructions from this canvas
    ///
    pub fn stream(&self) -> impl Stream<Item=Draw>+Send {
        // Create a new canvas stream
        let new_core    = Arc::new(Desync::new(DrawStreamCore::new()));
        let new_stream  = DrawStream::with_core(&new_core);

        // Register it and send the current set of pending commands to it
        let add_stream = Arc::clone(&new_core);
        self.core.desync(move |core| {
            // Send the data we've received since the last clear
            add_stream.sync(|stream| {
                stream.write(iter::once(Draw::ResetFrame));
                stream.write(core.main_core.get_pending_drawing())
            });

            // Store the stream in the core so future notifications get sent there
            core.streams.push(Arc::downgrade(&add_stream));

            // Wake the stream if it's not awake
            add_stream.sync(|stream| stream.take_waker().map(|waker| waker.wake()));
        });

        // Return the new stream
        new_stream
    }

    ///
    /// Retrieves the list of drawing actions in this canvas
    ///
    pub fn get_drawing(&self) -> Vec<Draw> {
        self.core.sync(|core| core.main_core.get_pending_drawing().collect())
    }
}

impl Clone for Canvas {
    fn clone(&self) -> Canvas {
        self.core.desync(|core| core.main_core.add_usage());

        Canvas {
            core: self.core.clone()
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // The streams drop if this is the last canvas with this core
        self.core.sync(|core| {
            if core.main_core.finish_usage() == 0 {
                // Close all the streams and then wake them up
                core.streams.drain(..)
                    .map(|stream| {
                        if let Some(stream) = stream.upgrade() {
                            stream.sync(|stream| { stream.close(); stream.take_waker() })
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .for_each(|waker| waker.wake());
            }
        });
    }
}

///
/// Graphics context for a Canvas
///
pub struct CanvasGraphicsContext<'a> {
    core:       &'a mut CanvasCore,
    pending:    Vec<Draw>
}

impl<'a> GraphicsContext for CanvasGraphicsContext<'a> {
    fn draw(&mut self, d: Draw)                                 { self.pending.push(d); }
}

impl<'a> Drop for CanvasGraphicsContext<'a> {
    fn drop(&mut self) {
        let wakers = self.core.write(mem::take(&mut self.pending));
        wakers.into_iter().for_each(|waker| waker.wake());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::path::*;
    use crate::font::*;
    use crate::sprite::*;
    use crate::gradient::*;
    use crate::font_face::*;
    use crate::primitives::*;
    use crate::transform2d::*;

    use futures::prelude::*;
    use futures::executor;

    use std::thread::*;
    use std::time::*;

    #[test]
    fn can_draw_to_canvas() {
        let canvas = Canvas::new();

        canvas.write(vec![Draw::Path(PathOp::NewPath)]);
    }

    #[test]
    fn can_follow_canvas_stream() {
        let canvas      = Canvas::new();
        let mut stream  = canvas.stream();

        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            canvas.write(vec![
                Draw::Path(PathOp::NewPath),
                Draw::Path(PathOp::Move(0.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 10.0)),
                Draw::Path(PathOp::Line(0.0, 10.0))
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        // Check we can get the results via the stream
        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));
            assert!(stream.next().await == Some(Draw::ShowFrame));

            // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
            assert!(stream.next().await == None);
        })
    }

    #[test]
    fn can_draw_using_gc() {
        let canvas      = Canvas::new();
        let mut stream  = canvas.stream();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);
        });

        // Check we can get the results via the stream
        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));
            assert!(stream.next().await == Some(Draw::ShowFrame));
        });
    }

    #[test]
    fn restore_rewinds_canvas() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.store();
            gc.new_path();
            gc.rect(0.0,0.0, 100.0,100.0);
            gc.restore();

            gc.stroke();
        });

        // Only the commands before the 'store' should be present
        let mut stream  = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            // 'Store' is still present as we can restore the same thing repeatedly
            assert!(stream.next().await == Some(Draw::Store));

            assert!(stream.next().await == Some(Draw::Stroke));
        })
    }

    #[test]
    fn free_stored_buffer_after_restore() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.store();
            gc.new_path();
            gc.rect(0.0,0.0, 100.0,100.0);
            gc.restore();
            gc.free_stored_buffer();

            gc.stroke();
        });

        // Only the commands before the 'store' should be present
        let mut stream  = canvas.stream();

        executor::block_on(async
        {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            assert!(stream.next().await == Some(Draw::Stroke));
        })
    }

    #[test]
    fn clip_interrupts_rewind() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.store();
            gc.clip();
            gc.new_path();
            gc.restore();
        });

        // Only the commands before the 'store' should be present
        let mut stream  = canvas.stream();
        mem::drop(canvas);

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            assert!(stream.next().await == Some(Draw::Store));
            assert!(stream.next().await == Some(Draw::Clip));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Restore));
        })
    }

    #[test]
    fn can_follow_many_streams() {
        let canvas      = Canvas::new();
        let mut stream  = canvas.stream();
        let mut stream2 = canvas.stream();

        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            canvas.write(vec![
                Draw::Path(PathOp::NewPath),
                Draw::Path(PathOp::Move(0.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 10.0)),
                Draw::Path(PathOp::Line(0.0, 10.0))
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        executor::block_on(async {
            // Check we can get the results via the stream
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));

            assert!(stream2.next().await == Some(Draw::ResetFrame));
            assert!(stream2.next().await == Some(Draw::StartFrame));
            assert!(stream2.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream2.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream2.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));

            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            assert!(stream2.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream2.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream2.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
            assert!(stream.next().await == Some(Draw::ShowFrame));
            assert!(stream2.next().await == Some(Draw::ShowFrame));

            assert!(stream.next().await == None);
            assert!(stream2.next().await == None);
        });
    }

    #[test]
    fn commands_after_clear_are_suppressed() {
        let canvas      = Canvas::new();
        let mut stream  = canvas.stream();

        // Thread to draw some stuff to the canvas
        spawn(move || {
            canvas.write(vec![
                Draw::Path(PathOp::NewPath),
                Draw::Path(PathOp::Move(0.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 10.0)),
                Draw::Path(PathOp::Line(0.0, 10.0))
            ]);

            // Enough time that we read the first few commands
            sleep(Duration::from_millis(100));

            canvas.write(vec![
                Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0)),
                Draw::Path(PathOp::Move(200.0, 200.0)),
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/
        executor::block_on(async {
            // Give the thread some time to clear the canvas
            sleep(Duration::from_millis(200));

            // Should immediately stop the old frame and start a new one
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::StartFrame));

            // Commands we sent before the flush are gone
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 1.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(200.0, 200.0))));
            assert!(stream.next().await == Some(Draw::ShowFrame));

            // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
            assert!(stream.next().await == None);
        })
    }

    #[test]
    fn clear_layer_0_removes_commands() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.stroke();
            gc.clear_layer();

            gc.new_path();
            gc.move_to(10.0, 10.0);
            gc.fill();
        });

        // Only the commands after clear_layer should be present
        let mut stream  = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_only_removes_commands_for_the_current_layer() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(20.0, 20.0);

            gc.stroke();

            gc.layer(LayerId(1));
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.clear_layer();

            gc.new_path();
            gc.move_to(10.0, 10.0);
            gc.fill();
        });

        // Only the commands after clear_layer should be present
        let mut stream  = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(20.0, 20.0))));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_does_not_clear_sprites() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(20.0, 20.0);

            gc.stroke();

            gc.layer(LayerId(1));
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.sprite(SpriteId(1));
            gc.clear_sprite();

            gc.new_path();
            gc.move_to(10.0, 10.0);
            gc.fill();

            gc.layer(LayerId(1));
            gc.clear_layer();

            gc.fill();
        });

        // Only the commands after clear_layer should be present
        let mut stream  = canvas.stream();
        println!("{:?}", canvas.get_drawing());

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(20.0, 20.0))));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(1))));
            assert!(stream.next().await == Some(Draw::ClearSprite));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_removes_pushed_transforms() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.transform(Transform2D::identity());

            gc.push_state();
            gc.transform(Transform2D::scale(10.0, 10.0));
            gc.pop_state();

            gc.clear_layer();
        });

        // Only the commands after clear_layer should be present
        let mut stream  = canvas.stream();
        println!("{:?}", canvas.get_drawing());

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::MultiplyTransform(Transform2D::identity())));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
        });
    }

    #[test]
    fn pushed_transforms_kept_in_order() {
        let canvas      = Canvas::new();

        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.transform(Transform2D::identity());

            gc.push_state();
            gc.transform(Transform2D::scale(10.0, 10.0));

            gc.clear_layer();
        });

        // Only the commands after clear_layer should be present
        let mut stream  = canvas.stream();
        println!("{:?}", canvas.get_drawing());

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::MultiplyTransform(Transform2D::identity())));
            assert!(stream.next().await == Some(Draw::PushState));
            assert!(stream.next().await == Some(Draw::MultiplyTransform(Transform2D::scale(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
        });
    }

    #[test]
    fn font_definitions_survive_clear_layer() {
        let canvas  = Canvas::new();
        let lato    = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.define_font_data(FontId(1), lato.clone());
            gc.set_font_size(FontId(1), 12.0);
            gc.draw_text(FontId(1), "Test".to_string(), 100.0, 100.0);

            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(match stream.next().await { Some(Draw::Font(FontId(1), FontOp::UseFontDefinition(_))) => true, _ => false });
            assert!(stream.next().await == Some(Draw::Font(FontId(1), FontOp::FontSize(12.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn only_one_font_definition_survives_clear_layer() {
        let canvas  = Canvas::new();
        let lato    = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.define_font_data(FontId(1), lato.clone());
            gc.define_font_data(FontId(1), lato.clone());
            gc.define_font_data(FontId(2), lato.clone());
            gc.define_font_data(FontId(1), lato.clone());
            gc.set_font_size(FontId(1), 12.0);
            gc.draw_text(FontId(1), "Test".to_string(), 100.0, 100.0);

            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(match stream.next().await { Some(Draw::Font(FontId(2), FontOp::UseFontDefinition(_))) => true, _ => false });
            assert!(match stream.next().await { Some(Draw::Font(FontId(1), FontOp::UseFontDefinition(_))) => true, _ => false });
            assert!(stream.next().await == Some(Draw::Font(FontId(1), FontOp::FontSize(12.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn only_one_font_size_survives_clear_layer() {
        let canvas  = Canvas::new();
        let lato    = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.define_font_data(FontId(1), lato.clone());
            gc.set_font_size(FontId(1), 16.0);
            gc.set_font_size(FontId(1), 15.0);
            gc.set_font_size(FontId(2), 18.0);
            gc.set_font_size(FontId(1), 14.0);
            gc.set_font_size(FontId(1), 13.0);
            gc.set_font_size(FontId(1), 12.0);
            gc.draw_text(FontId(1), "Test".to_string(), 100.0, 100.0);

            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(match stream.next().await { Some(Draw::Font(FontId(1), FontOp::UseFontDefinition(_))) => true, _ => false });
            assert!(stream.next().await == Some(Draw::Font(FontId(2), FontOp::FontSize(18.0))));
            assert!(stream.next().await == Some(Draw::Font(FontId(1), FontOp::FontSize(12.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn show_start_frames_cancel_out() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));

            gc.start_frame();
            gc.new_path();
            gc.start_frame();
            gc.move_to(20.0, 20.0);

            gc.start_frame();
            gc.stroke();

            gc.start_frame();
            gc.layer(LayerId(1));
            gc.start_frame();
            gc.new_path();
            gc.start_frame();
            gc.move_to(0.0, 0.0);
            gc.start_frame();
            gc.line_to(10.0, 0.0);
            gc.start_frame();
            gc.line_to(10.0, 10.0);
            gc.start_frame();
            gc.line_to(0.0, 10.0);

            gc.start_frame();
            gc.clear_layer();

            gc.start_frame();
            gc.new_path();
            gc.start_frame();
            gc.move_to(10.0, 10.0);
            gc.start_frame();
            gc.fill();

            // Cancel all but one of the start frames
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
            gc.show_frame();
        });

        let mut stream = canvas.stream();

        // Only the one uncanceled start_frame should be in the canvas
        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(20.0, 20.0))));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::ShowFrame));
        });
    }

    #[test]
    fn gradient_definitions_survive_clear_layer() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::Create(Color::Rgba(0.0, 0.1, 0.2, 0.3)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0)))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn only_one_gradient_definition_survives_clear_layer() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
            gc.layer(LayerId(1));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::Create(Color::Rgba(0.0, 0.1, 0.2, 0.3)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0)))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn only_one_gradient_definition_survives_clear_other_layer() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
            gc.layer(LayerId(0));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.layer(LayerId(1));
            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::Create(Color::Rgba(0.0, 0.1, 0.2, 0.3)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0)))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn used_gradient_survives_clear_other_layer() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
            gc.layer(LayerId(0));

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.fill_gradient(GradientId(2), 0.0, 1.0, 2.0, 3.0);

            gc.create_gradient(GradientId(2), Color::Rgba(0.0, 0.1, 0.2, 0.3));
            gc.gradient_stop(GradientId(2), 0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0));
            gc.gradient_stop(GradientId(2), 1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0));

            gc.layer(LayerId(1));
            gc.clear_layer();
            gc.fill();
        });

        let mut stream = canvas.stream();

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));

            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::Create(Color::Rgba(0.0, 0.1, 0.2, 0.3)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0)))));

            assert!(stream.next().await == Some(Draw::FillGradient(GradientId(2), (0.0, 1.0), (2.0, 3.0))));

            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::Create(Color::Rgba(0.0, 0.1, 0.2, 0.3)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(0.5, Color::Rgba(0.4, 0.5, 0.6, 1.0)))));
            assert!(stream.next().await == Some(Draw::Gradient(GradientId(2), GradientOp::AddStop(1.0, Color::Rgba(0.7, 0.8, 0.9, 1.0)))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }
}
