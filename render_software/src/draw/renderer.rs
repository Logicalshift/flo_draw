use super::canvas_drawing::*;

use crate::pixel::*;
use crate::render::*;

use std::marker::{PhantomData};

///
/// Renders sections of a canvas drawing
///
pub struct CanvasDrawingRegionRenderer<TPixel, const N: usize>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    pixel: PhantomData<TPixel>,
}

impl<TPixel, const N: usize> Renderer for CanvasDrawingRegionRenderer<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    type Region = RenderSlice;
    type Source = CanvasDrawing<TPixel, N>;
    type Dest   = [TPixel];

    fn render(&self, region: &RenderSlice, source: &CanvasDrawing<TPixel, N>, dest: &mut [TPixel]) {
        todo!()
    }
}
