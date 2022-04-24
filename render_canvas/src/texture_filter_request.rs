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

    ///
    /// Add transparency to the image, where a value of 1.0 is opaque (leave the image as is), and 0.0 is transparent (no image result)
    ///
    AlphaBlend(f32),

    ///
    /// Use the alpha channel of a source texture as a mask for the input texture
    ///
    Mask(render::TextureId),

    ///
    /// Use the red and green channels of a source texture as a displacement map. The two other parameters are the scale factors (maximum displacement in canvas units, or
    /// pixels if no transform is supplied)
    ///
    DisplacementMap(render::TextureId, f32, f32, Option<canvas::Transform2D>),
}

impl TextureFilterRequest {
    ///
    /// Returns the radius of this transform in canvas units
    ///
    pub fn radius(&self) -> f32 {
        use TextureFilterRequest::*;

        match self {
            // The pixel blur does affect a texture in canvas units, so its radius is considered to be 0
            PixelBlur(_)                    => 0.0,
            AlphaBlend(_)                   => 0.0,
            Mask(_)                         => 0.0,

            DisplacementMap(_, _x_r, _y_r, None)            => 0.0,
            DisplacementMap(_, x_r, y_r, Some(transform))   => {
                let (x1, y1)    = transform.transform_point(0.0, 0.0);
                let (x2, y2)    = transform.transform_point(*x_r, *y_r);

                let min_x       = f32::min(x1, x2);
                let min_y       = f32::min(y1, y2);
                let max_x       = f32::max(x1, x2);
                let max_y       = f32::max(y1, y2);

                // Size relative to the framebuffer size
                let size_w      = (max_x - min_x)/2.0;
                let size_h      = (max_y - min_y)/2.0;

                (size_w*size_w + size_h*size_h).sqrt()
            }

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
            PixelBlur(_)                            => vec![],
            CanvasBlur(_, _)                        => vec![],
            AlphaBlend(_)                           => vec![],
            Mask(texture_id)                        => vec![*texture_id],
            DisplacementMap(texture_id, _, _, _)    => vec![*texture_id],
        }
    }
}
