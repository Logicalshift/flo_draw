use super::gc::*;
use super::draw::*;
use super::color::*;
use super::transform2d::*;

use std::collections::vec_deque::*;
use std::sync::*;
use std::mem;

use desync::*;
use futures::task::Task;
use futures::task;
use futures::{Stream,Poll,Async};

///
/// The core structure used to store details of a canvas 
///
struct CanvasCore {
    /// What was drawn since the last clear command was sent to this canvas
    drawing_since_last_clear: Vec<Draw>,

    // Tasks to notify next time we add to the canvas
    pending_streams: Vec<Arc<CanvasStream>>,
}

///
/// A canvas is an abstract interface for drawing graphics. It doesn't actually provide a means to
/// render anything, but rather a way to describe how things should be drawn and pass those on to
/// a renderer elsewhere. 
///
pub struct Canvas {
    /// The core is shared amongst the canvas streams as well as used by the canvas itself
    core: Desync<CanvasCore>
}

impl CanvasCore {
    ///
    /// On restore, rewinds the canvas to before the last store operation
    /// 
    fn rewind_to_last_store(&mut self) {
        let mut last_store = None;

        // Search backwards in the drawing commands for the last store command
        for draw_index in (0..self.drawing_since_last_clear.len()).rev() {
            match self.drawing_since_last_clear[draw_index] {
                // Commands that might cause the store/restore to not undo perfectly break the sequence
                Draw::Clip      => break,
                Draw::Unclip    => break,
                Draw::PushState => break,
                Draw::PopState  => break,

                // If we find no sequence breaks and a store, this is where we want to rewind to
                Draw::Store     => {
                    last_store = Some(draw_index);
                    break;
                },

                _               => ()
            };
        }

        // Remove everything up to the last store position
        if let Some(last_store) = last_store {
            self.drawing_since_last_clear.truncate(last_store);
        }
    }

    ///
    /// Writes some drawing commands to this core
    /// 
    fn write(&mut self, to_draw: Vec<Draw>) {
        // Build up the list of new drawing commands
        let mut new_drawing     = vec![];
        let mut clear_pending   = false;

        // Process the drawing commands
        to_draw.iter().for_each(|draw| {
            match draw {
                &Draw::ClearCanvas => {
                    // Clearing the canvas empties the command list and updates the clear count
                    self.drawing_since_last_clear   = vec![];
                    clear_pending                   = true;

                    new_drawing = vec![];

                    // Start the new drawing with the 'clear' command
                    self.drawing_since_last_clear.push(*draw);
                },

                &Draw::Restore => {
                    // Have to push the restore in case it can't be cleared
                    self.drawing_since_last_clear.push(*draw);

                    // On a 'restore' command we clear out everything since the 'store' if we can (so we don't build a backlog)
                    self.rewind_to_last_store();
                }

                // Default is to add to the current drawing
                _ => self.drawing_since_last_clear.push(*draw)
            }

            // Send everything to the streams
            new_drawing.push(*draw);
        });

        // Send the new drawing commands to the streams
        let mut to_remove = vec![];

        for stream_index in 0..self.pending_streams.len() {
            // Send commands to this stream
            if !self.pending_streams[stream_index].send_drawing(&new_drawing, clear_pending) {
                // If it returns false then the stream has been dropped and we should remove it from this object
                to_remove.push(stream_index);
            }
        }

        // Remove streams (in reverse order so the indexes don't get messed up)
        for remove_index in to_remove.into_iter().rev() {
            self.pending_streams.remove(remove_index);
        }
    }
}

impl Canvas {
    ///
    /// Creates a new, blank, canvas
    ///
    pub fn new() -> Canvas {
        // A canvas is initially just a clear command
        let core = CanvasCore { 
            drawing_since_last_clear:   vec![ Draw::ClearCanvas ],
            pending_streams:            vec![ ]
        };

        Canvas {
            core: Desync::new(core)
        }
    }

    ///
    /// Sends some new drawing commands to this canvas
    ///
    pub fn write(&self, to_draw: Vec<Draw>) {
        // Only draw if there are any drawing commands
        if to_draw.len() != 0 {
            self.core.async(move |core| core.write(to_draw));
        }
    }

    ///
    /// Provides a way to draw on this canvas via a GC
    /// 
    pub fn draw<FnAction>(&self, action: FnAction)
    where FnAction: Send+FnOnce(&mut GraphicsPrimitives) -> () {
        self.core.sync(move |core| {
            let mut graphics_context = CoreContext {
                core:       core,
                pending:    vec![]
            };

            action(&mut graphics_context);
        })
    }

    ///
    /// Creates a stream for reading the instructions from this canvas
    ///
    pub fn stream(&self) -> Box<Stream<Item=Draw,Error=()>+Send> {
        // Create a new canvas stream
        let new_stream = Arc::new(CanvasStream::new());

        // Register it and send the current set of pending commands to it
        let add_stream = Arc::clone(&new_stream);
        self.core.sync(move |core| {
            // Send the data we've received since the last clear
            add_stream.send_drawing(&core.drawing_since_last_clear, true);

            // Store the stream in the core so future notifications get sent there
            core.pending_streams.push(add_stream);
        });

        // Return the new stream
        Box::new(FragileCanvasStream::new(new_stream))
    }

    ///
    /// Retrieves the list of drawing actions in this canvas
    ///
    pub fn get_drawing(&self) -> Vec<Draw> {
        self.core.sync(|core| core.drawing_since_last_clear.clone())
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.core.async(|core| {
            // Notify any streams that are using the canvas that it has gone away
            core.pending_streams.iter_mut().for_each(|stream| stream.notify_dropped());
        });
    }
}

///
/// Graphics context built from a canvas core
///
struct CoreContext<'a> {
    core:       &'a mut CanvasCore,
    pending:    Vec<Draw>
}

impl<'a> GraphicsContext for CoreContext<'a> {
    fn new_path(&mut self)                          { self.pending.push(Draw::NewPath); }
    fn move_to(&mut self, x: f32, y: f32)           { self.pending.push(Draw::Move(x, y)); }
    fn line_to(&mut self, x: f32, y: f32)           { self.pending.push(Draw::Line(x, y)); }

    fn bezier_curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        self.pending.push(Draw::BezierCurve((x1, y1), (x2, y2), (x3, y3)));
    }

    fn close_path(&mut self)                        { self.pending.push(Draw::ClosePath); }
    fn fill(&mut self)                              { self.pending.push(Draw::Fill); }
    fn stroke(&mut self)                            { self.pending.push(Draw::Stroke); }
    fn line_width(&mut self, width: f32)            { self.pending.push(Draw::LineWidth(width)); }
    fn line_width_pixels(&mut self, width: f32)     { self.pending.push(Draw::LineWidthPixels(width)); }
    fn line_join(&mut self, join: LineJoin)         { self.pending.push(Draw::LineJoin(join)); }
    fn line_cap(&mut self, cap: LineCap)            { self.pending.push(Draw::LineCap(cap)); }
    fn new_dash_pattern(&mut self)                  { self.pending.push(Draw::NewDashPattern); }
    fn dash_length(&mut self, length: f32)          { self.pending.push(Draw::DashLength(length)); }
    fn dash_offset(&mut self, offset: f32)          { self.pending.push(Draw::DashOffset(offset)); }
    fn fill_color(&mut self, col: Color)            { self.pending.push(Draw::FillColor(col)); }
    fn stroke_color(&mut self, col: Color)          { self.pending.push(Draw::StrokeColor(col)); }
    fn blend_mode(&mut self, mode: BlendMode)       { self.pending.push(Draw::BlendMode(mode)); }
    fn identity_transform(&mut self)                { self.pending.push(Draw::IdentityTransform); }
    fn canvas_height(&mut self, height: f32)        { self.pending.push(Draw::CanvasHeight(height)); }
    fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32) { self.pending.push(Draw::CenterRegion((minx, miny), (maxx, maxy))); }
    fn transform(&mut self, transform: Transform2D) { self.pending.push(Draw::MultiplyTransform(transform)); }
    fn unclip(&mut self)                            { self.pending.push(Draw::Unclip); }
    fn clip(&mut self)                              { self.pending.push(Draw::Clip); }
    fn store(&mut self)                             { self.pending.push(Draw::Store); }
    fn restore(&mut self)                           { self.pending.push(Draw::Restore); }
    fn push_state(&mut self)                        { self.pending.push(Draw::PushState); }
    fn pop_state(&mut self)                         { self.pending.push(Draw::PopState); }
    fn clear_canvas(&mut self)                      { self.pending.push(Draw::ClearCanvas); }
    fn layer(&mut self, layer_id: u32)              { self.pending.push(Draw::Layer(layer_id)); }
    fn layer_blend(&mut self, layer_id: u32, blend_mode: BlendMode) { self.pending.push(Draw::LayerBlend(layer_id, blend_mode)); }

    fn draw(&mut self, d: Draw)                     { self.pending.push(d); }
}

impl<'a> GraphicsPrimitives for CoreContext<'a> { }

impl<'a> Drop for CoreContext<'a> {
    fn drop(&mut self) {
        let mut to_draw = vec![];
        mem::swap(&mut self.pending, &mut to_draw);
        self.core.write(to_draw);
    }
}

///
/// Internals of a canvas stream
/// 
struct CanvasStreamCore {
    /// Items waiting to be drawn for this stream
    queue: VecDeque<Draw>,

    /// The task to notify when extra data is available
    waiting_task: Option<Task>,

    /// Set to true when the canvas is dropped
    canvas_dropped: bool,

    /// Set to true if the stream is dropped
    stream_dropped: bool
}

///
/// The canvas stream can be used to read the contents of the canvas and follow new content as it arrives.
/// Unconsumed commands are cut off if the `Draw::ClearCanvas` command is issued.
///
struct CanvasStream {
    /// The core of this stream
    core: Mutex<CanvasStreamCore>
}

impl CanvasStream {
    ///
    /// Creates a new canvas stream
    /// 
    pub fn new() -> CanvasStream {
        CanvasStream {
            core: Mutex::new(CanvasStreamCore {
                queue:          VecDeque::new(),
                waiting_task:   None,
                canvas_dropped: false,
                stream_dropped: false
            })
        }
    }

    ///
    /// Wakes the stream task
    /// 
    fn notify_dropped(&self) {
        let mut core = self.core.lock().unwrap();

        core.canvas_dropped = true;

        if let Some(ref task) = core.waiting_task {
            task.notify();
        }
        core.waiting_task = None;
    }

    ///
    /// Sends some drawing commands to this stream (returning true if this stream wants more commands)
    /// 
    fn send_drawing(&self, drawing: &Vec<Draw>, clear_pending: bool) -> bool {
        if drawing.len() > 0 {
            let mut core = self.core.lock().unwrap();

            // Clear out any pending commands if they're hidden by a clear
            if clear_pending {
                core.queue.clear();
            }

            // Push the drawing commands
            for draw in drawing {
                core.queue.push_back(*draw);
            }

            // If a task needs waking up, wake it
            if let Some(ref task) = core.waiting_task {
                task.notify();
            }
            core.waiting_task = None;

            // We want more commands if the stream is not dropped
            !core.stream_dropped
        } else {
            !self.core.lock().unwrap().stream_dropped
        }
    }
}

impl CanvasStream {
    fn poll(&self) -> Poll<Option<Draw>, ()> {
        use self::Async::*;

        let mut core = self.core.lock().unwrap();

        if let Some(value) = core.queue.pop_front() {
            Ok(Ready(Some(value)))
        } else if core.canvas_dropped {
            Ok(Ready(None))
        } else {
            core.waiting_task = Some(task::current());
            Ok(NotReady)
        }
   }
}

///
/// The 'fragile' canvas stream is a variant of the canvas stream that marks the
/// stream as being dropped if this happens (so that we can remove it from the
/// list in the canvas)
/// 
struct FragileCanvasStream {
    stream: Arc<CanvasStream>
}

impl FragileCanvasStream {
    pub fn new(stream: Arc<CanvasStream>) -> FragileCanvasStream {
        FragileCanvasStream { stream: stream }
    }
}

impl Drop for FragileCanvasStream {
    fn drop(&mut self) {
        let mut core = self.stream.core.lock().unwrap();

        core.stream_dropped = true;
    }
}

impl Stream for FragileCanvasStream {
    type Item = Draw;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Draw>, ()> {
        self.stream.poll()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::executor;

    use std::thread::*;
    use std::time::*;

    #[test]
    fn can_draw_to_canvas() {
        let canvas = Canvas::new();

        canvas.write(vec![Draw::NewPath]);
    }

    #[test]
    fn can_follow_canvas_stream() {
        let canvas  = Canvas::new();
        let mut stream  = executor::spawn(canvas.stream());
        
        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            canvas.write(vec![
                Draw::NewPath,
                Draw::Move(0.0, 0.0),
                Draw::Line(10.0, 0.0),
                Draw::Line(10.0, 10.0),
                Draw::Line(0.0, 10.0)
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        // Check we can get the results via the stream
        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));

        // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
        assert!(stream.wait_stream() == None);
    }

    #[test]
    fn can_draw_using_gc() {
        let canvas      = Canvas::new();
        let mut stream  = executor::spawn(canvas.stream());
        
        // Draw using a graphics context
        canvas.draw(|gc| {
            gc.new_path();
            gc.move_to(0.0, 0.0);
            gc.line_to(10.0, 0.0);
            gc.line_to(10.0, 10.0);
            gc.line_to(0.0, 10.0);
        });

        // Check we can get the results via the stream
        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));
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
        let mut stream  = executor::spawn(canvas.stream());

        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));

        assert!(stream.wait_stream() == Some(Ok(Draw::Stroke)));
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
        let mut stream  = executor::spawn(canvas.stream());

        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));

        assert!(stream.wait_stream() == Some(Ok(Draw::Store)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Clip)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Restore)));
    }

    #[test]
    fn can_follow_many_streams() {
        let canvas  = Canvas::new();
        let mut stream  = executor::spawn(canvas.stream());
        let mut stream2  = executor::spawn(canvas.stream());
        
        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            canvas.write(vec![
                Draw::NewPath,
                Draw::Move(0.0, 0.0),
                Draw::Line(10.0, 0.0),
                Draw::Line(10.0, 10.0),
                Draw::Line(0.0, 10.0)
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        // Check we can get the results via the stream
        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));

        assert!(stream2.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream2.wait_stream() == Some(Ok(Draw::NewPath)));
        assert!(stream2.wait_stream() == Some(Ok(Draw::Move(0.0, 0.0))));

        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));

        assert!(stream2.wait_stream() == Some(Ok(Draw::Line(10.0, 0.0))));
        assert!(stream2.wait_stream() == Some(Ok(Draw::Line(10.0, 10.0))));
        assert!(stream2.wait_stream() == Some(Ok(Draw::Line(0.0, 10.0))));

        // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
        assert!(stream.wait_stream() == None);
        assert!(stream2.wait_stream() == None);
    }

    #[test]
    fn commands_after_clear_are_suppressed() {
        let canvas  = Canvas::new();
        let mut stream  = executor::spawn(canvas.stream());
        
        // Thread to draw some stuff to the canvas
        spawn(move || {
            sleep(Duration::from_millis(50));

            canvas.write(vec![
                Draw::NewPath,
                Draw::Move(0.0, 0.0),
                Draw::Line(10.0, 0.0),
                Draw::Line(10.0, 10.0),
                Draw::Line(0.0, 10.0)
            ]);

            // Enough time that we read the first few commands
            sleep(Duration::from_millis(100));

            canvas.write(vec![
                Draw::ClearCanvas,
                Draw::Move(200.0, 200.0),
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        // Check we can get the results via the stream
        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::NewPath)));

        // Give the thread some time to clear the canvas
        sleep(Duration::from_millis(120));

        // Commands we sent before the flush are gone
        assert!(stream.wait_stream() == Some(Ok(Draw::ClearCanvas)));
        assert!(stream.wait_stream() == Some(Ok(Draw::Move(200.0, 200.0))));

        // When the thread goes away, it'll drop the canvas, so we should get the 'None' request here too
        assert!(stream.wait_stream() == None);
    }
}
