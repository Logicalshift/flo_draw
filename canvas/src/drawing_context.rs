use crate::draw::*;
use crate::draw_stream::*;

use ::desync::*;

use std::sync::*;

type DrawGraphicsContext = Vec<Draw>;

///
/// A drawing context sends drawing instructions to a `DrawStream`
///
/// Unlike `Canvas` - which performs a similar function - `DrawingContext` does not keep the drawing instructions
/// permanently in memory, it just forwards them as fast as possible to a drawing target.
///
pub struct DrawingContext {
    /// The stream core is where drawing instructions will be sent to
    stream_core: Arc<Desync<DrawStreamCore>>,
}

impl DrawingContext {
    ///
    /// Creates a new drawing context and a stream that can be used to read the instructions sent to it
    ///
    pub fn new() -> (DrawingContext, DrawStream) {
        // Create the core
        let core    = Arc::new(Desync::new(DrawStreamCore::new()));

        // Create the stream
        let stream  = DrawStream::with_core(&core);

        // Create the context
        let context = DrawingContext {
            stream_core: core
        };

        (context, stream)
    }

    ///
    /// Sends some drawing instructions to this context
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
    /// Provides a way to draw on this context via a graphics context
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
}

impl Drop for DrawingContext {
    fn drop(&mut self) {
        let waker = self.stream_core.sync(|core| { core.close(); core.take_waker() });
        waker.map(|waker| waker.wake());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::font::*;
    use crate::context::*;
    use crate::font_face::*;

    use futures::prelude::*;
    use futures::executor;

    use std::thread::*;
    use std::time::*;
    use std::mem;

    #[test]
    fn follow_drawing_context_stream() {
        let (context, stream) = DrawingContext::new();

        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            context.write(vec![
                Draw::NewPath,
                Draw::Move(0.0, 0.0),
                Draw::Line(10.0, 0.0),
                Draw::Line(10.0, 10.0),
                Draw::Line(0.0, 10.0)
            ]);
        });

        // Check we can get the results via the stream
        executor::block_on(async {
            let mut stream = stream;

            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));

            // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
            assert!(stream.next().await == None);
        })
    }

    #[test]
    fn clear_layer_0_removes_commands() {
        let (context, stream)   = DrawingContext::new();

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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_only_removes_commands_for_the_current_layer() {
        let (context, stream)   = DrawingContext::new();

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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(20.0, 20.0)));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn clear_layer_does_not_clear_sprites() {
        let (context, stream)   = DrawingContext::new();

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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(20.0, 20.0)));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(1))));
            assert!(stream.next().await == Some(Draw::ClearSprite));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::ClearLayer));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn only_one_font_definition_survives_clear_layer() {
        let (context, stream)   = DrawingContext::new();
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
        let (context, stream)   = DrawingContext::new();
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
}
