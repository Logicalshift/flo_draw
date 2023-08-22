use super::renderer::*;
use super::render_slice::*;

use crate::edgeplan::*;
use crate::scanplan::*;

use std::marker::{PhantomData};

///
/// The edge plan region renderer renders blocks of scanlines from an edge plan, supplied as y coordinates. It will work
/// with any kind of renderer that takes scanline plans (ScanlineRenderer being the most simple example of this)
///
pub struct EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer> 
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner,
    TLineRenderer:  Renderer<Region=f64, Source=ScanlinePlan>,
{
    edge_plan:      PhantomData<TEdge>,
    scan_planner:   TPlanner,
    line_renderer:  TLineRenderer,
}

impl<TEdge, TPlanner, TLineRenderer> EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer>
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner,
    TLineRenderer:  Renderer<Region=f64, Source=ScanlinePlan>,
{
    ///
    /// Creates a new region renderer
    ///
    pub fn new(scan_planner: TPlanner, line_renderer: TLineRenderer) -> Self {
        Self {
            edge_plan:      PhantomData, 
            scan_planner:   scan_planner, 
            line_renderer:  line_renderer,
        }
    }
}

impl<'a, TEdge, TPlanner, TLineRenderer> Renderer for &'a EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer>
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner<Edge=TEdge>,
    TLineRenderer:  Renderer<Region=f64, Source=ScanlinePlan>,
{
    type Region = RenderSlice;
    type Source = EdgePlan<TEdge>;
    type Dest   = [&'a mut TLineRenderer::Dest];

    fn render(&self, region: &RenderSlice, source: &EdgePlan<TEdge>, dest: &mut [&'a mut TLineRenderer::Dest]) {
        let y_positions = &region.y_positions;
        let width       = region.width as f64;
        let edge_plan   = source;

        // Plan the lines
        let mut scanlines = vec![(0.0f64, ScanlinePlan::default()); y_positions.len()];
        self.scan_planner.plan_scanlines(edge_plan, y_positions, 0.0..width, &mut scanlines);

        // Pass them on to the line renderer to generate the result
        for idx in 0..y_positions.len() {
            let (ypos, scanline) = &scanlines[idx];

            self.line_renderer.render(ypos, scanline, dest[idx]);
        }
    }
}
