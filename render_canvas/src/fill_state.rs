use super::matrix::*;

use flo_canvas as canvas;
use flo_render as render;

///
/// The ways the next path can be filled
///
#[derive(Clone, PartialEq, Debug)]
pub enum FillState {
    ///
    /// Unknown fill state
    ///
    None,

    ///
    /// Fill with a solid colour
    ///
    Color(render::Rgba8),

    ///
    /// Fill with a particular texture
    ///
    Texture(canvas::TextureId, render::Matrix, bool, f32),

    ///
    /// Fill with a particular gradient
    ///
    LinearGradient(canvas::GradientId, render::Matrix, bool, f32)
}

impl FillState {
    ///
    /// Returns the flat colour to use for this fill state
    ///
    pub fn flat_color(&self) -> render::Rgba8 {
        match self {
            FillState::None                         => render::Rgba8([0, 0, 0, 255]),
            FillState::Color(color)                 => *color,
            FillState::Texture(_, _, _, _)          => render::Rgba8([0, 0, 0, 255]),
            FillState::LinearGradient(_, _, _, _)   => render::Rgba8([0, 0, 0, 255])
        }
    }

    ///
    /// Creates a texture fill 
    ///
    pub fn texture_fill(texture_id: canvas::TextureId, x1: f32, y1: f32, x2: f32, y2: f32, alpha: f32) -> FillState {
        // Avoid division by zero
        let x2 = if x2 == x1 { x1 + 0.0000001 } else { x2 };
        let y2 = if y2 == y1 { y1 + 0.0000001 } else { y2 };

        // Generate a matrix that transforms x1, y1 to 0,0 and x2, y2 to 1,1
        let a       = 1.0/(x2-x1);
        let b       = 0.0;
        let c       = -x1 * a;

        let d       = 0.0;
        let e       = 1.0/(y2-y1);
        let f       = -y1 * e;

        let matrix  = render::Matrix([
            [a,   b,   0.0, c  ],
            [d,   e,   0.0, f  ],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]);

        // Create the fill-state for this matrix
        FillState::Texture(texture_id, matrix, true, alpha)
    }

    ///
    /// Creates a linear gradient fill
    ///
    pub fn linear_gradient_fill(gradient_id: canvas::GradientId, x1: f32, y1: f32, x2: f32, y2: f32) -> FillState {
        // Avoid division by zero
        let x2 = if x2 == x1 { x1 + 0.0000001 } else { x2 };
        let y2 = if y2 == y1 { y1 + 0.0000001 } else { y2 };

        let dx      = x2-x1;
        let dy      = y2-y1;

        let theta   = f32::atan2(dy, dx);
        let scale   = 1.0/f32::sqrt(dx*dx + dy*dy);

        let cos     = f32::cos(-theta);
        let sin     = f32::sin(-theta);

        let a       = cos * scale;
        let b       = -sin * scale;
        let d       = sin * scale;
        let e       = cos * scale;

        let c       = -x1 * a - y1 * b;
        let f       = -x1 * d - y1 * e;

        // Assemble into a matrix
        let matrix  = render::Matrix([
            [a,   b,   0.0, c  ],
            [d,   e,   0.0, f  ],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0]
        ]);

        // Create the fill-state for this matrix
        FillState::LinearGradient(gradient_id, matrix, false, 1.0)
    }

    ///
    /// Returns the ID of the texture used by this state
    ///
    pub fn texture_id(&self) -> Option<canvas::TextureId> {
        match self {
            FillState::None                         => None,
            FillState::Color(_)                     => None,
            FillState::Texture(texture_id, _, _, _) => Some(*texture_id),
            FillState::LinearGradient(_, _, _, _)   => None
        }
    }

    ///
    /// Updates the fill state with a new texture alpha
    ///
    pub fn with_texture_alpha(&self, new_alpha: f32) -> Self {
        match self {
            FillState::None                                     => self.clone(),
            FillState::Color(_)                                 => self.clone(),
            FillState::Texture(texture_id, matrix, repeat, _)   => FillState::Texture(*texture_id, *matrix, *repeat, new_alpha),
            FillState::LinearGradient(_, _, _, _)               => self.clone()
        }
    }

    ///
    /// Updates the fill state with a transformed matrix
    ///
    pub fn transform(&self, transform_matrix: &canvas::Transform2D) -> Self {
        let transform_matrix = transform_to_matrix(&transform_matrix);

        match self {
            FillState::None                                                 => self.clone(),
            FillState::Color(_)                                             => self.clone(),
            FillState::Texture(texture_id, matrix, repeat, alpha)           => FillState::Texture(*texture_id, (*matrix).multiply(transform_matrix), *repeat, *alpha),
            FillState::LinearGradient(texture_id, matrix, repeat, alpha)    => FillState::LinearGradient(*texture_id, (*matrix).multiply(transform_matrix), *repeat, *alpha)
        }
    }
}
