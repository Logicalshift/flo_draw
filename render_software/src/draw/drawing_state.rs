use super::canvas_drawing::*;

use crate::edgeplan::*;
use crate::edges::*;
use crate::pixel::*;

use flo_canvas as canvas;
use flo_canvas::curves::line::*;
use flo_canvas::curves::bezier::*;
use flo_canvas::curves::bezier::path as curves_path;

use std::sync::*;

///
/// A brush represents what will be used to fill in the next region 
///
#[derive(Clone)]
pub enum Brush {
    /// Basic solid colour brush (will be drawn opaque so the image behind will be hidden)
    OpaqueSolidColor(canvas::Color),

    /// Transparent solid colour brush (will be blended with the image behind)
    TransparentSolidColor(canvas::Color),
}

#[derive(Clone)]
pub enum DrawingClipRegion {
    /// No clip region set
    None,

    /// A clip region described by a path with an even-odd winding rule
    EvenOdd(Arc<ClipRegion<FlattenedBezierEvenOddEdge>>),

    /// A clip region described by a path with a non-zero winding rule
    NonZero(Arc<ClipRegion<FlattenedBezierNonZeroEdge>>),

    /// A clip region that is itself clipped by another region
    Nested(Arc<ClipRegion<ClippedShapeEdge<Arc<dyn EdgeDescriptor>, Arc<dyn EdgeDescriptor>>>>)
}

///
/// Represents the active drawing state for a canvas drawing
///
#[derive(Clone)]
pub struct DrawingState {
    /// The transform to apply to points added to a path
    ///
    /// The internal coordinates should range from 1 to -1 along the y axis (with the x scaling determined by whatever is needed to make the pixels square)
    pub (super) transform: canvas::Transform2D,

    /// The shape descriptor that will be used for filling the next shape (or None if we haven't allocated data for it yet)
    pub (super) fill_program: Option<ShapeDescriptor>,

    /// The shape descriptor that will be used for filling the stroke of the next shape (or None if we haven't allocated data for it yet)
    pub (super) stroke_program: Option<ShapeDescriptor>,

    /// The brush to select next time fill_program is None
    pub (super) next_fill_brush: Brush,

    /// The brush to select next time stroke_program is None
    pub (super) next_stroke_brush: Brush,

    /// The current position along the path
    pub (super) path_position: Coord2,

    /// The edges of the current path in this drawing state
    pub (super) path_edges: Vec<Curve<Coord2>>,

    /// Indexes of the points where the subpaths starts
    pub (super) subpaths: Vec<usize>,

    /// The winding rule to apply to the next path to be drawn
    pub (super) winding_rule: canvas::WindingRule,

    /// The width of the next stroke
    pub (super) stroke_width: f64,

    /// How lines should be joined together
    pub (super) stroke_join: curves_path::LineJoin,

    /// The start cap for the next stroke
    pub (super) stroke_start_cap: curves_path::LineCap,

    /// The end cap for the next stroke
    pub (super) stroke_end_cap: curves_path::LineCap,

    /// The currently set clip region, if any
    pub (super) clip_path: DrawingClipRegion,
}

impl Default for DrawingState {
    fn default() -> Self {
        DrawingState { 
            transform:          canvas::Transform2D::identity(),
            fill_program:       None,
            stroke_program:     None,
            next_fill_brush:    Brush::OpaqueSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            next_stroke_brush:  Brush::OpaqueSolidColor(canvas::Color::Rgba(0.0, 0.0, 0.0, 1.0)),
            path_position:      Coord2::origin(),
            path_edges:         vec![],
            subpaths:           vec![0],
            winding_rule:       canvas::WindingRule::NonZero,
            stroke_width:       1.0/200.0,
            stroke_join:        curves_path::LineJoin::Round,
            stroke_start_cap:   curves_path::LineCap::Butt,
            stroke_end_cap:     curves_path::LineCap::Butt,
            clip_path:          DrawingClipRegion::None,
        }
    }
}

impl DrawingState {
    ///
    /// Ensures that a program location is retained
    ///
    #[inline]
    pub fn retain_program<TPixel, const N: usize>(program: &Option<ShapeDescriptor>, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        if let Some(program) = &program {
            for program_data in program.programs.iter().copied() {
                data_cache.retain_program_data(program_data);
            }
        }
    }

    ///
    /// Ensures that a program location is released (sets it to None)
    ///
    /// The state holds on to the programs it's going to use, so they have to be released before they can be changed
    ///
    #[inline]
    pub fn release_program<TPixel, const N: usize>(program: &mut Option<ShapeDescriptor>, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        if let Some(mut program) = program.take() {
            for program_data in program.programs.drain(..) {
                data_cache.release_program_data(program_data);
            }
        }
    }

    ///
    /// Releases any pixel program data that is being retained by this state
    ///
    pub fn release_all_programs<TPixel, const N: usize>(&mut self, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        Self::release_program(&mut self.fill_program, data_cache);
        Self::release_program(&mut self.stroke_program, data_cache);
    }

    ///
    /// Updates the state so that the next shape added will use a solid fill colour 
    ///
    pub fn fill_solid_color<TPixel, const N: usize>(&mut self, colour: canvas::Color, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        // This clears the fill program so we allocate data for it next time
        Self::release_program(&mut self.fill_program, data_cache);

        // Choose opaque or transparent for the brush based on the alpha component
        if colour.alpha_component() >= 1.0 {
            self.next_fill_brush = Brush::OpaqueSolidColor(colour);
        } else {
            self.next_fill_brush = Brush::TransparentSolidColor(colour);
        }
    }

    ///
    /// Updates the state so that the next shape added will use a solid fill colour 
    ///
    pub fn stroke_solid_color<TPixel, const N: usize>(&mut self, colour: canvas::Color, data_cache: &mut PixelProgramDataCache<TPixel>)
    where
        TPixel: Send + Pixel<N>,
    {
        // This clears the stroke program so we allocate data for it next time
        Self::release_program(&mut self.stroke_program, data_cache);

        // Choose opaque or transparent for the brush based on the alpha component
        if colour.alpha_component() >= 1.0 {
            self.next_stroke_brush = Brush::OpaqueSolidColor(colour);
        } else {
            self.next_stroke_brush = Brush::TransparentSolidColor(colour);
        }
    }

    ///
    /// Sets the winding rule to use for the next path to be drawn
    ///
    #[inline]
    pub fn winding_rule(&mut self, winding_rule: canvas::WindingRule) {
        self.winding_rule = winding_rule;
    }

    ///
    /// Sets the line join style
    ///
    #[inline]
    pub fn line_join(&mut self, join: canvas::LineJoin) {
        self.stroke_join = join.into();
    }

    ///
    /// Sets the line join style
    ///
    #[inline]
    pub fn line_cap(&mut self, cap: canvas::LineCap) {
        self.stroke_start_cap   = cap.into();
        self.stroke_end_cap     = cap.into();
    }

    ///
    /// Applies the clipping rules to a shape, returning an edge descriptor
    ///
    #[inline]
    pub fn clip_shape(&self, shape_id: ShapeId, shape: Vec<impl 'static + Clone + EdgeDescriptor>) -> Vec<Arc<dyn EdgeDescriptor>> {
        match &self.clip_path {
            DrawingClipRegion::None             => shape.into_iter().map(|edge| { let result: Arc<dyn EdgeDescriptor> = Arc::new(edge); result }).collect(),
            DrawingClipRegion::EvenOdd(region)  => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
            DrawingClipRegion::NonZero(region)  => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
            DrawingClipRegion::Nested(region)   => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
        }
    }
}


impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Pushes a state onto the stack
    ///
    pub (super) fn push_state(&mut self) {
        // Copy the existing state
        let state_copy = self.current_state.clone();

        // Retain the fill and stroke shapes
        DrawingState::retain_program(&state_copy.fill_program, &mut self.program_data_cache);
        DrawingState::retain_program(&state_copy.stroke_program, &mut self.program_data_cache);

        // Store on the stack
        self.state_stack.push(state_copy);
    }

    ///
    /// Removes a state from the stack and makes it the current state
    ///
    pub (super) fn pop_state(&mut self) {
        if let Some(new_state) = self.state_stack.pop() {
            // Release the programs for the current state
            DrawingState::release_program(&mut self.current_state.fill_program, &mut self.program_data_cache);
            DrawingState::release_program(&mut self.current_state.stroke_program, &mut self.program_data_cache);

            // Replace with the new state
            self.current_state = new_state;
        }
    }
}
