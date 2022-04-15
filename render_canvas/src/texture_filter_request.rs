use flo_canvas as canvas;

///
/// Represents a request to perform a filter on a texture (replacing the texture with the filtered version) 
///
#[derive(Clone, Debug)]
pub enum TextureFilterRequest {
    ///
    /// Performs a gaussian blur with the specified radius measured in pixels
    ///
    /// This is used with 
    ///
    PixelBlur(f32),

    ///
    /// Performs a gaussian blur with a radius measured in canvas units
    ///
    /// This is used with dynamic textures where the pixel size is not defined.
    ///
    CanvasBlur(f32, canvas::Transform2D),
}
