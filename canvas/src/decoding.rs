use super::draw::*;
use super::color::*;
use super::transform2d::*;

use futures::*;
use futures::stream;
use futures::task::{Poll};

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
            // Whitespace ignored if we're not parsing a command
            '\n' | '\r' | ' ' => Ok((DecoderState::None, None)),

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

///
/// Decodes a canvas drawing represented as an iterator of characters. If there's an error in the stream, it will
/// be the last item decoded.
///
pub fn decode_drawing<In: IntoIterator<Item=char>>(source: In) -> impl Iterator<Item=Result<Draw, DecoderError>> {
    // The decoder represents the state machine used for decoding this item
    let mut decoder     = CanvasDecoder::new();
    let mut seen_error  = false;

    // Map the source characters into draw actions via the decoder
    source.into_iter()
        .filter_map(move |chr| {
            match decoder.decode(chr) {
                Ok(Some(draw))  => Some(Ok(draw)),
                Ok(None)        => None,
                Err(err)        => {
                    // The decoder will just return errors once it hits a failure: only return the initial error
                    if !seen_error {
                        seen_error = true;
                        Some(Err(err))
                    } else {
                        None
                    }
                }
            }
        })
}

///
/// Error from either a decoder or the stream that's feeding it
///
#[derive(Clone, Debug, PartialEq)]
pub enum StreamDecoderError<E> {
    /// Error from the decoder
    Decoder(DecoderError),

    /// Error from the stream
    Stream(E)
}

///
/// Decodes a canvas drawing represented as a stream of characters.
///
pub fn decode_drawing_stream<In: Unpin+Stream<Item=Result<char, E>>, E>(source: In) -> impl Unpin+Stream<Item=Result<Draw, StreamDecoderError<E>>> {
    let mut source      = source;
    let mut decoder     = CanvasDecoder::new();
    let mut seen_error  = false;

    stream::poll_fn(move |context| {
        if seen_error {
            // Only allow one error from the decoder (it remains in an error state after this)
            Poll::Ready(None)
        } else {
            loop {
                match source.poll_next_unpin(context) {
                    Poll::Ready(None)           => { return Poll::Ready(None); },
                    Poll::Pending               => { return Poll::Pending; },
                    Poll::Ready(Some(Ok(c)))    => {
                        match decoder.decode(c) {
                            Ok(None)            => { continue; },
                            Ok(Some(draw))      => { return Poll::Ready(Some(Ok(draw))); },
                            Err(err)            => { seen_error = true; return Poll::Ready(Some(Err(StreamDecoderError::Decoder(err)))); }
                        }
                    },

                    Poll::Ready(Some(Err(err))) => { return Poll::Ready(Some(Err(StreamDecoderError::Stream(err)))); }
                }
            }
        }
    })
}

#[cfg(test)]
mod test {
    use futures::prelude::*;
    use futures::executor;

    use super::*;
    use super::super::encoding::*;

    ///
    /// Checks if a particular drawing operation can be both encoded and decoded
    ///
    fn check_round_trip_single(instruction: Draw) {
        check_round_trip(vec![instruction])
    }

    ///
    /// Checks if a particular string of drawing operations can be both encoded and decoded
    ///
    fn check_round_trip(instructions: Vec<Draw>) {
        // Encode the instruction
        let mut encoded = String::new();
        for instruction in instructions.iter() {
            instruction.encode_canvas(&mut encoded);
        }

        println!("{:?} {:?}", instructions, encoded);

        // Try decoding it
        let decoded = decode_drawing(encoded.chars()).collect::<Vec<_>>();

        println!("  -> {:?}", decoded);

        // Should decode OK
        assert!(decoded.len() == instructions.len());

        // Should be the same as the original instruction
        assert!(decoded == instructions.into_iter().map(|draw| Ok(draw)).collect::<Vec<_>>());
    }

    #[test]
    fn decode_new_path() {
        check_round_trip_single(Draw::NewPath);
    }

    #[test]
    fn decode_move() {
        check_round_trip_single(Draw::Move(10.0, 15.0));
    }

    #[test]
    fn decode_line() {
        check_round_trip_single(Draw::Line(20.0, 42.0));
    }

    #[test]
    fn decode_bezier_curve() {
        check_round_trip_single(Draw::BezierCurve((1.0, 2.0), (3.0, 4.0), (5.0, 6.0)));
    }

    #[test]
    fn decode_close_path() {
        check_round_trip_single(Draw::ClosePath);
    }

    #[test]
    fn decode_fill() {
        check_round_trip_single(Draw::Fill);
    }

    #[test]
    fn decode_stroke() {
        check_round_trip_single(Draw::Stroke);
    }

    #[test]
    fn decode_line_width() {
        check_round_trip_single(Draw::LineWidth(23.0));
    }

    #[test]
    fn decode_line_width_pixels() {
        check_round_trip_single(Draw::LineWidthPixels(43.0));
    }

    #[test]
    fn decode_line_join() {
        check_round_trip_single(Draw::LineJoin(LineJoin::Bevel));
    }

    #[test]
    fn decode_line_cap() {
        check_round_trip_single(Draw::LineCap(LineCap::Round));
    }

    #[test]
    fn decode_new_dash_pattern() {
        check_round_trip_single(Draw::NewDashPattern);
    }

    #[test]
    fn decode_dash_length() {
        check_round_trip_single(Draw::DashLength(56.0));
    }

    #[test]
    fn decode_dash_offset() {
        check_round_trip_single(Draw::DashOffset(13.0));
    }

    #[test]
    fn decode_stroke_color() {
        check_round_trip_single(Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)));
    }

    #[test]
    fn decode_fill_color() {
        check_round_trip_single(Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)));
    }

    #[test]
    fn decode_blend_mode() {
        check_round_trip_single(Draw::BlendMode(BlendMode::Lighten));
    }

    #[test]
    fn decode_identity_transform() {
        check_round_trip_single(Draw::IdentityTransform);
    }

    #[test]
    fn decode_canvas_height() {
        check_round_trip_single(Draw::CanvasHeight(81.0));
    }

    #[test]
    fn decode_center_region() {
        check_round_trip_single(Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)));
    }

    #[test]
    fn decode_multiply_transform() {
        check_round_trip_single(Draw::MultiplyTransform(Transform2D((1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0))));
    }

    #[test]
    fn decode_unclip() {
        check_round_trip_single(Draw::Unclip);
    }

    #[test]
    fn decode_clip() {
        check_round_trip_single(Draw::Clip)
    }

    #[test]
    fn decode_store() {
        check_round_trip_single(Draw::Store);
    }

    #[test]
    fn decode_restore() {
        check_round_trip_single(Draw::Restore);
    }

    #[test]
    fn decode_free_stored_buffer() {
        check_round_trip_single(Draw::FreeStoredBuffer);
    }

    #[test]
    fn decode_push_state() {
        check_round_trip_single(Draw::PushState);
    }

    #[test]
    fn decode_pop_state() {
        check_round_trip_single(Draw::PopState);
    }

    #[test]
    fn decode_clear_canvas() {
        check_round_trip_single(Draw::ClearCanvas);
    }

    #[test]
    fn decode_layer() {
        check_round_trip_single(Draw::Layer(21));
    }

    #[test]
    fn decode_layer_blend() {
        check_round_trip_single(Draw::LayerBlend(76, BlendMode::Lighten))
    }

    #[test]
    fn decode_clear_layer() {
        check_round_trip_single(Draw::ClearLayer);
    }

    #[test]
    fn will_accept_newlines() {
        let mut decoder = CanvasDecoder::new();
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('\n') == Ok(None));
        assert!(decoder.decode('N') == Ok(None));
        assert!(decoder.decode('p') == Ok(Some(Draw::NewPath)));
    }

    #[test]
    fn error_on_bad_char() {
        let mut decoder = CanvasDecoder::new();
        assert!(decoder.decode('N') == Ok(None));
        assert!(decoder.decode('X') == Err(DecoderError::InvalidCharacter('X')));
    }

    #[test]
    fn decode_all_iter() {
        check_round_trip(vec![
            Draw::NewPath,
            Draw::Move(10.0, 15.0),
            Draw::Line(20.0, 42.0),
            Draw::BezierCurve((1.0, 2.0), (3.0, 4.0), (5.0, 6.0)),
            Draw::ClosePath,
            Draw::Fill,
            Draw::Stroke,
            Draw::LineWidth(23.0),
            Draw::LineWidthPixels(43.0),
            Draw::LineJoin(LineJoin::Bevel),
            Draw::LineCap(LineCap::Round),
            Draw::NewDashPattern,
            Draw::DashLength(56.0),
            Draw::DashOffset(13.0),
            Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)),
            Draw::BlendMode(BlendMode::Lighten),
            Draw::IdentityTransform,
            Draw::CanvasHeight(81.0),
            Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)),
            Draw::MultiplyTransform(Transform2D((1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0))),
            Draw::Unclip,
            Draw::Store,
            Draw::Restore,
            Draw::FreeStoredBuffer,
            Draw::PushState,
            Draw::PopState,
            Draw::ClearCanvas,
            Draw::Layer(21),
            Draw::ClearLayer,
            Draw::NewPath
        ]);
    }

    #[test]
    fn decode_all_stream() {
        let all = vec![
            Draw::NewPath,
            Draw::Move(10.0, 15.0),
            Draw::Line(20.0, 42.0),
            Draw::BezierCurve((1.0, 2.0), (3.0, 4.0), (5.0, 6.0)),
            Draw::ClosePath,
            Draw::Fill,
            Draw::Stroke,
            Draw::LineWidth(23.0),
            Draw::LineWidthPixels(43.0),
            Draw::LineJoin(LineJoin::Bevel),
            Draw::LineCap(LineCap::Round),
            Draw::NewDashPattern,
            Draw::DashLength(56.0),
            Draw::DashOffset(13.0),
            Draw::StrokeColor(Color::Rgba(0.1, 0.2, 0.3, 0.4)),
            Draw::FillColor(Color::Rgba(0.2, 0.3, 0.4, 0.5)),
            Draw::BlendMode(BlendMode::Lighten),
            Draw::IdentityTransform,
            Draw::CanvasHeight(81.0),
            Draw::CenterRegion((6.0, 7.0), (8.0, 9.0)),
            Draw::MultiplyTransform(Transform2D((1.0, 2.0, 3.0), (4.0, 5.0, 6.0), (7.0, 8.0, 9.0))),
            Draw::Unclip,
            Draw::Store,
            Draw::Restore,
            Draw::FreeStoredBuffer,
            Draw::PushState,
            Draw::PopState,
            Draw::ClearCanvas,
            Draw::Layer(21),
            Draw::ClearLayer,
            Draw::NewPath
        ];
        let mut encoded = String::new();
        all.encode_canvas(&mut encoded);

        println!("{:?}", encoded);

        let all_stream  = stream::iter(encoded.chars().into_iter().map(|c| -> Result<_, ()> { Ok(c) }));
        let decoder     = decode_drawing_stream(all_stream);
        let mut decoder = decoder;

        executor::block_on(async {
            let mut decoded = vec![];
            while let Some(next) = decoder.next().await {
                decoded.push(next);
            }

            println!(" -> {:?}", decoded);

            let all = all.into_iter().map(|item| Ok(item)).collect::<Vec<_>>();
            assert!(all == decoded);
        });
    }
}
