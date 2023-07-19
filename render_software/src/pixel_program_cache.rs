use crate::pixel_program::*;

use std::marker::{PhantomData};
use std::ops::{Range};

///
/// The pixel program cache provides a way to assign IDs to pixel programs and support initialising them
/// with a data cache.
///
pub struct PixelProgramCache {

}

///
/// The pixel program data cache stores the program data for the pixel programs
///
pub struct PixelProgramDataCache {

}

///
/// A data manager is used to store data associated with a program into a data cache
///
pub struct PixelProgramDataManager<TProgramData> {
    data: PhantomData<TProgramData>
}

///
/// Identifier for the program data for a pixel program
///
/// Every pixel program has a separate set of identifiers for their data
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelProgramDataId(usize);

///
/// Identifier for some scanline data
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelScanlineDataId(usize);

impl PixelProgramCache {
    ///
    /// Creates an empty pixel program cache
    ///
    pub fn empty() -> PixelProgramCache {
        PixelProgramCache {
        }
    }

    ///
    /// Caches a pixel program, returns its ID and a data manager to store data relating to the program
    ///
    pub fn add_program<TProgram>(&mut self, program: TProgram) -> (PixelProgramId, PixelProgramDataManager<TProgram::ProgramData>) 
    where
        TProgram: PixelProgram,
    {
        // Assign a data cache index for this program (or this program's data type? Might be easier to just make it per-program though)

        // Convert the program to read from the data cache

        // Store the program in the cache
        todo!()
    }

    ///
    /// Creates scanline data for a program 
    ///
    pub fn create_scanline_data(&self, data_cache: &mut PixelProgramDataCache, program_id: PixelProgramId, x_range: Range<f32>, ypos: i32, program_data: PixelProgramDataId) -> PixelScanlineDataId {
        todo!()        
    }

    ///
    /// Runs a program on a range of pixels
    ///
    pub fn run_program(&self, data_cache: &PixelProgramDataCache, program_id: PixelProgramId, target: &mut [[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: PixelProgramDataId, scanline_data: PixelScanlineDataId) {
        todo!()
    }
}
