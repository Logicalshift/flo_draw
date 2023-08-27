///
/// A 16 to 8 bit gamma correction look-up table
///
pub struct U8GammaLut {
    gamma:          f64,
    look_up_table:  [u8;65536],
}

impl U8GammaLut {
    ///
    /// Creates a gamma look-up table for a gamma correction value
    ///
    pub fn new(gamma: f64) -> Self {
        // Allocate the LUT (64k in size)
        let mut lut = [0u8; 65536];

        // Calculate the 65536 gamma-corrected values
        for idx in 0..65536 {
            let t = (idx as f64)/65535.0;
            let t = t.powf(gamma);
            let t = (t * 255.0) as u8;

            lut[idx] = t;
        }

        // Store the final look-up table
        U8GammaLut { 
            gamma:          gamma, 
            look_up_table:  lut 
        }
    }

    ///
    /// Returns the gamma correction value this table is using
    ///
    #[inline]
    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    ///
    /// Looks up a gamma corrected value. `val` can be from 0 to 65535 (where 65535 represents an intensity of 1.0)
    ///
    #[inline]
    pub fn look_up(&self, val: u16) -> u8 {
        self.look_up_table[val as usize]
    }
}