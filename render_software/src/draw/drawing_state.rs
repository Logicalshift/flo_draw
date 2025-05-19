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
/// The renderer representation of a sprite transform
///
#[derive(Copy, Clone)]
pub enum SpriteTransform {
    /// Scale then transform 
    ScaleTransform { scale: (f64, f64), translate: (f64, f64) },

    /// Arbitrary transform
    Matrix(canvas::Transform2D),
}

///
/// A brush represents what will be used to fill in the next region 
///
#[derive(Clone)]
pub enum Brush {
    /// Basic solid colour brush (will be drawn opaque so the image behind will be hidden)
    OpaqueSolidColor(canvas::Color),

    /// Transparent solid colour brush (will be blended with the image behind)
    TransparentSolidColor(canvas::Color),

    /// A transformed texture, known to have transparent pixels in it
    TransparentTexture(f64, Arc<RgbaTexture>, canvas::Transform2D),

    /// A transformed texture, known to have transparent pixels in it
    TransparentLinearTexture(f64, Arc<U16LinearTexture>, canvas::Transform2D),

    /// A transformed texture that will be rendered using mip-maps
    TransparentMipMapTexture(f64, Arc<MipMap<Arc<U16LinearTexture>>>, canvas::Transform2D),

    /// A linear gradient
    LinearGradient(f64, canvas::NamespaceId, canvas::GradientId, canvas::Transform2D),
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

    /// The blend mode to use with the brush
    pub (super) blend_mode: AlphaOperation,

    /// The transform that's applied to the next sprite to be drawn
    pub (super) sprite_transform: SpriteTransform,
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
            blend_mode:         AlphaOperation::SourceOver,
            sprite_transform:   SpriteTransform::ScaleTransform { scale: (1.0, 1.0), translate: (0.0, 0.0) },
        }
    }
}

impl DrawingState {
    ///
    /// Ensures that a program location is retained
    ///
    #[inline]
    pub (crate) fn retain_program<TPixel, const N: usize>(program: &Option<ShapeDescriptor>, data_cache: &mut PixelProgramDataCache<TPixel>) 
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
    pub (crate) fn release_program<TPixel, const N: usize>(program: &mut Option<ShapeDescriptor>, data_cache: &mut PixelProgramDataCache<TPixel>) 
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
    pub (crate) fn release_all_programs<TPixel, const N: usize>(&mut self, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        Self::release_program(&mut self.fill_program, data_cache);
        Self::release_program(&mut self.stroke_program, data_cache);
    }

    ///
    /// Updates the state so that the next shape added will use a solid fill colour 
    ///
    pub (crate) fn fill_solid_color<TPixel, const N: usize>(&mut self, colour: canvas::Color, data_cache: &mut PixelProgramDataCache<TPixel>) 
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
    pub (crate) fn stroke_solid_color<TPixel, const N: usize>(&mut self, colour: canvas::Color, data_cache: &mut PixelProgramDataCache<TPixel>)
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
    /// Sets the blending mode of the current brush
    ///
    pub (crate) fn blend_mode<TPixel, const N: usize>(&mut self, blend_mode: canvas::BlendMode, data_cache: &mut PixelProgramDataCache<TPixel>) 
    where
        TPixel: Send + Pixel<N>,
    {
        use canvas::BlendMode::*;

        // Convert the blend mode to an alpha operation
        let operation = match blend_mode {
            SourceOver          => { AlphaOperation::SourceOver },
            SourceIn            => { AlphaOperation::SourceIn },
            SourceOut           => { AlphaOperation::SourceHeldOut },
            DestinationOver     => { AlphaOperation::DestOver },
            DestinationIn       => { AlphaOperation::DestIn },
            DestinationOut      => { AlphaOperation::DestHeldOut },
            SourceAtop          => { AlphaOperation::SourceAtop },
            DestinationAtop     => { AlphaOperation::DestAtop },

            Multiply            => { todo!() },
            Screen              => { todo!() },
            Darken              => { todo!() },
            Lighten             => { todo!() },
        };

        if operation != self.blend_mode {
            // This will change the brush used for filling as well as the stroke
            Self::release_program(&mut self.fill_program, data_cache);
            Self::release_program(&mut self.stroke_program, data_cache);

            // Set the alpha operation as our blend mode
            self.blend_mode = operation;
        }
    }

    ///
    /// Sets the winding rule to use for the next path to be drawn
    ///
    #[inline]
    pub (crate) fn winding_rule(&mut self, winding_rule: canvas::WindingRule) {
        self.winding_rule = winding_rule;
    }

    ///
    /// Sets the line join style
    ///
    #[inline]
    pub (crate) fn line_join(&mut self, join: canvas::LineJoin) {
        self.stroke_join = join.into();
    }

    ///
    /// Sets the line join style
    ///
    #[inline]
    pub (crate) fn line_cap(&mut self, cap: canvas::LineCap) {
        self.stroke_start_cap   = cap.into();
        self.stroke_end_cap     = cap.into();
    }

    ///
    /// Applies the clipping rules to a shape, returning an edge descriptor
    ///
    #[inline]
    pub (crate) fn clip_shape(&self, shape_id: ShapeId, shape: Vec<impl 'static + Clone + EdgeDescriptor>) -> Vec<Arc<dyn EdgeDescriptor>> {
        match &self.clip_path {
            DrawingClipRegion::None             => shape.into_iter().map(|edge| { let result: Arc<dyn EdgeDescriptor> = Arc::new(edge); result }).collect(),
            DrawingClipRegion::EvenOdd(region)  => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
            DrawingClipRegion::NonZero(region)  => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
            DrawingClipRegion::Nested(region)   => vec![Arc::new(ClippedShapeEdge::new(shape_id, Arc::clone(region), shape))],
        }
    }

    ///
    /// Sets the fill transform to a particular transformation
    ///
    #[inline]
    pub (crate) fn fill_transform(&mut self, transform: canvas::Transform2D) {
        let transform = transform.invert().unwrap_or_else(|| canvas::Transform2D::identity());

        match &mut self.next_fill_brush {
            Brush::OpaqueSolidColor(_) |
            Brush::TransparentSolidColor(_)                         => { }

            Brush::TransparentTexture(_, _, fill_transform)         |
            Brush::TransparentLinearTexture(_, _, fill_transform)   |
            Brush::TransparentMipMapTexture(_, _, fill_transform)   => { *fill_transform = transform * *fill_transform; }

            Brush::LinearGradient(_, _, _, gradient_transform)      => { *gradient_transform = transform * *gradient_transform; }
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

#[cfg(test)]
mod test {
    use flo_canvas::*;

    use super::*;

    fn texture_transform(state: &DrawingState) -> Transform2D {
        match state.next_fill_brush {
            Brush::TransparentTexture(_, _, fill_transform)         |
            Brush::TransparentLinearTexture(_, _, fill_transform)   |
            Brush::TransparentMipMapTexture(_, _, fill_transform)   => fill_transform.clone(),

            _ => panic!("not a texture brush")
        }
    }

    #[test]
    fn texture_positioning_1() {
        // Checking that we create the right transform for positioning the texture
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 0.0, 0.0, 1000.0, 1000.0);

        // Point 0,0 on the canvas should be 0,0 on the texture
        let (x,y)       = drawing.current_state.transform.transform_point(0.0, 0.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);

        // Point 1000,1000 on the canvas should be 100,200 on the texture
        let (x,y)       = drawing.current_state.transform.transform_point(1000.0, 1000.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 100.0).abs() < 0.01, "Expected 100,200, got {} {}", tx, ty);
        assert!((ty - 200.0).abs() < 0.01, "Expected 100,200, got {} {}", tx, ty);
    }

    #[test]
    fn texture_positioning_2() {
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 100.0, 100.0, 800.0, 800.0);

        // Point 100,100 on the canvas should be 0,0 on the texture
        let (x,y)       = drawing.current_state.transform.transform_point(100.0, 100.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);

        // Point 800,800 on the canvas should be 100,200 on the texture
        let (x,y)       = drawing.current_state.transform.transform_point(800.0, 800.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 100.0).abs() < 0.01, "Expected 100,200, got {} {}", tx, ty);
        assert!((ty - 200.0).abs() < 0.01, "Expected 100,200, got {} {}", tx, ty);
    }

    #[test]
    fn texture_scale_origin_same_1() {
        // The texture should transform around its origin point (defined by the first two parameters to fill_texture)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 0.0, 0.0, 1000.0, 1000.0);

        // Rotate it
        drawing.current_state.fill_transform(Transform2D::scale(2.0, 2.0));

        // Top corner of the texture should be 50, 100
        let (x,y)       = drawing.current_state.transform.transform_point(1000.0, 1000.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 50.0).abs() < 0.01, "Expected 50,100, got {} {}", tx, ty);
        assert!((ty - 100.0).abs() < 0.01, "Expected 50,100, got {} {}", tx, ty);

        // Bottom corner of the texture should stay at 0,0
        let (x,y)       = drawing.current_state.transform.transform_point(0.0, 0.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
    }

    #[test]
    fn texture_scale_origin_same_2() {
        // The texture should transform around its origin point (defined by the first two parameters to fill_texture)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 200.0, 200.0, 800.0, 800.0);

        // Rotate it
        drawing.current_state.fill_transform(Transform2D::scale(2.0, 2.0));

        // Top corner of the texture should be 50, 100
        let (x,y)       = drawing.current_state.transform.transform_point(800.0, 800.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 50.0).abs() < 0.01, "Expected 50,100, got {} {}", tx, ty);
        assert!((ty - 100.0).abs() < 0.01, "Expected 50,100, got {} {}", tx, ty);

        // Bottom corner of the texture should stay at 0,0
        let (x,y)       = drawing.current_state.transform.transform_point(200.0, 200.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
    }

    #[test]
    fn texture_rotation_origin_same_1() {
        // The texture should transform around its origin point (defined by the first two parameters to fill_texture)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 0.0, 0.0, 1000.0, 1000.0);

        // Rotate it
        drawing.current_state.fill_transform(Transform2D::rotate_degrees(45.0));

        // Point 0,0 on the canvas should still be 0,0 on the texture after rotation (we rotate about the 0,0 point on the texture)
        let (x,y)       = drawing.current_state.transform.transform_point(0.0, 0.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
    }

    #[test]
    fn texture_rotation_origin_same_2() {
        // The texture should transform around its origin point (this time, the origin is at a different point on the canvas)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 200.0, 200.0, 800.0, 800.0);

        // Rotate it
        drawing.current_state.fill_transform(Transform2D::rotate_degrees(45.0));

        // Point 0,0 on the canvas should still be 0,0 on the texture after rotation (we rotate about the 0,0 point on the texture)
        let (x,y)       = drawing.current_state.transform.transform_point(200.0, 200.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
        assert!((ty - 0.0).abs() < 0.01, "Expected 0,0, got {} {}", tx, ty);
    }

    #[test]
    fn texture_translation_1() {
        // The texture should transform around its origin point (defined by the first two parameters to fill_texture)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 0.0, 0.0, 1000.0, 1000.0);

        // Translate it: translations should use the coordinates we set up in `fill_texture`
        drawing.current_state.fill_transform(Transform2D::translate(-500.0, -500.0));

        // Point 0,0 on the canvas should be 50, 100 (ie, the midpoint) of the texture
        let (x,y)       = drawing.current_state.transform.transform_point(0.0, 0.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((tx - 50.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx, ty);
        assert!((ty - 100.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx, ty);
    }

    #[test]
    fn texture_translation_rotation_1() {
        // The texture should transform around its origin point (defined by the first two parameters to fill_texture)
        let mut drawing = CanvasDrawing::<U32LinearPixel, 4>::empty();

        // Set up a basic texture and a canvas height
        drawing.current_state.canvas_height(1000.0);
        drawing.texture(TextureId(0), TextureOp::Create(TextureSize(100, 200), TextureFormat::Rgba));
        drawing.texture(TextureId(0), TextureOp::SetBytes(TexturePosition(0, 0), TextureSize(0, 0), Arc::new(vec![])));

        // Set the texture as the fill texture to set up the initial transformation
        drawing.fill_texture(TextureId(0), 0.0, 0.0, 1000.0, 1000.0);

        // Translate it
        drawing.current_state.fill_transform(Transform2D::translate(-500.0, -500.0));

        // Point 0,0 on the canvas should be 50, 100 (ie, the midpoint) of the texture
        let (x,y)       = drawing.current_state.transform.transform_point(0.0, 0.0);
        let (tx, ty)    = texture_transform(&drawing.current_state).transform_point(x, y);

        // Should rotate around the new 0,0 point (we use the calculated value in case there's an issue with translation)
        drawing.current_state.fill_transform(Transform2D::rotate_degrees(45.0));

        let (rx, ry)    = texture_transform(&drawing.current_state).transform_point(x, y);

        assert!((rx - tx).abs() < 0.01, "Expected {}, {}, got {} {}", tx, ty, rx, ry);
        assert!((ry - ty).abs() < 0.01, "Expected {}, {}, got {} {}", tx, ty, rx, ry);
    }

    #[test]
    fn transform_test_1() {
        let w = 100.0;
        let h = 200.0;
        let x1 = 200.0;
        let y1 = 200.0;
        let x2 = 800.0;
        let y2 = 800.0;

        let transform = canvas::Transform2D::identity();

        // Transform so that the texture coordinates map from (0,0) to (x2-x1, y2-y1)
        let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
        let transform = canvas::Transform2D::scale(w, h) * transform;

        // Apply custom transformations
        let transform2 = canvas::Transform2D::translate(-300.0, -300.0).invert().unwrap();
        let transform2 = transform2 * canvas::Transform2D::rotate_degrees(45.0).invert().unwrap();
        let transform2 = transform2 * canvas::Transform2D::scale(1.0/3.0, 1.0/3.0).invert().unwrap();

        // Apply the custom transform to the mapping
        let transform = transform * transform2;

        // Transform the result to the final position
        let transform = transform * canvas::Transform2D::translate(-x1, -y1);

        // Center point of the texture should map to the 200, 200 coordinate we passed in
        let (tx1, ty1) = transform.transform_point(200.0, 200.0);

        assert!((tx1 - 50.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx1, ty1);
        assert!((ty1 - 100.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx1, ty1);
    }

    #[test]
    fn transform_test_2() {
        let w = 100.0;
        let h = 200.0;
        let x1 = 200.0;
        let y1 = 800.0;
        let x2 = 800.0;
        let y2 = 200.0;

        let transform = canvas::Transform2D::identity();

        // Transform so that the texture coordinates map from (0,0) to (x2-x1, y2-y1)
        let transform = canvas::Transform2D::scale(1.0/(x2-x1), 1.0/(y2-y1)) * transform;
        let transform = canvas::Transform2D::scale(w, h) * transform;

        // Apply custom transformations
        let transform2 = canvas::Transform2D::translate(-300.0, -300.0).invert().unwrap();
        let transform2 = transform2 * canvas::Transform2D::rotate_degrees(45.0).invert().unwrap();
        let transform2 = transform2 * canvas::Transform2D::scale(1.0/3.0, 1.0/3.0).invert().unwrap();

        // Apply the custom transform to the mapping
        let transform = transform * transform2;

        // Transform the result to the final position
        let transform = transform * canvas::Transform2D::translate(-x1, -y1);

        // Center point of the texture should map to the 200, 800 coordinate we passed in
        let (tx1, ty1) = transform.transform_point(200.0, 800.0);
        let (tx1, ty1) = (tx1.rem_euclid(100.0), ty1.rem_euclid(200.0));

        assert!((tx1 - 50.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx1, ty1);
        assert!((ty1 - 100.0).abs() < 0.01, "Expected 50, 100, got {} {}", tx1, ty1);
    }
}
