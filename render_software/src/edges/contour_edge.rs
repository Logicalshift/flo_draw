use crate::edgeplan::*;

use flo_canvas::curves::bezier::vectorize::*;

use smallvec::*;

///
/// A contour edge provides an implementation of an edge for any type that implements the `SampledContour` trait
///
pub struct ContourEdge<TContour> 
where
    TContour: SampledContour,
{
    /// The offset of the corner of where the contour should appear in space
    corner_offset: (f64, f64),

    /// The ID of the shape that this contour surrounds
    shape_id: ShapeId,

    /// The contour itself
    contour: TContour,
}

impl<TContour> ContourEdge<TContour>
where
    TContour: SampledContour,
{
    ///
    /// Creates a new edge description from a sampled contour
    ///
    pub fn new(corner_offset: (f64, f64), shape_id: ShapeId, contour: TContour) -> Self {
        ContourEdge {
            corner_offset,
            shape_id,
            contour
        }
    }
}

impl<TContour> EdgeDescriptor for ContourEdge<TContour>
where
    TContour: SampledContour,
{
    #[inline]
    fn shape(&self) -> ShapeId { 
        self.shape_id
    }

    fn bounding_box(&self) -> ((f64, f64), (f64, f64)) { 
        let ContourSize(w, h)   = self.contour.contour_size();
        let (w, h)              = (w as f64, h as f64);
        let (x, y)              = self.corner_offset;

        (self.corner_offset, (x+w, y+h))
    }

    #[inline]
    fn intercepts(&self, y_pos: f64) -> SmallVec<[(EdgeInterceptDirection, f64); 2]> {
        let ContourSize(_, h)   = self.contour.contour_size();
        let h                   = h as f64;
        let (x, y)              = self.corner_offset;

        let y_pos = y_pos - y;
        if y_pos < 0.0 || y_pos >= h {
            smallvec![]
        } else {
            self.contour.intercepts_on_line(y_pos).into_iter()
                .flat_map(|intercept| [(EdgeInterceptDirection::Toggle, intercept.start + x), (EdgeInterceptDirection::Toggle, intercept.end + x)])
                .collect()
        }
    }
}
