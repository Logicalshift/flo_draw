///
/// A pixel filter implements a filter algorithm that can be applied to the result of 
///
pub trait PixelFilter {
    /// The type of the pixel that the filter accepts
    type Pixel : Send;

    ///
    /// Retrieves the number of extra lines that are required to produce a single output line
    ///
    /// The result here is the number of lines above the current line and the number of lines below the current line that are required.
    /// The 'current' line is always supplied to the filter.
    ///
    fn input_lines(&self) -> (usize, usize);

    ///
    /// Filters a single line of pixels from an input set of pixels. For lines outside of the input range, the pixels are always returned as
    /// the default '0' value.
    ///
    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]);
}
