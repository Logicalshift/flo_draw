use super::draw::*;
use super::color::*;
use super::transform2d::*;

use std::mem;
use std::str::*;
use std::result::Result;

///
/// The possible states for a decoder to be in after accepting some characters from the source
///
enum DecoderState {
    None,
    Error,
    
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

    NewLayer(String),               // 'Nl' (id)
    NewLayerBlend(String),          // 'Nb' (id, mode)
}

///
/// Possible error from the decoder
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecoderError {
    /// The character was not valid for the current state of the decoder
    InvalidCharacter(char),

    /// The decoder tried to decode something before it had accepted all characters (probably a bug)
    MissingCharacter,

    /// A number could not be parsed for some reason
    BadNumber,

    /// A color had an unknown type
    UnknownColorType,

    /// The decoder previously encountered an error and cannot continue
    IsInErrorState
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
        let mut state = DecoderState::Error;
        mem::swap(&mut self.state, &mut state);

        let (next_state, result) = match state {
            None                            => Self::decode_none(next_chr)?,
            Error                           => Err(DecoderError::IsInErrorState)?,

            New                             => Self::decode_new(next_chr)?,
            LineStyle                       => Self::decode_line_style(next_chr)?,
            Dash                            => Self::decode_dash(next_chr)?,
            Color                           => Self::decode_color(next_chr)?,
            Transform                       => Self::decode_transform(next_chr)?,
            State                           => Self::decode_state(next_chr)?,

            Move(param)                     => Self::decode_move(next_chr, param)?,
            Line(param)                     => Self::decode_line(next_chr, param)?,
            BezierCurve(param)              => Self::decode_bezier_curve(next_chr, param)?,

            LineStyleWidth(param)           => Self::decode_line_width(next_chr, param)?,
            LineStyleWidthPixels(param)     => Self::decode_line_width_pixels(next_chr, param)?,
            LineStyleJoin(param)            => Self::decode_line_style_join(next_chr, param)?,
            LineStyleCap(param)             => Self::decode_line_style_cap(next_chr, param)?,

            DashLength(param)               => Self::decode_dash_length(next_chr, param)?,
            DashOffset(param)               => Self::decode_dash_offset(next_chr, param)?,

            ColorStroke(param)              => Self::decode_color_stroke(next_chr, param)?,
            ColorFill(param)                => Self::decode_color_fill(next_chr, param)?,

            BlendMode(param)                => Self::decode_blend_mode(next_chr, param)?,

            TransformHeight(param)          => Self::decode_transform_height(next_chr, param)?,
            TransformCenter(param)          => Self::decode_transform_center(next_chr, param)?,
            TransformMultiply(param)        => Self::decode_transform_multiply(next_chr, param)?,

            NewLayer(param)                 => Self::decode_new_layer(next_chr, param)?,
            NewLayerBlend(param)            => Self::decode_new_layer_blend(next_chr, param)?
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
        // Matched 'N' so far
        match next_chr {
            'p'     => Ok((DecoderState::None, Some(Draw::NewPath))),
            'A'     => Ok((DecoderState::None, Some(Draw::ClearCanvas))),
            'C'     => Ok((DecoderState::None, Some(Draw::ClearLayer))),

            'l'     => Ok((DecoderState::NewLayer(String::new()), None)),
            'b'     => Ok((DecoderState::NewLayerBlend(String::new()), None)),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_line_style(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'L' so far
        match next_chr {
            'w'     => Ok((DecoderState::LineStyleWidth(String::new()), None)),
            'p'     => Ok((DecoderState::LineStyleWidthPixels(String::new()), None)),
            'j'     => Ok((DecoderState::LineStyleJoin(String::new()), None)),
            'c'     => Ok((DecoderState::LineStyleCap(String::new()), None)),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_dash(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'D' so far
        match next_chr {
            'n'     => Ok((DecoderState::None, Some(Draw::NewDashPattern))),

            'l'     => Ok((DecoderState::DashLength(String::new()), None)),
            'o'     => Ok((DecoderState::DashOffset(String::new()), None)),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_color(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'C' so far
        match next_chr {
            's'     => Ok((DecoderState::ColorStroke(String::new()), None)),
            'f'     => Ok((DecoderState::ColorFill(String::new()), None)),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_transform(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'T' so far
        match next_chr {
            'i'     => Ok((DecoderState::None, Some(Draw::IdentityTransform))),
            'h'     => Ok((DecoderState::TransformHeight(String::new()), None)),
            'c'     => Ok((DecoderState::TransformCenter(String::new()), None)),
            'm'     => Ok((DecoderState::TransformMultiply(String::new()), None)),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_state(next_chr: char) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        // Matched 'Z' so far
        match next_chr {
            'n'     => Ok((DecoderState::None, Some(Draw::Unclip))),
            'c'     => Ok((DecoderState::None, Some(Draw::Clip))),
            's'     => Ok((DecoderState::None, Some(Draw::Store))),
            'r'     => Ok((DecoderState::None, Some(Draw::Restore))),
            'f'     => Ok((DecoderState::None, Some(Draw::FreeStoredBuffer))),

            _       => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_line_width_pixels(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::LineStyleWidthPixels(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let width       = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::LineWidthPixels(width))))
        }
    }

    #[inline] fn decode_move(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::Move(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let x           = Self::decode_f32(&mut param)?;
            let y           = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::Move(x, y))))
        }
    }

    #[inline] fn decode_line(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 11 {
            param.push(next_chr);
            Ok((DecoderState::Line(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let x           = Self::decode_f32(&mut param)?;
            let y           = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::Line(x, y))))
        }
    }

    #[inline] fn decode_bezier_curve(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 35 {
            param.push(next_chr);
            Ok((DecoderState::BezierCurve(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let x1          = Self::decode_f32(&mut param)?;
            let y1          = Self::decode_f32(&mut param)?;
            let x2          = Self::decode_f32(&mut param)?;
            let y2          = Self::decode_f32(&mut param)?;
            let x3          = Self::decode_f32(&mut param)?;
            let y3          = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::BezierCurve((x1, y1), (x2, y2), (x3, y3)))))
        }
    }

    #[inline] fn decode_line_width(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::LineStyleWidth(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let width       = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::LineWidth(width))))
        }
    }

    #[inline] fn decode_line_style_join(next_chr: char, _param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'M' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Miter)))),
            'R' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Round)))),
            'B' => Ok((DecoderState::None, Some(Draw::LineJoin(LineJoin::Bevel)))),

            _ => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_line_style_cap(next_chr: char, _param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        match next_chr {
            'B' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Butt)))),
            'R' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Round)))),
            'S' => Ok((DecoderState::None, Some(Draw::LineCap(LineCap::Square)))),

            _ => Err(DecoderError::InvalidCharacter(next_chr))
        }
    }

    #[inline] fn decode_dash_length(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::DashLength(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((DecoderState::None, Some(Draw::DashLength(Self::decode_f32(&mut param)?))))
        }
    }

    #[inline] fn decode_dash_offset(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::DashOffset(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((DecoderState::None, Some(Draw::DashOffset(Self::decode_f32(&mut param)?))))
        }
    }

    #[inline] fn decode_color_stroke(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::ColorStroke(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let col_type    = param.next();
            let r           = Self::decode_f32(&mut param)?;
            let g           = Self::decode_f32(&mut param)?;
            let b           = Self::decode_f32(&mut param)?;
            let a           = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((DecoderState::None, Some(Draw::StrokeColor(Color::Rgba(r, g, b, a)))))
        }
    }

    #[inline] fn decode_color_fill(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 24 {
            param.push(next_chr);
            Ok((DecoderState::ColorFill(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let col_type    = param.next();
            let r           = Self::decode_f32(&mut param)?;
            let g           = Self::decode_f32(&mut param)?;
            let b           = Self::decode_f32(&mut param)?;
            let a           = Self::decode_f32(&mut param)?;

            if col_type != Some('R') {
                Err(DecoderError::UnknownColorType)?;
            }

            Ok((DecoderState::None, Some(Draw::FillColor(Color::Rgba(r, g, b, a)))))
        }
    }

    #[inline] fn decode_blend_mode(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 1 {
            param.push(next_chr);
            Ok((DecoderState::BlendMode(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((DecoderState::None, Some(Draw::BlendMode(Self::decode_blend_mode_only(&mut param)?))))
        }
    }

    #[inline] fn decode_transform_height(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::TransformHeight(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((DecoderState::None, Some(Draw::CanvasHeight(Self::decode_f32(&mut param)?))))
        }
    }

    #[inline] fn decode_transform_center(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 23 {
            param.push(next_chr);
            Ok((DecoderState::TransformCenter(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let min_x       = Self::decode_f32(&mut param)?;
            let min_y       = Self::decode_f32(&mut param)?;
            let max_x       = Self::decode_f32(&mut param)?;
            let max_y       = Self::decode_f32(&mut param)?;

            Ok((DecoderState::None, Some(Draw::CenterRegion((min_x, min_y), (max_x, max_y)))))
        }
    }

    #[inline] fn decode_transform_multiply(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 53 {
            param.push(next_chr);
            Ok((DecoderState::TransformMultiply(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();

            let mut matrix = [0.0; 9];
            for entry in 0..9 {
                matrix[entry] = Self::decode_f32(&mut param)?;
            }

            let transform = Transform2D((matrix[0], matrix[1], matrix[2]), (matrix[3], matrix[4], matrix[5]), (matrix[6], matrix[7], matrix[8]));

            Ok((DecoderState::None, Some(Draw::MultiplyTransform(transform))))
        }
    }

    #[inline] fn decode_new_layer(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 5 {
            param.push(next_chr);
            Ok((DecoderState::NewLayer(param), None))
        } else {
            param.push(next_chr);
            let mut param = param.chars();
            Ok((DecoderState::None, Some(Draw::Layer(Self::decode_u32(&mut param)?))))
        }
    }

    #[inline] fn decode_new_layer_blend(next_chr: char, mut param: String) -> Result<(DecoderState, Option<Draw>), DecoderError> {
        if param.len() < 7 {
            param.push(next_chr);
            Ok((DecoderState::NewLayerBlend(param), None))
        } else {
            param.push(next_chr);

            let mut param   = param.chars();
            let layer_id    = Self::decode_u32(&mut param)?;
            let blend_mode  = Self::decode_blend_mode_only(&mut param)?;

            Ok((DecoderState::None, Some(Draw::LayerBlend(layer_id, blend_mode))))
        }
    }

    ///
    /// Consumes 2 characters to decode a blend mode
    ///
    fn decode_blend_mode_only(param: &mut Chars) -> Result<BlendMode, DecoderError> {
        let (a, b)  = (param.next(), param.next());
        let a       = a.ok_or(DecoderError::MissingCharacter)?;
        let b       = b.ok_or(DecoderError::MissingCharacter)?;

        match (a, b) {
            ('S', 'V') => Ok(BlendMode::SourceOver),
            ('S', 'I') => Ok(BlendMode::SourceIn),
            ('S', 'O') => Ok(BlendMode::SourceOut),
            ('S', 'A') => Ok(BlendMode::SourceAtop),

            ('D', 'V') => Ok(BlendMode::DestinationOver),
            ('D', 'I') => Ok(BlendMode::DestinationIn),
            ('D', 'O') => Ok(BlendMode::DestinationOut),
            ('D', 'A') => Ok(BlendMode::DestinationAtop),

            ('E', 'M') => Ok(BlendMode::Multiply),
            ('E', 'S') => Ok(BlendMode::Screen),
            ('E', 'D') => Ok(BlendMode::Darken),
            ('E', 'L') => Ok(BlendMode::Lighten),

            _          => Err(DecoderError::InvalidCharacter(a))
        }
    }

    ///
    /// Consumes 6 characters to decode a f32
    ///
    fn decode_f32(chrs: &mut Chars) -> Result<f32, DecoderError> {
        let as_u32 = Self::decode_u32(chrs)?;
        let as_f32 = f32::from_bits(as_u32);

        Ok(as_f32)
    }

    ///
    /// Consumes 6 characters to decode a u32
    ///
    fn decode_u32(chrs: &mut Chars) -> Result<u32, DecoderError> {
        let mut result  = 0;
        let mut shift   = 0;

        for _ in 0..6 {
            let next_chr    = chrs.next().ok_or(DecoderError::BadNumber)?;
            result          |= (Self::decode_base64(next_chr)? as u32) << shift;
            shift           += 6;
        }


        Ok(result)
    }

    ///
    /// Decodes a base64 character to a number (in the range 0x00 -> 0x3f)
    ///
    #[inline] fn decode_base64(chr: char) -> Result<u8, DecoderError> {
        if chr >= 'A' && chr <= 'Z' {
            Ok((chr as u8) - ('A' as u8))
        } else if chr >= 'a' && chr <= 'z' {
            Ok((chr as u8) - ('a' as u8) + 26)
        } else if chr >= '0' && chr <= '9' {
            Ok((chr as u8) - ('0' as u8) + 52)
        } else if chr == '+' {
            Ok(62)
        } else if chr == '/' {
            Ok(63)
        } else {
            Err(DecoderError::BadNumber)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::encoding::*;

    ///
    /// Checks if a particular drawing operation can be both encoded and decoded
    ///
    fn check_can_round_trip(instruction: Draw) {
        // Encode the instruction
        let mut encoded = String::new();
        instruction.encode_canvas(&mut encoded);

        println!("{:?} {:?}", instruction, encoded);

        // Try decoding it
        let mut decoder = CanvasDecoder::new();
        let mut decoded = None;

        for c in encoded.chars() {
            // As we've encoded a single instruction we should never start with a valid value
            assert!(decoded.is_none());

            // Update with the next state
            decoded = decoder.decode(c).unwrap();
        }

        println!("  -> {:?}", decoded);

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
