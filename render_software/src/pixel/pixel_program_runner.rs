use super::pixel_program_cache::*;

use std::ops::{Range};

///
/// Trait implemented by types that can run pixel programs (identified by their data ID)
///
/// `PixelProgramDataCache` is the one provided by this library, but this trait can be re-implemented to customise how a scene is rendered.
///
pub trait PixelProgramRunner<TPixel> {
    ///
    /// Runs a program with the data found at the `program_data` identifier, to render the pixels in `x_range` to `target`. The pixels will
    /// eventually be rendered at the specified y position in the frame.
    ///
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [TPixel], x_range: Range<i32>, y_pos: f64);
}
