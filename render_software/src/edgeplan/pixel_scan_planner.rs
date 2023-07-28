use super::edge_descriptor::*;
use super::edge_plan::*;

use crate::scanplan::*;

use std::ops::{Range};

///
/// Creates a pixel-precise scanline plan from an edge plan at a particular y position
///
/// This plan is created from the edge plan, and pixel-aligned to produce a 'jaggy' pixel-precise version of the plan.
/// Ie, no anti-aliasing of any kind is performed with this scanline plan.
///
pub fn plan_pixel_scanline<TEdge>(edge_plan: &EdgePlan<TEdge>, y_pos: f64, x_range: Range<i64>) -> ScanlinePlan 
where
    TEdge: EdgeDescriptor,
{
    todo!()
}
