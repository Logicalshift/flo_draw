///
/// A pixel filter implements a filter algorithm that can be applied to pixels one line at a time
///
/// The design is such that it's not necessary to have the entire set of input pixels in one place for most filter types.
///
pub trait PixelFilter {
    /// The type of the pixel that the filter accepts
    type Pixel : Send;

    ///
    /// Retrieves the number of extra lines that are required to produce a single output line (above and below)
    ///
    /// The result here is the number of lines above the current line and the number of lines below the current line that are required.
    /// The 'current' line is always supplied to the filter.
    ///
    fn input_lines(&self) -> (usize, usize);

    ///
    /// Retrieves the number of extra columns that are needed as input (left and right)
    ///
    /// The sum of these values will be added to the length of each input line. These extra pixels are useful when partially applying
    /// a filter (eg, when rendering via a pixel program), as it provides a way to generate the extra pixels needed to fully apply
    /// something like a gaussian filter.
    ///
    fn extra_columns(&self) -> (usize, usize);

    ///
    /// Filters a single line of pixels from an input set of pixels. For lines outside of the input range, the pixels are always returned as
    /// the default '0' value.
    ///
    fn filter_line(&self, input_lines: &[&[Self::Pixel]], output_line: &mut [Self::Pixel]);
}
