use crate::edgeplan::*;
use crate::pixel::*;

use flo_canvas as canvas;
use flo_canvas::curves::line::*;
use flo_canvas::curves::bezier::*;

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
        }
    }
}

impl DrawingState {
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
}
