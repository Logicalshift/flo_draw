///
/// An RGBA pixel as a set of u8 values
///
/// The alpha value is pre-multiplied into the RGB values, and the colour space is gamma-corrected
///
pub struct U8RgbaPremultipliedPixel([u8; 4]);
