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
        let mut actions = vec![];
        action(&mut actions);

        // Send the actions to the core
        self.write(actions);
    }
}

impl Drop for DrawingContext {
    fn drop(&mut self) {
        let waker = self.stream_core.sync(|core| { core.close(); core.take_waker() });
        waker.map(|waker| waker.wake());
    }
}
