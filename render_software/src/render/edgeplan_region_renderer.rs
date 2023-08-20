use super::renderer::*;

use crate::edgeplan::*;
use crate::scanplan::*;

///
/// The edge plan region renderer renders blocks of scanlines from an edge plan, supplied as y coordinates. It will work
/// with any kind of renderer that takes scanline plans (ScanlineRenderer being the most simple example of this)
///
pub struct EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer> 
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner,
    TLineRenderer:  Renderer<Source=(f64, ScanlinePlan)>,
{
    width:          f64,
    edge_plan:      EdgePlan<TEdge>,
    scan_planner:   TPlanner,
    line_renderer:  TLineRenderer,
}

impl<TEdge, TPlanner, TLineRenderer> EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer>
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner,
    TLineRenderer:  Renderer<Source=(f64, ScanlinePlan)>,
{
    ///
    /// Creates a new region renderer
    ///
    pub fn new(width: usize, edge_plan: EdgePlan<TEdge>, scan_planner: TPlanner, line_renderer: TLineRenderer) -> Self {
        let width = width as f64;

        Self {
            width, edge_plan, scan_planner, line_renderer,
        }
    }
}

impl<'a, TEdge, TPlanner, TLineRenderer> Renderer for &'a EdgePlanRegionRenderer<TEdge, TPlanner, TLineRenderer>
where
    TEdge:          EdgeDescriptor,
    TPlanner:       ScanPlanner<Edge=TEdge>,
    TLineRenderer:  Renderer<Source=(f64, ScanlinePlan)>,
{
    type Source = [f64];
    type Dest   = [&'a mut TLineRenderer::Dest];

    fn render(&self, source: &[f64], dest: &mut [&'a mut TLineRenderer::Dest]) {
        // Plan the lines
        let mut scanlines = vec![(0.0, ScanlinePlan::default()); source.len()];
        self.scan_planner.plan_scanlines(&self.edge_plan, source, 0.0..self.width, &mut scanlines);

        // Pass them on to the line renderer to generate the result
        for idx in 0..source.len() {
            self.line_renderer.render(&scanlines[idx], dest[idx]);
        }
    }
}
