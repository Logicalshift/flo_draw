use crate::draw::*;
use crate::draw_stream::*;

use ::desync::*;
use futures::prelude::*;

use std::sync::*;

type DrawGraphicsContext = Vec<Draw>;

///
/// A drawing target sends drawing instructions to a `DrawStream`
///
/// `flo_draw` provides two structures for sending drawing instructions to other part of the application. `DrawingTarget`
/// is used when the instructions do not need to be retained: eg, when rendering to a window or to an offscreen target.
///
/// See `Canvas` for a structure that can store drawing instructions as well as send them to a target.
///
pub struct DrawingTarget {
    /// The stream core is where drawing instructions will be sent to
    stream_core: Arc<Desync<DrawStreamCore>>,
}

impl DrawingTarget {
    ///
    /// Creates a new drawing target and a stream that can be used to read the instructions sent to it
    ///
    pub fn new() -> (DrawingTarget, DrawStream) {
        // Create the core
        let core    = Arc::new(Desync::new(DrawStreamCore::new()));
        core.desync(|core| core.add_usage());

        // Create the stream
        let stream  = DrawStream::with_core(&core);

        // Create the context
        let context = DrawingTarget {
            stream_core: core
        };

        (context, stream)
    }

    ///
    /// Sends some drawing instructions to this target
    ///
    pub fn write<Drawing: Send+IntoIterator<Item=Draw>>(&self, drawing: Drawing) {
        // Write the drawing instructions to the pending queue
        let waker = self.stream_core.sync(move |core| {
            core.write(drawing.into_iter());
            core.take_waker()
        });

        // Wake the stream, if anything is listening
        waker.map(|waker| waker.wake());
    }

    ///
    /// Provides a way to draw on this target via a graphics context
    ///
    pub fn draw<FnAction>(&self, action: FnAction)
    where FnAction: Send+FnOnce(&mut DrawGraphicsContext) -> () {
        // Fill a buffer with the drawing actions
        let mut draw_actions = vec![];
        action(&mut draw_actions);

        draw_actions.insert(0, Draw::StartFrame);
        draw_actions.push(Draw::ShowFrame);

        // Send the actions to the core
        self.write(draw_actions);
    }

    ///
    /// Sends the results of a future to this target
    ///
    pub fn receive<DrawStream: Unpin+Stream<Item=Draw>>(self, actions: DrawStream) -> impl Future {
        async move {
            let mut actions = actions.ready_chunks(1000);
            let target      = self;

            while let Some(drawing) = actions.next().await {
                target.write(drawing);
            }
        }
    }
}

///
/// A drawing context can be cloned in order to create multiple sources for a single drawing target.
///
/// This is particularly useful when combined with layers: multiple threads can draw to different layers
/// without interfering with each other, so it's possible to design renderers where the rendering
/// instructions have multiple sources (see the mandelbrot example for an example of where this is used)
///
impl Clone for DrawingTarget {
    fn clone(&self) -> DrawingTarget {
        let new_core = Arc::clone(&self.stream_core);
        new_core.desync(|core| core.add_usage());
        DrawingTarget {
            stream_core: new_core
        }
    }
}

impl Drop for DrawingTarget {
    fn drop(&mut self) {
        let waker = self.stream_core.sync(|core| { 
            if core.finish_usage() == 0 {
                core.close(); 
                core.take_waker() 
            } else {
                None
            }
        });
        waker.map(|waker| waker.wake());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::path::*;
    use crate::font::*;
    use crate::color::*;
    use crate::sprite::*;
    use crate::context::*;
    use crate::font_face::*;

    use futures::executor;

    use std::thread::*;
    use std::time::*;
    use std::mem;

    #[test]
    fn follow_drawing_context_stream() {
        let (context, stream) = DrawingTarget::new();

        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            context.write(vec![
                Draw::Path(PathOp::NewPath),
                Draw::Path(PathOp::Move(0.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 0.0)),
                Draw::Path(PathOp::Line(10.0, 10.0)),
                Draw::Path(PathOp::Line(0.0, 10.0))
            ]);
        });

        // Check we can get the results via the stream
        executor::block_on(async {
            let mut stream = stream;

            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Line(0.0, 10.0))));

            // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
            assert!(stream.next().await == None);
        })
    }

    #[test]
    fn clear_layer_0_removes_commands() {
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
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
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_0_leaves_clear_canvas() {
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));
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
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(10.0, 10.0))));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_all_layers_leaves_state() {
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
            gc.clear_canvas(Color::Rgba(0.0, 0.0, 0.0, 0.0));

            gc.layer(LayerId(2));

            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);

            gc.stroke();
            gc.clear_layer();

            gc.new_path();
            gc.move_to(10.0, 10.0);
            gc.fill_color(Color::Rgba(0.1, 0.2, 0.3, 0.4));
            gc.fill();

            gc.clear_all_layers();
        });

        // Only the commands after clear_layer should be present
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::FillColor(Color::Rgba(0.1, 0.2, 0.3, 0.4))));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(2))));
            assert!(stream.next().await == Some(Draw::ClearAllLayers));
            assert!(stream.next().await == Some(Draw::ShowFrame));
        });
    }

    #[test]
    fn canvas_transforms_are_used_by_drawing_actions() {
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
            gc.layer(LayerId(0));
            gc.canvas_height(1000.0);

            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.stroke();

            gc.layer(LayerId(1));
            gc.canvas_height(2000.0);

            gc.new_path();
            gc.move_to(10.0, 10.0);
            gc.fill();

            gc.clear_layer();
        });

        // Only the commands after clear_layer should be present
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::CanvasHeight(1000.0)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Path(PathOp::Move(0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Stroke));
            assert!(stream.next().await == Some(Draw::CanvasHeight(2000.0)));
            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
        });
    }

    #[test]
    fn clear_layer_only_removes_commands_for_the_current_layer() {
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
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
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
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
        let (context, stream)   = DrawingTarget::new();

        // Draw using a graphics context
        context.draw(|gc| {
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
        let mut stream          = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));
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
    fn only_one_font_definition_survives_clear_layer() {
        let (context, stream)   = DrawingTarget::new();
        let lato                = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        context.draw(|gc| {
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

        mem::drop(context);
        let mut stream = stream;

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::StartFrame));

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
        let (context, stream)   = DrawingTarget::new();
        let lato                = CanvasFontFace::from_slice(include_bytes!("../test_data/Lato-Regular.ttf"));

        context.draw(|gc| {
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

        mem::drop(context);

        executor::block_on(async {
            let mut stream = stream;

            assert!(stream.next().await == Some(Draw::StartFrame));

            assert!(match stream.next().await { Some(Draw::Font(FontId(1), FontOp::UseFontDefinition(_))) => true, _ => false });
            assert!(stream.next().await == Some(Draw::Font(FontId(2), FontOp::FontSize(18.0))));
            assert!(stream.next().await == Some(Draw::Font(FontId(1), FontOp::FontSize(12.0))));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn sprite_definition_cleared_if_not_in_use() {
        let (context, stream)   = DrawingTarget::new();

        context.draw(|gc| {
            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.new_path();
            gc.fill();

            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.new_path();
            gc.fill();

            gc.layer(LayerId(0));
            gc.draw_sprite(SpriteId(0));
        });

        mem::drop(context);

        executor::block_on(async {
            let mut stream = stream;

            assert!(stream.next().await == Some(Draw::StartFrame));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(0))));
            assert!(stream.next().await == Some(Draw::ClearSprite));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::DrawSprite(SpriteId(0))));
        });
    }

    #[test]
    fn sprite_definition_survives_if_in_use() {
        let (context, stream)   = DrawingTarget::new();

        context.draw(|gc| {
            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.new_path();
            gc.fill();

            gc.layer(LayerId(0));
            gc.draw_sprite(SpriteId(0));

            gc.sprite(SpriteId(0));
            gc.clear_sprite();

            gc.new_path();
            gc.fill();

            gc.layer(LayerId(0));
            gc.draw_sprite(SpriteId(0));
        });

        mem::drop(context);

        executor::block_on(async {
            let mut stream = stream;

            assert!(stream.next().await == Some(Draw::StartFrame));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(0))));
            assert!(stream.next().await == Some(Draw::ClearSprite));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::DrawSprite(SpriteId(0))));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(0))));
            assert!(stream.next().await == Some(Draw::ClearSprite));
            assert!(stream.next().await == Some(Draw::Path(PathOp::NewPath)));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(0))));
            assert!(stream.next().await == Some(Draw::DrawSprite(SpriteId(0))));
        });
    }
}
