use crate::edgeplan::*;
use crate::pixel::*;
use crate::scanplan::*;
use crate::render::*;

use std::ops::{Range};
use std::sync::*;
use std::marker::{PhantomData};

///
/// A sprite is a renderer that can be run as a pixel program. This can be used for repeatedly re-rendering a shape
/// with a performance boost from bypassing the need to perform many of the usual preparation steps.
///
/// Sprite programs are generally drawn as transparent so they can blend with the pixels underneath but can potentially
/// be rendered more efficiently if the algorithm is able to detect opaque areas.
///
pub struct BasicSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static,
{
    /// The scan planner is used for planning how the edges should be rendered for this sprite
    planner: TPlanner,

    /// Data types used by the pixel program implementation
    phantom_data: PhantomData<(TPixel, BasicSpriteData<TEdgeDescriptor>)>,
}

///
/// Data that can be used to run a basic sprite program
///
pub struct BasicSpriteData<TEdgeDescriptor>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    /// The edges for the sprite program
    edges: Arc<EdgePlan<TEdgeDescriptor>>,

    /// The scaling to apply to coordinates supplied to the edge plan
    scale: (f64, f64),

    /// The translation to apply to coordinates supplied to the edge plan
    translate: (f64, f64),
}

impl<TEdgeDescriptor> BasicSpriteData<TEdgeDescriptor>
where
    TEdgeDescriptor: EdgeDescriptor,
{
    ///
    /// Creates a new instance of the data for the basic sprite pixel program
    ///
    pub fn new(edges: Arc<EdgePlan<TEdgeDescriptor>>, scale: (f64, f64), translate: (f64, f64)) -> Self {
        BasicSpriteData { edges, scale, translate }
    }
}

impl<TPixel, TEdgeDescriptor, TPlanner> PixelProgram for BasicSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static + Send + Sync + Copy + AlphaBlend,
    TPlanner:            Send + Sync + ScanPlanner<Edge=TEdgeDescriptor>,
{
    type Pixel          = TPixel;
    type ProgramData    = BasicSpriteData<TEdgeDescriptor>;

    fn draw_pixels(&self, data_cache: &PixelProgramRenderCache<Self::Pixel>, target: &mut [Self::Pixel], x_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64, data: &Self::ProgramData) {
        // Calculate the transform for the sprite region
        let sprite_ypos         = y_pos * data.scale.1 + data.translate.1;
        let sprite_transform    = x_transform.transform(data.scale.0, data.translate.0);
        let sprite_xrange       = sprite_transform.pixel_x_to_source_x(x_range.start)..sprite_transform.pixel_x_to_source_x(x_range.end);

        // Plan the rendering for the sprite
        // TODO: we might render the same sprite multiple times on a line, in which case it would be faster to do this once and re-use it later on, maybe can exploit that 
        // on one thread this is called for the same line repeatedly. We'll need to clip to the pixel range though, so will need to test to know if this is worth the extra effort
        //
        // It may also be possible to cache scaline plans for longer to re-use when the sprite is rendered in multiple positions (though this requires pixel-perfect alignment so
        // may be even less useful)
        let mut scanline = [(sprite_ypos, ScanlinePlan::default())];
        self.planner.plan_scanlines(&*data.edges, &sprite_transform, &[sprite_ypos], sprite_xrange, &mut scanline);

        // Render the scanplan to the pixels using a scanline renderer (which should appropriately blend transparent pixels)
        let scanplan = &scanline[0].1;
        let renderer = ScanlineRenderer::new(data_cache);
        let region   = ScanlineRenderRegion { y_pos: sprite_ypos, transform: sprite_transform };

        renderer.render(&region, scanplan, target);
    }
}

impl<TPixel, TEdgeDescriptor, TPlanner> Default for BasicSpriteProgram<TPixel, TEdgeDescriptor, TPlanner>
where
    TEdgeDescriptor:    'static + EdgeDescriptor,
    TPixel:             'static,
    TPlanner:           Default,
{
    #[inline]
    fn default() -> Self {
        BasicSpriteProgram { 
            planner:        TPlanner::default(), 
            phantom_data:   PhantomData
        }
    }
}
