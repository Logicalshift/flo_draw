use super::draw::*;

use std::result::Result;

/*
            &NewPath                                => ('N', 'p').encode_canvas(append_to),
            &Move(x, y)                             => ('m', x, y).encode_canvas(append_to),
            &Line(x, y)                             => ('l', x, y).encode_canvas(append_to),
            &BezierCurve(p1, p2, p3)                => ('c', p1, p2, p3).encode_canvas(append_to),
            &ClosePath                              => ('.').encode_canvas(append_to),
            &Fill                                   => 'F'.encode_canvas(append_to),
            &Stroke                                 => 'S'.encode_canvas(append_to),
            &LineWidth(width)                       => ('L', 'w', width).encode_canvas(append_to),
            &LineWidthPixels(width)                 => ('L', 'p', width).encode_canvas(append_to),
            &LineJoin(join)                         => ('L', 'j', join).encode_canvas(append_to),
            &LineCap(cap)                           => ('L', 'c', cap).encode_canvas(append_to),
            &NewDashPattern                         => ('D', 'n').encode_canvas(append_to),
            &DashLength(length)                     => ('D', 'l', length).encode_canvas(append_to),
            &DashOffset(offset)                     => ('D', 'o', offset).encode_canvas(append_to),
            &StrokeColor(col)                       => ('C', 's', col).encode_canvas(append_to),
            &FillColor(col)                         => ('C', 'f', col).encode_canvas(append_to),
            &BlendMode(mode)                        => ('M', mode).encode_canvas(append_to),
            &IdentityTransform                      => ('T', 'i').encode_canvas(append_to),
            &CanvasHeight(height)                   => ('T', 'h', height).encode_canvas(append_to),
            &CenterRegion(min, max)                 => ('T', 'c', min, max).encode_canvas(append_to),
            &MultiplyTransform(transform)           => ('T', 'm', transform).encode_canvas(append_to),
            &Unclip                                 => ('Z', 'n').encode_canvas(append_to),
            &Clip                                   => ('Z', 'c').encode_canvas(append_to),
            &Store                                  => ('Z', 's').encode_canvas(append_to),
            &Restore                                => ('Z', 'r').encode_canvas(append_to),
            &FreeStoredBuffer                       => ('Z', 'f').encode_canvas(append_to),
            &PushState                              => 'P'.encode_canvas(append_to),
            &PopState                               => 'p'.encode_canvas(append_to),
            &ClearCanvas                            => ('N', 'A').encode_canvas(append_to),
            &Layer(layer_id)                        => ('N', 'l', layer_id).encode_canvas(append_to),
            &LayerBlend(layer_id, blend_mode)       => ('N', 'b', layer_id, blend_mode).encode_canvas(append_to),
            &ClearLayer                             => ('N', 'C').encode_canvas(append_to)
 */

enum DecoderState {
    None,
    
    New,                            // 'N'
    LineStyle,                      // 'L'
    Dash,                           // 'D'
    Color,                          // 'C'
    Transform,                      // 'T'
    State,                          // 'Z'

    Move(String),                   // m (x, y)
    Line(String),                   // l (x, y)
    BezierCurve(String),            // c (x, y, x, y, x, y)

    LineStyleWidth(String),         // 'Lw' (w)
    LineStyleWidthPixels(String),   // 'Lp' (w)
    LineStyleJoin(String),          // 'Lj' (j)
    LineStyleCap(String),           // 'Lc' (c)

    DashLength(String),             // 'Dl' (len)
    DashOffset(String),             // 'Do' (offset)

    ColorStroke(String),            // 'Cs' (r, g, b, a)
    ColorFill(String),              // 'Cf' (r, g, b, a)

    BlendMode(String),              // 'M' (mode)

    TransformHeight(String),        // 'Th' (h)
    TransformCenter(String),        // 'Tc' (min, max)
    TransformMultiply(String),      // 'Tm' (transform)

    StateLayer(String),             // 'Nl' (id)
    StateLayerBlend(String),        // 'Nb' (id, mode)
}

///
/// Possible error from the decoder
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecoderError {
    InvalidCharacter(char)
}

///
/// Represents a (stateful) canvas decoder
///
pub struct CanvasDecoder {
    state: DecoderState
}

impl CanvasDecoder {
    ///
    /// Creates a new canvas decoder
    ///
    pub fn new() -> CanvasDecoder {
        CanvasDecoder {
            state: DecoderState::None
        }
    }

    ///
    /// Decodes a character, returning the next Draw operation if there is one
    ///
    pub fn decode(&mut self, next_chr: char) -> Result<Option<Draw>, DecoderError> {
        use self::DecoderState::*;

        // Next state depends on the character and the current state
        let (next_state, result) = match self.state {
            None                    => Self::decode_none(next_chr)?,

            New                     => Self::decode_new(next_chr)?,
            Dash                    => Self::decode_dash(next_chr)?,
            Color                   => Self::decode_color(next_chr)?,
            Transform               => Self::decode_transform(next_chr)?,
            State                   => Self::decode_state(next_chr)?,

            _                       => unimplemented!()
        };

        self.state = next_state;
        Ok(result)
    }

    ///
    /// Matches the first character of a canvas item
    ///
    #[inline] fn decode_none(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            // Multi-character commands
            'N' => Ok((DecoderState::New, None)),
            'L' => Ok((DecoderState::LineStyle, None)),
            'D' => Ok((DecoderState::Dash, None)),
            'C' => Ok((DecoderState::Color, None)),
            'T' => Ok((DecoderState::Transform, None)),
            'Z' => Ok((DecoderState::State, None)),

            // Single character commands
            '.' => Ok((DecoderState::None, Some(Draw::ClosePath))),
            'F' => Ok((DecoderState::None, Some(Draw::Fill))),
            'S' => Ok((DecoderState::None, Some(Draw::Stroke))),
            'P' => Ok((DecoderState::None, Some(Draw::PushState))),
            'p' => Ok((DecoderState::None, Some(Draw::PopState))),

            // Single character commands with a parameter
            'm' => Ok((DecoderState::Move(String::new()), None)),
            'l' => Ok((DecoderState::Line(String::new()), None)),
            'c' => Ok((DecoderState::BezierCurve(String::new()), None)),
            'M' => Ok((DecoderState::BlendMode(String::new()), None)),

            // Other characters are not accepted
            _   => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_new(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        unimplemented!()
    }

    #[inline] fn decode_dash(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        unimplemented!()
    }

    #[inline] fn decode_color(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        unimplemented!()
    }

    #[inline] fn decode_transform(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        unimplemented!()
    }

    #[inline] fn decode_state(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::color::*;
    use super::super::encoding::*;
    use super::super::transform2d::*;

    ///
    /// Checks if a particular drawing operation can be both encoded and decoded
    ///
    fn check_can_round_trip(instruction: Draw) {
        // Encode the instruction
        let mut encoded = String::new();
        instruction.encode_canvas(&mut encoded);

        // Try decoding it
        let mut decoder = CanvasDecoder::new();
        let mut decoded = None;

        for c in encoded.chars() {
            // As we've encoded a single instruction we should never start with a valid value
            assert!(decoded.is_none());

            // Update with the next state
            decoded = decoder.decode(c).unwrap();
        }

        // Should decode OK
        assert!(decoded.is_some());

        // Should be the same as the original instruction
        assert!(decoded == Some(instruction));
    }

    #[test]
    fn decode_new_path() {
        check_can_round_trip(Draw::NewPath);
    }

    #[test]
    fn decode_move() {
        check_can_round_trip(Draw::Move(10.0, 15.0));
    }

    #[test]
    fn decode_line() {
        check_can_round_trip(Draw::Line(20.0, 42.0));
    }

    #[test]
    fn decode_bezier_curve() {
        check_can_round_trip(Draw::BezierCurve((1.0, 2.0), (3.0, 4.0), (5.0, 6.0)));
    }

    #[test]
    fn decode_close_path() {
        check_can_round_trip(Draw::ClosePath);
    }

    #[test]
    fn decode_fill() {
        check_can_round_trip(Draw::Fill);
    }

    #[test]
    fn decode_stroke() {
        check_can_round_trip(Draw::Stroke);
    }

    #[test]
    fn decode_line_width() {
        check_can_round_trip(Draw::LineWidth(23.0));
    }

    #[test]
    fn decode_line_width_pixels() {
        check_can_round_trip(Draw::LineWidthPixels(43.0));
    }

    #[test]
    fn decode_line_join() {
        check_can_round_trip(Draw::LineJoin(LineJoin::Bevel));
    }

    #[test]
    fn decode_line_cap() {
        check_can_round_trip(Draw::LineCap(LineCap::Round));
    }

    #[test]
    fn decode_new_dash_pattern() {
        check_can_round_trip(Draw::NewDashPattern);
    }

    #[test]
    fn decode_dash_length() {
        check_can_round_trip(Draw::DashLength(56.0));
    }

    #[test]
    fn decode_dash_offset() {
        check_can_round_trip(Draw::DashOffset(13.0));
    }

    #[test]
    fn decode_stroke_color() {
        check_can_round_trip(Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)));
    }

    #[test]
    fn decode_fill_color() {
        check_can_round_trip(Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)));
    }

    #[test]
    fn decode_blend_mode() {
        check_can_round_trip(Draw::BlendMode(BlendMode::Lighten));
    }

    #[test]
    fn decode_identity_transform() {
        check_can_round_trip(Draw::IdentityTransform);
    }

    #[test]
    fn decode_canvas_height() {
        check_can_round_trip(Draw::CanvasHeight(81.0));
    }

    #[test]
    fn decode_center_region() {
        check_can_round_trip(Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)));
    }

    #[test]
    fn decode_multiply_transform() {
        check_can_round_trip(Draw::MultiplyTransform(Transform2D((1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0))));
    }

    #[test]
    fn decode_unclip() {
        check_can_round_trip(Draw::Unclip);
    }

    #[test]
    fn decode_clip() {
        check_can_round_trip(Draw::Clip)
    }

    #[test]
    fn decode_store() {
        check_can_round_trip(Draw::Store);
    }

    #[test]
    fn decode_restore() {
        check_can_round_trip(Draw::Restore);
    }

    #[test]
    fn decode_free_stored_buffer() {
        check_can_round_trip(Draw::FreeStoredBuffer);
    }

    #[test]
    fn decode_push_state() {
        check_can_round_trip(Draw::PushState);
    }

    #[test]
    fn decode_pop_state() {
        check_can_round_trip(Draw::PopState);
    }

    #[test]
    fn decode_clear_canvas() {
        check_can_round_trip(Draw::ClearCanvas);
    }

    #[test]
    fn decode_layer() {
        check_can_round_trip(Draw::Layer(21));
    }

    #[test]
    fn decode_layer_blend() {
        check_can_round_trip(Draw::LayerBlend(76, BlendMode::Lighten))
    }

    #[test]
    fn decode_clear_layer() {
        check_can_round_trip(Draw::ClearLayer);
    }
}
