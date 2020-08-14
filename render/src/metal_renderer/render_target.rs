use metal;

///
/// Describes a Metal render target
///
#[derive(Clone)]
pub enum RenderTarget {
    /// Simple texture
    Texture {
        texture:    metal::Texture,
        width:      usize,
        height:     usize
    },

    /// MSAA render target
    MSAA {
        samples:    metal::Texture,
        resolved:   metal::Texture,
        width:      usize,
        height:     usize
    }
}

impl RenderTarget {
    ///
    /// Returns the width and height of this render target
    ///
    fn size(&self) -> (usize, usize) {
        match self {
            RenderTarget::Texture { texture: _, width, height }             => (*width, *height),
            RenderTarget::MSAA { samples: _, resolved: _, width, height }   => (*width, *height)
        }
    }
}
