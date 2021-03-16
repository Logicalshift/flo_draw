use super::draw::*;
use super::font::*;
use super::color::*;
use super::context::*;
use super::texture::*;
use super::font_face::*;
use super::transform2d::*;

use std::collections::vec_deque::*;
use std::collections::{HashMap, HashSet};
use std::sync::*;
use std::mem;
use std::pin::*;

use desync::{Desync};
use futures::task;
use futures::task::{Poll, Waker};
use futures::{Stream};

/// Used to represent the layer or sprite that a drawing operation is for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LayerOrSpriteId {
    Layer(LayerId),
    Sprite(SpriteId)
}

///
/// The core structure used to store details of a canvas
///
struct CanvasCore {
    /// What was drawn since the last clear command was sent to this canvas (and the layer that it's on)
    drawing_since_last_clear: Vec<(LayerOrSpriteId, Draw)>,

    /// The current layer that we're drawing on
    current_layer: LayerOrSpriteId,

    // Tasks to notify next time we add to the canvas
    pending_streams: Vec<Arc<CanvasStream>>,
}

///
/// A canvas is an abstract interface for drawing graphics. It doesn't actually provide a means to
/// render anything, but rather a way to describe how things should be drawn and pass those on to
/// a renderer elsewhere.
///
#[derive(Clone)]
pub struct Canvas {
    /// The core is shared amongst the canvas streams as well as used by the canvas itself
    core: Arc<Desync<CanvasCore>>
}

impl CanvasCore {
    ///
    /// On restore, rewinds the canvas to before the last store operation
    ///
    fn rewind_to_last_store(&mut self) {
        let mut last_store = None;

        // Search backwards in the drawing commands for the last store command
        let mut state_stack_depth = 0;

        for draw_index in (0..self.drawing_since_last_clear.len()).rev() {
            match self.drawing_since_last_clear[draw_index] {
                // Commands that might cause the store/restore to not undo perfectly break the sequence
                (_, Draw::Clip)         => break,
                (_, Draw::Unclip)       => break,

                // If the state stack has a pop for every push then we can remove these requests too
                // TODO: this has a bug in that if the final event is a 'push' instead of a 'pop'
                // then it will mistakenly believe the states can be removed
                (_, Draw::PushState)    => { state_stack_depth += 1; },
                (_, Draw::PopState)     => { state_stack_depth -= 1; },

                // If we find no sequence breaks and a store, this is where we want to rewind to
                (_, Draw::Store)        => {
                    if state_stack_depth == 0 {
                        last_store = Some(draw_index+1);
                    }
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
    /// Removes all of the drawing for the specified layer
    ///
    /// (Except for ClearCanvas)
    ///
    fn clear_layer(&mut self, layer_id: LayerOrSpriteId) {
        // Take the old drawing from this object
        let mut old_drawing = vec![];
        mem::swap(&mut self.drawing_since_last_clear, &mut old_drawing);

        // Create a new drawing by filtering all of the actions for the current layer
        let new_drawing = old_drawing.into_iter()
            .filter(|drawing| {
                match drawing {
                    // Don't filter operations that affect the canvas state as a whole
                    (_, Draw::ClearCanvas(_))                           => true,
                    (_, Draw::LayerBlend(_, _))                         => true,
                    (_, Draw::StartFrame)                               => true,
                    (_, Draw::ShowFrame)                                => true,
                    (_, Draw::ResetFrame)                               => true,
                    (_, Draw::Font(_, FontOp::UseFontDefinition(_)))    => true,
                    (_, Draw::Font(_, FontOp::FontSize(_)))             => true,

                    // Don't filter anything that doesn't affect the cleared layer
                    (layer, _)                                          => *layer != layer_id
                }
            })
            .collect();

        // This becomes the new drawing for this layer
        self.drawing_since_last_clear = new_drawing;

        self.remove_unused_resources();
    }

    ///
    /// Iterates through the drawing since last clear and removes any resource operations that
    /// are replaced before they are used
    ///
    /// For example, if a font is declared and then redeclared with no usages, this will remove
    /// it from the drawing
    ///
    fn remove_unused_resources(&mut self) {
        let mut font_declarations       = HashMap::new();
        let mut font_size_declarations  = HashMap::new();
        let mut unused_indexes          = HashSet::new();

        // Find the indexes of the unused font operations
        for (idx, drawing) in self.drawing_since_last_clear.iter().enumerate() {
            match drawing {
                (_, Draw::Font(font_id, FontOp::UseFontDefinition(_))) => {
                    // If the font has an unused declaration, remove it from the list
                    if let Some(last_idx) = font_declarations.get(&font_id) {
                        unused_indexes.insert(*last_idx);
                    }

                    // This becomes the new last definition of this font
                    font_declarations.insert(font_id, idx);
                },

                (_, Draw::Font(font_id, FontOp::FontSize(_))) => {
                    // If the font has an unused declaration, remove it from the list
                    if let Some(last_idx) = font_size_declarations.get(&font_id) {
                        unused_indexes.insert(*last_idx);
                    }

                    // This becomes the new last definition of this font's size
                    font_size_declarations.insert(font_id, idx);
                }

                (_, Draw::Font(font_id, _)) => {
                    // Other fontops count as using the font
                    font_declarations.remove(font_id);
                    font_size_declarations.remove(font_id);
                }

                (_, Draw::DrawText(font_id, _, _, _)) => {
                    // As does drawing text with the font
                    font_declarations.remove(font_id);
                    font_size_declarations.remove(font_id);
                }

                _ => {}
            }
        }

        if unused_indexes.len() > 0 {
            // Remove any unused drawing item by filtering the drawing
            let drawing                     = mem::take(&mut self.drawing_since_last_clear);
            self.drawing_since_last_clear   = drawing.into_iter()
                .enumerate()
                .filter(|(idx, _)| !unused_indexes.contains(idx))
                .map(|(_, item)| item)
                .collect();
        }
    }

    ///
    /// Writes some drawing commands to this core
    ///
    fn write(&mut self, to_draw: Vec<Draw>) {
        // Build up the list of new drawing commands. new_drawing are the commands sent to the streams, and we build up a representation of the layer in drawing_since_last_clear
        let mut new_drawing     = vec![Draw::StartFrame];
        let mut clear_pending   = false;

        // Process the drawing commands
        to_draw.into_iter().for_each(|draw| {
            match &draw {
                Draw::ClearCanvas(_) => {
                    // Clearing the canvas empties the command list and updates the clear count
                    self.drawing_since_last_clear   = vec![(LayerOrSpriteId::Layer(LayerId(0)), Draw::ResetFrame)];
                    self.current_layer              = LayerOrSpriteId::Layer(LayerId(0));
                    clear_pending                   = true;

                    new_drawing                     = vec![Draw::StartFrame];

                    // Start the new drawing with the 'clear' command
                    self.drawing_since_last_clear.push((LayerOrSpriteId::Layer(LayerId(0)), draw.clone()));
                },

                Draw::Restore => {
                    // Have to push the restore in case it can't be cleared
                    self.drawing_since_last_clear.push((self.current_layer, draw.clone()));

                    // On a 'restore' command we clear out everything since the 'store' if we can (so we don't build a backlog)
                    self.rewind_to_last_store();
                },

                Draw::FreeStoredBuffer => {
                    // If the last operation was a store, pop it
                    if let Some(&(_, Draw::Store)) = self.drawing_since_last_clear.last() {
                        // Store and immediate free = just free
                        self.drawing_since_last_clear.pop();
                    } else {
                        // Something else: the free becomes part of the drawing log (this is often inefficient)
                        self.drawing_since_last_clear.push((self.current_layer, draw.clone()));
                    }
                },

                Draw::Layer(new_layer) => {
                    let new_layer       = LayerOrSpriteId::Layer(*new_layer);
                    self.current_layer  = new_layer;
                    self.drawing_since_last_clear.push((new_layer, draw.clone()));
                },

                Draw::Sprite(new_sprite) => {
                    let new_sprite      = LayerOrSpriteId::Sprite(*new_sprite);
                    self.current_layer  = new_sprite;
                    self.drawing_since_last_clear.push((new_sprite, draw.clone()));
                },

                Draw::ClearLayer => {
                    // Remove all of the commands for the current layer, replacing them with just a switch to this layer
                    let current_layer = self.current_layer;

                    match current_layer {
                        LayerOrSpriteId::Layer(layer_id) => {
                            self.clear_layer(current_layer);
                            self.drawing_since_last_clear.push((current_layer, Draw::Layer(layer_id)));
                        }

                        _ => { }
                    }
                },

                Draw::ClearSprite => {
                    // Remove all of the commands for the current layer, replacing them with just a switch to this layer
                    let current_layer = self.current_layer;

                    match current_layer {
                        LayerOrSpriteId::Sprite(sprite_id) => {
                            self.clear_layer(current_layer);
                            self.drawing_since_last_clear.push((current_layer, Draw::Sprite(sprite_id)));
                        }

                        _ => { }
                    }
                },

                Draw::ShowFrame => {
                    // Search for a 'StartFrame' instruction in the existing drawing and remove it
                    for idx in (0..self.drawing_since_last_clear.len()).into_iter().rev() {
                        if let (_, Draw::StartFrame) = &self.drawing_since_last_clear[idx] {
                            self.drawing_since_last_clear.remove(idx);
                            break;
                        }
                    }
                },

                Draw::ResetFrame => {
                    // Remove any frame instructions in the current drawing
                    self.drawing_since_last_clear.retain(|item| {
                        match item {
                            (_, Draw::StartFrame)   => false,
                            (_, Draw::ShowFrame)    => false,
                            _                       => true
                        }
                    });
                },

                // Default is to add to the current drawing
                _ => self.drawing_since_last_clear.push((self.current_layer, draw.clone()))
            }

            // Send everything to the streams
            new_drawing.push(draw);
        });

        // Send the new drawing commands to the streams
        let mut to_remove = vec![];

        // All these commands should be rendered as a single frame
        new_drawing.push(Draw::ShowFrame);

        for stream_index in 0..self.pending_streams.len() {
            // Send commands to this stream
            if !self.pending_streams[stream_index].send_drawing(new_drawing.iter().map(|draw| draw.clone()), clear_pending) {
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
            drawing_since_last_clear:   vec![ 
                (LayerOrSpriteId::Layer(LayerId(0)), Draw::ResetFrame),
                (LayerOrSpriteId::Layer(LayerId(0)), Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)))
            ],
            current_layer:              LayerOrSpriteId::Layer(LayerId(0)),
            pending_streams:            vec![ ]
        };

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
            self.core.desync(move |core| core.write(to_draw));
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
        let new_stream = Arc::new(CanvasStream::new());

        // Register it and send the current set of pending commands to it
        let add_stream = Arc::clone(&new_stream);
        self.core.sync(move |core| {
            // Send the data we've received since the last clear
            add_stream.send_drawing(core.drawing_since_last_clear.iter().map(|(_, draw)| draw.clone()), true);

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
        self.core.sync(|core| core.drawing_since_last_clear.iter().map(|(_, draw)| draw.clone()).collect())
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // The streams drop if this is the last canvas with this core
        if Arc::strong_count(&self.core) == 1 {
            self.core.desync(|core| {
                // Notify any streams that are using the canvas that it has gone away
                core.pending_streams.iter_mut().for_each(|stream| stream.notify_dropped());
            });
        }
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
    fn start_frame(&mut self)                       { self.pending.push(Draw::StartFrame); }
    fn show_frame(&mut self)                        { self.pending.push(Draw::ShowFrame); }
    fn reset_frame(&mut self)                       { self.pending.push(Draw::ResetFrame); }

    fn new_path(&mut self)                          { self.pending.push(Draw::NewPath); }
    fn move_to(&mut self, x: f32, y: f32)           { self.pending.push(Draw::Move(x, y)); }
    fn line_to(&mut self, x: f32, y: f32)           { self.pending.push(Draw::Line(x, y)); }

    fn bezier_curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        self.pending.push(Draw::BezierCurve((x1, y1), (x2, y2), (x3, y3)));
    }

    fn close_path(&mut self)                                                        { self.pending.push(Draw::ClosePath); }
    fn fill(&mut self)                                                              { self.pending.push(Draw::Fill); }
    fn stroke(&mut self)                                                            { self.pending.push(Draw::Stroke); }
    fn line_width(&mut self, width: f32)                                            { self.pending.push(Draw::LineWidth(width)); }
    fn line_width_pixels(&mut self, width: f32)                                     { self.pending.push(Draw::LineWidthPixels(width)); }
    fn line_join(&mut self, join: LineJoin)                                         { self.pending.push(Draw::LineJoin(join)); }
    fn line_cap(&mut self, cap: LineCap)                                            { self.pending.push(Draw::LineCap(cap)); }
    fn winding_rule(&mut self, rule: WindingRule)                                   { self.pending.push(Draw::WindingRule(rule)); }
    fn new_dash_pattern(&mut self)                                                  { self.pending.push(Draw::NewDashPattern); }
    fn dash_length(&mut self, length: f32)                                          { self.pending.push(Draw::DashLength(length)); }
    fn dash_offset(&mut self, offset: f32)                                          { self.pending.push(Draw::DashOffset(offset)); }
    fn fill_color(&mut self, col: Color)                                            { self.pending.push(Draw::FillColor(col)); }
    fn fill_texture(&mut self, t: TextureId, x1: f32, y1: f32, x2: f32, y2: f32)    { self.pending.push(Draw::FillTexture(t, (x1, y1), (x2, y2))); }
    fn stroke_color(&mut self, col: Color)                                          { self.pending.push(Draw::StrokeColor(col)); }
    fn blend_mode(&mut self, mode: BlendMode)                                       { self.pending.push(Draw::BlendMode(mode)); }
    fn identity_transform(&mut self)                                                { self.pending.push(Draw::IdentityTransform); }
    fn canvas_height(&mut self, height: f32)                                        { self.pending.push(Draw::CanvasHeight(height)); }
    fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32)         { self.pending.push(Draw::CenterRegion((minx, miny), (maxx, maxy))); }
    fn transform(&mut self, transform: Transform2D)                                 { self.pending.push(Draw::MultiplyTransform(transform)); }
    fn unclip(&mut self)                                                            { self.pending.push(Draw::Unclip); }
    fn clip(&mut self)                                                              { self.pending.push(Draw::Clip); }
    fn store(&mut self)                                                             { self.pending.push(Draw::Store); }
    fn restore(&mut self)                                                           { self.pending.push(Draw::Restore); }
    fn free_stored_buffer(&mut self)                                                { self.pending.push(Draw::FreeStoredBuffer); }
    fn push_state(&mut self)                                                        { self.pending.push(Draw::PushState); }
    fn pop_state(&mut self)                                                         { self.pending.push(Draw::PopState); }
    fn clear_canvas(&mut self, color: Color)                                        { self.pending.push(Draw::ClearCanvas(color)); }
    fn layer(&mut self, layer_id: LayerId)                                          { self.pending.push(Draw::Layer(layer_id)); }
    fn layer_blend(&mut self, layer_id: LayerId, blend_mode: BlendMode)             { self.pending.push(Draw::LayerBlend(layer_id, blend_mode)); }
    fn clear_layer(&mut self)                                                       { self.pending.push(Draw::ClearLayer); }
    fn sprite(&mut self, sprite_id: SpriteId)                                       { self.pending.push(Draw::Sprite(sprite_id)); }
    fn clear_sprite(&mut self)                                                      { self.pending.push(Draw::ClearSprite); }
    fn sprite_transform(&mut self, transform: SpriteTransform)                      { self.pending.push(Draw::SpriteTransform(transform)); }
    fn draw_sprite(&mut self, sprite_id: SpriteId)                                  { self.pending.push(Draw::DrawSprite(sprite_id)); }

    fn define_font_data(&mut self, font_id: FontId, font_data: Arc<CanvasFontFace>)                             { self.pending.push(Draw::Font(font_id, FontOp::UseFontDefinition(font_data))); }
    fn set_font_size(&mut self, font_id: FontId, size: f32)                                                     { self.pending.push(Draw::Font(font_id, FontOp::FontSize(size))); }
    fn draw_text(&mut self, font_id: FontId, text: String, baseline_x: f32, baseline_y: f32)                    { self.pending.push(Draw::DrawText(font_id, text, baseline_x, baseline_y)); }
    fn draw_glyphs(&mut self, font_id: FontId, glyphs: Vec<GlyphPosition>)                                      { self.pending.push(Draw::Font(font_id, FontOp::DrawGlyphs(glyphs))); }
    fn begin_line_layout(&mut self, x: f32, y: f32, align: TextAlignment)                                       { self.pending.push(Draw::BeginLineLayout(x, y, align)); }
    fn layout_text(&mut self, font_id: FontId, text: String)                                                    { self.pending.push(Draw::Font(font_id, FontOp::LayoutText(text))); }
    fn draw_text_layout(&mut self)                                                                              { self.pending.push(Draw::DrawLaidOutText); }

    fn create_texture(&mut self, texture_id: TextureId, w: u32, h: u32, format: TextureFormat)                  { self.pending.push(Draw::Texture(texture_id, TextureOp::Create(w, h, format))); }
    fn set_texture_bytes(&mut self, texture_id: TextureId, x: u32, y: u32, w: u32, h: u32, bytes: Arc<Vec<u8>>) { self.pending.push(Draw::Texture(texture_id, TextureOp::SetBytes(x, y, w, h, bytes))); }
    fn free_texture(&mut self, texture_id: TextureId)                                                           { self.pending.push(Draw::Texture(texture_id, TextureOp::Free)); }
    fn set_texture_fill_alpha(&mut self, texture_id: TextureId, alpha: f32)                                     { self.pending.push(Draw::Texture(texture_id, TextureOp::FillTransparency(alpha))); }

    fn draw(&mut self, d: Draw)                                 { self.pending.push(d); }
}

impl<'a> Drop for CanvasGraphicsContext<'a> {
    fn drop(&mut self) {
        self.core.write(mem::take(&mut self.pending));
    }
}

///
/// Internals of a canvas stream
///
struct CanvasStreamCore {
    /// Items waiting to be drawn for this stream
    queue: VecDeque<Draw>,

    /// The task to notify when extra data is available
    waiting_task: Option<Waker>,

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

        if let Some(task) = core.waiting_task.take() {
            task.wake();
        }
    }

    ///
    /// Sends some drawing commands to this stream (returning true if this stream wants more commands)
    ///
    fn send_drawing<DrawIter: Iterator<Item=Draw>> (&self, drawing: DrawIter, clear_pending: bool) -> bool {
        let mut core = self.core.lock().unwrap();

        // Clear out any pending commands if they're hidden by a clear (except frame commands, which we need to add up)
        if clear_pending {
            core.queue.retain(|action| {
                match action {
                    Draw::StartFrame    |
                    Draw::ShowFrame     |
                    Draw::ResetFrame    => true,
                    _                   => false
                }
            });
        }

        // Push the drawing commands
        for draw in drawing {
            core.queue.push_back(draw);
        }

        // If a task needs waking up, wake it
        if let Some(task) = core.waiting_task.take() {
            task.wake();
        }

        // We want more commands if the stream is not dropped
        !core.stream_dropped
    }
}

impl CanvasStream {
    fn poll(&self, context: &task::Context) -> Poll<Option<Draw>> {
        let mut core = self.core.lock().unwrap();

        if let Some(value) = core.queue.pop_front() {
            Poll::Ready(Some(value))
        } else if core.canvas_dropped {
            Poll::Ready(None)
        } else {
            core.waiting_task = Some(context.waker().clone());
            Poll::Pending
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

    fn poll_next(self: Pin<&mut Self>, context: &mut task::Context) -> Poll<Option<Draw>> {
        self.stream.poll(context)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::primitives::*;

    use futures::prelude::*;
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
        let canvas      = Canvas::new();
        let mut stream  = canvas.stream();

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
        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));
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
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));
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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));

            // 'Store' is still present as we can restore the same thing repeatedly
            assert!(stream.next().await == Some(Draw::Store));

            assert!(stream.next().await == Some(Draw::Stroke));
        })
    }

    #[test]
    fn free_store_rewinds_canvas_further() {
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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));

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

        executor::block_on(async {
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));

            assert!(stream.next().await == Some(Draw::Store));
            assert!(stream.next().await == Some(Draw::Clip));
            assert!(stream.next().await == Some(Draw::NewPath));
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
                Draw::NewPath,
                Draw::Move(0.0, 0.0),
                Draw::Line(10.0, 0.0),
                Draw::Line(10.0, 10.0),
                Draw::Line(0.0, 10.0)
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/

        executor::block_on(async {
            // Check we can get the results via the stream
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(0.0, 0.0)));

            assert!(stream2.next().await == Some(Draw::ResetFrame));
            assert!(stream2.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream2.next().await == Some(Draw::StartFrame));
            assert!(stream2.next().await == Some(Draw::NewPath));
            assert!(stream2.next().await == Some(Draw::Move(0.0, 0.0)));

            assert!(stream.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Line(0.0, 10.0)));

            assert!(stream2.next().await == Some(Draw::Line(10.0, 0.0)));
            assert!(stream2.next().await == Some(Draw::Line(10.0, 10.0)));
            assert!(stream2.next().await == Some(Draw::Line(0.0, 10.0)));

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
                Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0)),
                Draw::Move(200.0, 200.0),
            ]);
        });

        // TODO: if the canvas fails to notify, this will block forever :-/
        executor::block_on(async {
            // Check we can get the results via the stream
            assert!(stream.next().await == Some(Draw::ResetFrame));
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::NewPath));

            // Give the thread some time to clear the canvas
            sleep(Duration::from_millis(120));

            // Should immediately stop the old frame and start a new one
            assert!(stream.next().await == Some(Draw::ShowFrame));
            assert!(stream.next().await == Some(Draw::StartFrame));

            // Commands we sent before the flush are gone
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::Move(200.0, 200.0)));
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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(20.0, 20.0)));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
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
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(20.0, 20.0)));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Sprite(SpriteId(1))));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Fill));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::Fill));
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
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }

    #[test]
    fn show_start_frames_cancel_out() {
        let canvas  = Canvas::new();

        canvas.draw(|gc| {
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
            assert!(stream.next().await == Some(Draw::ClearCanvas(Color::Rgba(0.0, 0.0, 0.0, 0.0))));
            assert!(stream.next().await == Some(Draw::StartFrame));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(20.0, 20.0)));
            assert!(stream.next().await == Some(Draw::Stroke));

            assert!(stream.next().await == Some(Draw::Layer(LayerId(1))));
            assert!(stream.next().await == Some(Draw::NewPath));
            assert!(stream.next().await == Some(Draw::Move(10.0, 10.0)));
            assert!(stream.next().await == Some(Draw::Fill));
        });
    }
}
