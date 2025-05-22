///
/// Describes the size of a frame in pixels
///
#[derive(Copy, Clone, Debug)]
pub struct FrameSize {
    pub width:  usize,
    pub height: usize,    
}

///
/// Describes the size of a frame in pixels, and the gamma correction value to apply to the rendering result
///
/// This is used when the target frame buffer uses a non-linear colour space (as is typical when rendering to RGBA bytes)
///
#[derive(Copy, Clone, Debug)]
pub struct GammaFrameSize {
    pub width:  usize,
    pub height: usize,    
    pub gamma:  f64,
}

///
/// Converts a FrameSize into a GammaFrameSize (uses the standard 2.2 gamma value)
///
impl From<FrameSize> for GammaFrameSize {
    #[inline]
    fn from(frame_size: FrameSize) -> GammaFrameSize {
        GammaFrameSize { 
            width:  frame_size.width, 
            height: frame_size.height, 
            gamma:  2.2 }
    }
}

impl From<GammaFrameSize> for FrameSize {
    #[inline]
    fn from(frame_size: GammaFrameSize) -> FrameSize {
        FrameSize { 
            width:  frame_size.width, 
            height: frame_size.height
        }
    }
}
