use flo_canvas as canvas;
use flo_render as render;

///
/// Represents a request to perform a filter on a texture (replacing the texture with the filtered version) 
///
#[derive(Clone, Debug)]
pub enum TextureFilterRequest {
    ///
    /// Performs a gaussian blur with the specified radius measured in pixels
    ///
    /// This is used with textures that have a fixed size in pixels
    ///
    PixelBlur(f32),

    ///
    /// Performs a gaussian blur with a radius measured in canvas units
    ///
    /// This is used with dynamic textures where the pixel size is not defined.
    ///
    CanvasBlur(f32, canvas::Transform2D),
}

impl TextureFilterRequest {
    ///
    /// Returns the radius of this transform in canvas units
    ///
    pub fn radius(&self) -> f32 {
        use TextureFilterRequest::*;

        match self {
            // The pixel blur does affect a texture in canvas units, so its radius is considered to be 0
            PixelBlur(_) => 0.0,

            CanvasBlur(radius, transform) => {
                let (x1, y1)    = transform.transform_point(0.0, 0.0);
                let (x2, y2)    = transform.transform_point(*radius, *radius);

                let min_x       = f32::min(x1, x2);
                let min_y       = f32::min(y1, y2);
                let max_x       = f32::max(x1, x2);
                let max_y       = f32::max(y1, y2);

                // Size relative to the framebuffer size
                let size_w      = (max_x - min_x)/2.0;
                let size_h      = (max_y - min_y)/2.0;

                (size_w*size_w + size_h*size_h).sqrt()
            },
        }
    }

    ///
    /// Returns the textures used by this filter
    ///
    pub fn used_textures(&self) -> Vec<render::TextureId> {
        use TextureFilterRequest::*;

        match self {
            PixelBlur(_)        => vec![],
            CanvasBlur(_, _)    => vec![],
        }
    }
}
