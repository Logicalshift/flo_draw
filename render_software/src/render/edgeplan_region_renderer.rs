use super::renderer::*;

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
    TLineRenderer:  Renderer<Source=(f64, ScanlinePlan)>,
{
    width:          f64,
    edge_plan:      PhantomData<TEdge>,
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
    pub fn new(width: usize, scan_planner: TPlanner, line_renderer: TLineRenderer) -> Self {
        let width = width as f64;

        Self {
            width:          width,
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
    TLineRenderer:  Renderer<Source=(f64, ScanlinePlan)>,
{
    type Source = (&'a [f64], &'a EdgePlan<TEdge>);
    type Dest   = [&'a mut TLineRenderer::Dest];

    fn render(&self, source: &(&'a [f64], &'a EdgePlan<TEdge>), dest: &mut [&'a mut TLineRenderer::Dest]) {
        let (y_positions, edge_plan) = source;

        // Plan the lines
        let mut scanlines = vec![(0.0, ScanlinePlan::default()); y_positions.len()];
        self.scan_planner.plan_scanlines(edge_plan, y_positions, 0.0..self.width, &mut scanlines);

        // Pass them on to the line renderer to generate the result
        for idx in 0..y_positions.len() {
            self.line_renderer.render(&scanlines[idx], dest[idx]);
        }
    }
}
